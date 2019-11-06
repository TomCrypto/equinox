use js_sys::Error;
use maplit::hashmap;
use quasirandom::Qrng;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use web_sys::WebGl2RenderingContext as Context;
use zerocopy::{AsBytes, FromBytes};

use crate::*;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, AsBytes, FromBytes)]
pub(crate) struct PhotonData {
    hit_position: [f32; 3],
    padding1: f32,
    incident_direction: [f32; 3],
    padding2: f32,
    outgoing_direction: [f32; 3],
    padding3: f32,
    incident_throughput: [f32; 3],
    padding4: f32,
    outgoing_throughput: [f32; 3],
    padding5: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, AsBytes, FromBytes)]
pub(crate) struct PhotonTreeNode {
    hit_position: [f32; 3],
    left_index: u32,
    incident_direction: [f32; 3],
    right_index: u32,
    incident_throughput: [f32; 3],
    axis: u32, // TODO: store material info here? if needed?
    more_padding: [u32; 4],
}

impl VertexLayout for PhotonData {
    const VERTEX_LAYOUT: &'static [VertexAttribute] = &[
        VertexAttribute::new(0, 0, VertexAttributeKind::Float4), // position
        VertexAttribute::new(1, 16, VertexAttributeKind::Float4), // incident direction
        VertexAttribute::new(2, 32, VertexAttributeKind::Float4), // outgoing direction
        VertexAttribute::new(3, 48, VertexAttributeKind::Float4), // incident throughput
        VertexAttribute::new(4, 64, VertexAttributeKind::Float4), // outgoing throughput
    ];
}

/*

Buf1 (RGBA32F):

    RGB => visible point position
    A => material index + instance

Buf2 (RGBA16F):

    RGB => visible point throughput
    A => {0, 1, 2, 3, 4}

        0 => not a valid visible point, treat throughput as radiance
        1 => direction.z is positive, normal.z is positive
        2 => direction.z is positive, normal.z is negative
        3 => direction.z is negative, normal.z is positive
        4 => direction.z is negative, normal.z is negative

Buf3 (RGBA16F):

    RG => visible point path direction, packed
    BA => visible point path surface normal, packed

*/

#[derive(Debug)]
pub struct Device {
    pub(crate) gl: Context,

    pub(crate) program: Shader,
    pub(crate) present_program: Shader,

    pub(crate) read_convolution_buffers_shader: Shader,
    pub(crate) fft_shader: Shader,

    pub(crate) camera_buffer: UniformBuffer<CameraData>,

    pub(crate) geometry_buffer: UniformBuffer<[GeometryParameter]>,
    pub(crate) material_buffer: UniformBuffer<[MaterialParameter]>,
    pub(crate) instance_buffer: UniformBuffer<[SceneInstanceNode]>,

    pub(crate) display_buffer: UniformBuffer<DisplayData>,

    pub(crate) envmap_marg_cdf: Texture<R16F>,
    pub(crate) envmap_cond_cdf: Texture<R16F>,

    pub(crate) envmap_texture: Texture<RGBA16F>,

    pub(crate) globals_buffer: UniformBuffer<GlobalData>,
    pub(crate) raster_buffer: UniformBuffer<RasterData>,

    pub(crate) samples: Texture<RGBA32F>,
    pub(crate) samples_fbo: Framebuffer,

    // Complex-valued spectrums for each render channel
    pub(crate) rspectrum_temp1: Texture<RG32F>,
    pub(crate) gspectrum_temp1: Texture<RG32F>,
    pub(crate) bspectrum_temp1: Texture<RG32F>,
    pub(crate) rspectrum_temp2: Texture<RG32F>,
    pub(crate) gspectrum_temp2: Texture<RG32F>,
    pub(crate) bspectrum_temp2: Texture<RG32F>,

    pub(crate) r_aperture_spectrum: Texture<RG32F>,
    pub(crate) g_aperture_spectrum: Texture<RG32F>,
    pub(crate) b_aperture_spectrum: Texture<RG32F>,

    // Final convolved render output (real-valued)
    pub(crate) render: Texture<RGBA32F>,

    pub(crate) fft_pass_data: VertexArray<[FFTPassData]>,

    pub(crate) spectrum_temp1_fbo: Framebuffer,
    pub(crate) spectrum_temp2_fbo: Framebuffer,
    pub(crate) render_fbo: Framebuffer,
    pub(crate) aperture_fbo: Framebuffer,

    pub(crate) load_convolution_buffers_shader: Shader,

    // transform buffer for the photon data
    pub(crate) photon_hits: VertexArray<[PhotonData]>,

    // textures for the photon data and kd-tree
    pub(crate) photon_table_tex: Texture<RGBA32UI>,
    pub(crate) photon_fbo: Framebuffer,

    pub(crate) test_shader: Shader,

    // ping-pong buffers for the visible point data
    pub(crate) visible_point_count_a: Texture<R32F>,
    pub(crate) visible_point_count_b: Texture<R32F>,
    pub(crate) visible_point_data_a: Texture<RGBA32F>,
    pub(crate) visible_point_data_b: Texture<RGBA32F>,

    // buffer to store the visible point path information
    pub(crate) visible_point_path1: Texture<RGBA32F>,
    pub(crate) visible_point_path2: Texture<RGBA32F>,
    pub(crate) visible_point_path3: Texture<RGBA32F>,

    // buffer to store the visible point properties for this pass (photon contributions + photon
    // count)
    pub(crate) visible_point_pass_data: Texture<RGBA32F>,

    // FBO to write into each set of visible point data
    pub(crate) visible_point_a_fbo: Framebuffer,
    pub(crate) visible_point_b_fbo: Framebuffer,

    // FBO to write into the path information texture
    pub(crate) visible_point_path_fbo: Framebuffer,

    // FBO to write into the pass data texture
    pub(crate) visible_point_pass_data_fbo: Framebuffer,

    pub(crate) visible_point_update_pixels_shader: Shader,
    pub(crate) visible_point_gen_shader: Shader,

    pub(crate) allocator: Allocator,

    device_lost: bool,

    pub(crate) state: DeviceState,
}

impl Device {
    /// Creates a new device using a WebGL2 context.
    pub fn new(gl: &Context) -> Result<Self, Error> {
        Ok(Self {
            allocator: Allocator::new(),
            gl: gl.clone(),
            visible_point_gen_shader: Shader::new(
                gl.clone(),
                shaders::VS_FULLSCREEN,
                shaders::FS_GEN_VISIBLE_POINTS,
                hashmap! {
                    "Camera" => BindingPoint::UniformBlock(0),
                    "Instance" => BindingPoint::UniformBlock(4),
                    "Geometry" => BindingPoint::UniformBlock(7),
                    "Material" => BindingPoint::UniformBlock(8),
                    "Globals" => BindingPoint::UniformBlock(2),
                    "Raster" => BindingPoint::UniformBlock(3),
                    "envmap_texture" => BindingPoint::Texture(1),
                    "envmap_marg_cdf" => BindingPoint::Texture(2),
                    "envmap_cond_cdf" => BindingPoint::Texture(3),
                },
                &[],
                hashmap! {
                    "geometry-user.glsl" => "",
                },
                hashmap! {
                    "HAS_ENVMAP" => "0",
                    "ENVMAP_COLS" => "0",
                    "ENVMAP_ROWS" => "0",
                    "ENVMAP_ROTATION" => "0.0",
                    "INSTANCE_DATA_COUNT" => "0",
                    "GEOMETRY_DATA_COUNT" => "0",
                    "MATERIAL_DATA_COUNT" => "0",
                    "INSTANCE_DATA_PRESENT" => "0"
                },
            ),
            visible_point_update_pixels_shader: Shader::new(
                gl.clone(),
                shaders::VS_FULLSCREEN,
                shaders::FS_UPDATE_PIXELS,
                hashmap! {
                    "old_photon_count_tex" => BindingPoint::Texture(0),
                    "old_photon_data_tex" => BindingPoint::Texture(1),
                    "new_photon_data_tex" => BindingPoint::Texture(2),
                    "Globals" => BindingPoint::UniformBlock(2),
                },
                &[],
                hashmap! {},
                hashmap! {},
            ),
            photon_table_tex: Texture::new(gl.clone()),
            photon_hits: VertexArray::new(gl.clone()),
            fft_pass_data: VertexArray::new(gl.clone()),
            test_shader: Shader::new(
                gl.clone(),
                shaders::VS_TRANSFORM_FEEDBACK_TEST,
                shaders::FS_DUMMY,
                hashmap! {
                    "Instance" => BindingPoint::UniformBlock(4),
                    "Geometry" => BindingPoint::UniformBlock(7),
                    "Material" => BindingPoint::UniformBlock(8),
                    "Globals" => BindingPoint::UniformBlock(2),
                    "Raster" => BindingPoint::UniformBlock(3),
                    "envmap_texture" => BindingPoint::Texture(1),
                    "envmap_marg_cdf" => BindingPoint::Texture(2),
                    "envmap_cond_cdf" => BindingPoint::Texture(3),
                },
                &[
                    "out_position",
                    "out_outgoing_direction",
                    "out_outgoing_throughput",
                ],
                hashmap! {
                    "geometry-user.glsl" => "",
                },
                hashmap! {
                    "HAS_ENVMAP" => "0",
                    "ENVMAP_COLS" => "0",
                    "ENVMAP_ROWS" => "0",
                    "ENVMAP_ROTATION" => "0.0",
                    "INSTANCE_DATA_COUNT" => "0",
                    "GEOMETRY_DATA_COUNT" => "0",
                    "MATERIAL_DATA_COUNT" => "0",
                    "INSTANCE_DATA_PRESENT" => "0",
                    "HASH_TABLE_COLS" => "0",
                    "HASH_TABLE_ROWS" => "0",
                    "HASH_CELL_COLS" => "0",
                    "HASH_CELL_ROWS" => "0",
                },
            ),
            load_convolution_buffers_shader: Shader::new(
                gl.clone(),
                shaders::VS_FULLSCREEN,
                shaders::FS_LOAD_CONVOLUTION_BUFFERS,
                hashmap! {
                    "image" => BindingPoint::Texture(0),
                },
                &[],
                hashmap! {},
                hashmap! {
                    "CONV_DIMS" => "vec2(0.0, 0.0)",
                    "IMAGE_DIMS" => "vec2(0.0, 0.0)",
                },
            ),
            fft_shader: Shader::new(
                gl.clone(),
                shaders::VS_FFT_PASS,
                shaders::FS_FFT_PASS,
                hashmap! {
                    "r_conv_buffer" => BindingPoint::Texture(0),
                    "g_conv_buffer" => BindingPoint::Texture(1),
                    "b_conv_buffer" => BindingPoint::Texture(2),
                    "r_conv_filter" => BindingPoint::Texture(3),
                    "g_conv_filter" => BindingPoint::Texture(4),
                    "b_conv_filter" => BindingPoint::Texture(5),
                },
                &[],
                hashmap! {},
                hashmap! {},
            ),
            read_convolution_buffers_shader: Shader::new(
                gl.clone(),
                shaders::VS_FULLSCREEN,
                shaders::FS_READ_CONVOLUTION_BUFFERS,
                hashmap! {
                    "r_conv_buffer" => BindingPoint::Texture(0),
                    "g_conv_buffer" => BindingPoint::Texture(1),
                    "b_conv_buffer" => BindingPoint::Texture(2),
                    "source" => BindingPoint::Texture(3),
                },
                &[],
                hashmap! {},
                hashmap! {
                    "CONV_DIMS" => "vec2(0.0, 0.0)",
                    "IMAGE_DIMS" => "vec2(0.0, 0.0)",
                },
            ),
            program: Shader::new(
                gl.clone(),
                shaders::VS_FULLSCREEN,
                shaders::FRAG,
                hashmap! {
                    "Material" => BindingPoint::UniformBlock(8),
                    "Globals" => BindingPoint::UniformBlock(7),
                    "photon_table" => BindingPoint::Texture(4),
                    "photon_radius_tex" => BindingPoint::Texture(5),
                    "visible_point_path_buf1" => BindingPoint::Texture(0),
                    "visible_point_path_buf2" => BindingPoint::Texture(1),
                    "visible_point_path_buf3" => BindingPoint::Texture(2),
                },
                &[],
                hashmap! {},
                hashmap! {
                    "MATERIAL_DATA_COUNT" => "0",
                    "HASH_TABLE_COLS" => "0",
                    "HASH_TABLE_ROWS" => "0",
                    "HASH_CELL_COLS" => "0",
                    "HASH_CELL_ROWS" => "0",
                },
            ),
            present_program: Shader::new(
                gl.clone(),
                shaders::VS_FULLSCREEN,
                shaders::PRESENT,
                hashmap! {
                    "samples" => BindingPoint::Texture(0),
                    "Display" => BindingPoint::UniformBlock(0),
                },
                &[],
                hashmap! {},
                hashmap! {},
            ),
            camera_buffer: UniformBuffer::new(gl.clone()),
            geometry_buffer: UniformBuffer::new(gl.clone()),
            material_buffer: UniformBuffer::new(gl.clone()),
            instance_buffer: UniformBuffer::new(gl.clone()),
            raster_buffer: UniformBuffer::new(gl.clone()),
            display_buffer: UniformBuffer::new(gl.clone()),
            globals_buffer: UniformBuffer::new(gl.clone()),
            envmap_texture: Texture::new(gl.clone()),
            envmap_marg_cdf: Texture::new(gl.clone()),
            envmap_cond_cdf: Texture::new(gl.clone()),
            samples_fbo: Framebuffer::new(gl.clone()),
            rspectrum_temp1: Texture::new(gl.clone()),
            gspectrum_temp1: Texture::new(gl.clone()),
            bspectrum_temp1: Texture::new(gl.clone()),
            rspectrum_temp2: Texture::new(gl.clone()),
            gspectrum_temp2: Texture::new(gl.clone()),
            bspectrum_temp2: Texture::new(gl.clone()),
            r_aperture_spectrum: Texture::new(gl.clone()),
            g_aperture_spectrum: Texture::new(gl.clone()),
            b_aperture_spectrum: Texture::new(gl.clone()),
            render: Texture::new(gl.clone()),
            render_fbo: Framebuffer::new(gl.clone()),
            aperture_fbo: Framebuffer::new(gl.clone()),
            spectrum_temp1_fbo: Framebuffer::new(gl.clone()),
            spectrum_temp2_fbo: Framebuffer::new(gl.clone()),
            photon_fbo: Framebuffer::new(gl.clone()),
            samples: Texture::new(gl.clone()),
            visible_point_count_a: Texture::new(gl.clone()),
            visible_point_count_b: Texture::new(gl.clone()),
            visible_point_data_a: Texture::new(gl.clone()),
            visible_point_data_b: Texture::new(gl.clone()),
            visible_point_path1: Texture::new(gl.clone()),
            visible_point_path2: Texture::new(gl.clone()),
            visible_point_path3: Texture::new(gl.clone()),
            visible_point_pass_data: Texture::new(gl.clone()),
            visible_point_a_fbo: Framebuffer::new(gl.clone()),
            visible_point_b_fbo: Framebuffer::new(gl.clone()),
            visible_point_path_fbo: Framebuffer::new(gl.clone()),
            visible_point_pass_data_fbo: Framebuffer::new(gl.clone()),
            device_lost: true,
            state: DeviceState::new(),
        })
    }

    /// Signals the context was lost.
    pub fn context_lost(&mut self) {
        self.device_lost = true;
    }

    /// Updates this device to render a given scene or returns an error.
    pub fn update(&mut self, scene: &mut Scene) -> Result<bool, Error> {
        if self.device_lost && !self.try_restore(scene)? {
            return Ok(false); // context currently lost
        }

        let mut invalidated = false;

        invalidated |= Dirty::clean(&mut scene.camera, |camera| {
            self.update_camera(camera)?;

            Ok(())
        })?;

        let instances = &mut scene.instance_list;

        invalidated |= Dirty::clean(&mut scene.geometry_list, |geometries| {
            let mut generator = GeometryGlslGenerator::new();

            let mut geometry_functions = vec![];

            for geometry in geometries {
                geometry_functions.push((
                    generator.add_distance_function(geometry),
                    generator.add_normal_function(geometry),
                ));
            }

            let code = generator.generate(&geometry_functions);

            self.visible_point_gen_shader
                .set_header("geometry-user.glsl", &code);
            self.test_shader.set_header("geometry-user.glsl", &code);

            Dirty::dirty(instances);

            Ok(())
        })?;

        invalidated |= Dirty::clean(&mut scene.material_list, |materials| {
            self.update_materials(materials)?;

            Dirty::dirty(instances);

            Ok(())
        })?;

        let geometry_list = &scene.geometry_list;
        let material_list = &scene.material_list;

        invalidated |= Dirty::clean(&mut scene.instance_list, |instances| {
            self.update_instances(geometry_list, material_list, instances)?;

            Ok(())
        })?;

        let assets = &scene.assets;

        invalidated |= Dirty::clean(&mut scene.environment, |environment| {
            self.update_environment(assets, environment)?;

            Ok(())
        })?;

        invalidated |= Dirty::clean(&mut scene.raster, |raster| {
            self.update_raster(raster)?;

            self.samples
                .create(raster.width.get() as usize, raster.height.get() as usize);

            self.render
                .create(raster.width.get() as usize, raster.height.get() as usize);

            self.samples_fbo.rebuild(&[&self.samples]);

            // Configure the shaders with the desired resolutions...

            self.load_convolution_buffers_shader
                .set_define("CONV_DIMS", format!("vec2({:+e}, {:+e})", 2048.0, 1024.0));

            self.load_convolution_buffers_shader.set_define(
                "IMAGE_DIMS",
                format!(
                    "vec2({:+e}, {:+e})",
                    raster.width.get() as f32,
                    raster.height.get() as f32
                ),
            );

            self.read_convolution_buffers_shader
                .set_define("CONV_DIMS", format!("vec2({:+e}, {:+e})", 2048.0, 1024.0));

            self.read_convolution_buffers_shader.set_define(
                "IMAGE_DIMS",
                format!(
                    "vec2({:+e}, {:+e})",
                    raster.width.get() as f32,
                    raster.height.get() as f32
                ),
            );

            self.rspectrum_temp1.create(2048, 1024);
            self.gspectrum_temp1.create(2048, 1024);
            self.bspectrum_temp1.create(2048, 1024);

            self.rspectrum_temp2.create(2048, 1024);
            self.gspectrum_temp2.create(2048, 1024);
            self.bspectrum_temp2.create(2048, 1024);

            self.r_aperture_spectrum.create(2048, 1024);
            self.g_aperture_spectrum.create(2048, 1024);
            self.b_aperture_spectrum.create(2048, 1024);

            self.visible_point_count_a
                .create(raster.width.get() as usize, raster.height.get() as usize);
            self.visible_point_count_b
                .create(raster.width.get() as usize, raster.height.get() as usize);
            self.visible_point_data_a
                .create(raster.width.get() as usize, raster.height.get() as usize);
            self.visible_point_data_b
                .create(raster.width.get() as usize, raster.height.get() as usize);
            self.visible_point_path1
                .create(raster.width.get() as usize, raster.height.get() as usize);
            self.visible_point_path2
                .create(raster.width.get() as usize, raster.height.get() as usize);
            self.visible_point_path3
                .create(raster.width.get() as usize, raster.height.get() as usize);
            self.visible_point_pass_data
                .create(raster.width.get() as usize, raster.height.get() as usize);

            self.visible_point_a_fbo.rebuild(&[
                &self.visible_point_count_a,
                &self.visible_point_data_a,
                &self.samples,
            ]);
            self.visible_point_b_fbo.rebuild(&[
                &self.visible_point_count_b,
                &self.visible_point_data_b,
                &self.samples,
            ]);
            self.visible_point_path_fbo.rebuild(&[
                &self.visible_point_path1,
                &self.visible_point_path2,
                &self.visible_point_path3,
            ]);
            self.visible_point_pass_data_fbo
                .rebuild(&[&self.visible_point_pass_data]);

            self.render_fbo.rebuild(&[&self.render]);
            self.aperture_fbo.rebuild(&[
                &self.r_aperture_spectrum,
                &self.g_aperture_spectrum,
                &self.b_aperture_spectrum,
            ]);

            self.spectrum_temp1_fbo.rebuild(&[
                &self.rspectrum_temp1,
                &self.gspectrum_temp1,
                &self.bspectrum_temp1,
            ]);

            self.spectrum_temp2_fbo.rebuild(&[
                &self.rspectrum_temp2,
                &self.gspectrum_temp2,
                &self.bspectrum_temp2,
            ]);

            self.prepare_fft_pass_data();

            Ok(())
        })?;

        let assets = &scene.assets;

        invalidated |= Dirty::clean(&mut scene.aperture, |aperture| {
            if let Some(aperture) = aperture {
                self.preprocess_filter(
                    &assets[&aperture.aperture_texels],
                    aperture.aperture_width as usize,
                    aperture.aperture_height as usize,
                );
            }

            Ok(())
        })?;

        invalidated |= Dirty::clean(&mut scene.integrator, |integrator| {
            if integrator.hash_table_bits < 20 {
                return Err(Error::new("hash_table_bits needs to be at least 20"));
            }

            // TODO: return an error if the texture is too large to create here...
            // (check against the size limits or something)

            let col_bits = integrator.hash_table_bits / 2;
            let row_bits = integrator.hash_table_bits - col_bits;

            let cols = 2usize.pow(col_bits);
            let rows = 2usize.pow(row_bits);

            self.photon_table_tex.create(cols, rows);
            self.photon_fbo.rebuild(&[&self.photon_table_tex]);

            self.program
                .set_define("HASH_TABLE_COLS", format!("{}U", cols));
            self.program
                .set_define("HASH_TABLE_ROWS", format!("{}U", rows));

            self.test_shader
                .set_define("HASH_TABLE_COLS", format!("{}U", cols));
            self.test_shader
                .set_define("HASH_TABLE_ROWS", format!("{}U", rows));

            Ok(())
        })?;

        // These are post-processing settings that don't apply to the path-traced light
        // transport simulation, so we don't need to invalidate the render buffer here.

        Dirty::clean(&mut scene.display, |display| {
            self.update_display(display)?;

            Ok(())
        })?;

        self.program.rebuild()?;
        self.present_program.rebuild()?;

        self.read_convolution_buffers_shader.rebuild()?;
        self.fft_shader.rebuild()?;
        self.load_convolution_buffers_shader.rebuild()?;
        self.test_shader.rebuild()?;
        self.visible_point_update_pixels_shader.rebuild()?;
        self.visible_point_gen_shader.rebuild()?;

        if invalidated {
            self.state.reset(scene);
            self.reset_refinement();
        }

        self.allocator.shrink_to_watermark();

        Ok(invalidated)
    }

    /// Further refines the path-traced render buffer.
    pub fn refine(&mut self) {
        if self.device_lost {
            return;
        }

        // select the grid cell size
        let grid_cell_size = 2.0 * self.state.search_radius;

        // select N and M; M is a power of two, and we compute the hash cell cols/rows
        // based on that

        // TODO: take into account max-m

        let mut n = (self.state.integrator.photon_density / grid_cell_size.powi(2)) as usize;
        n = n.min(self.state.integrator.photons_per_pass).max(1);
        let mut m = self.state.integrator.photons_per_pass / n;
        m = m.next_power_of_two();
        // TODO: for now
        m = 1;
        // n = self.state.integrator.photons_per_pass / m;
        n /= m;

        let mut hash_cell_cols = 1;
        let mut hash_cell_rows = 1;

        while m > 1 {
            if m > 4 {
                hash_cell_cols *= 2;
                hash_cell_rows *= 2;
                m /= 4;
            } else {
                hash_cell_cols *= 2;
                m /= 2;
            }
        }

        assert_eq!(hash_cell_cols * hash_cell_rows, m);

        log::info!(
            "frame = {}, n = {}, m = {}, hash_cell_cols = {}, hash_cell_rows = {}",
            self.state.frame,
            n,
            m,
            hash_cell_cols,
            hash_cell_rows
        );

        // TODO: not happy with this, can we improve it
        self.state.update(
            &mut self.allocator,
            &mut self.globals_buffer,
            (n * m) as u32,
            grid_cell_size,
            hash_cell_cols as u32,
            hash_cell_rows as u32,
        );

        let iteration = self.state.frame - 1;

        // this is a new pass; reset all the per-pass data
        // GENERATE THE VISIBLE POINT INFORMATION

        let command = self.visible_point_gen_shader.begin_draw();

        command.bind(&self.camera_buffer, "Camera");
        command.bind(&self.geometry_buffer, "Geometry");
        command.bind(&self.material_buffer, "Material");
        command.bind(&self.instance_buffer, "Instance");
        command.bind(&self.globals_buffer, "Globals");
        command.bind(&self.raster_buffer, "Raster");
        command.bind(&self.envmap_texture, "envmap_texture");
        command.bind(&self.envmap_marg_cdf, "envmap_marg_cdf");
        command.bind(&self.envmap_cond_cdf, "envmap_cond_cdf");

        command.set_viewport(0, 0, self.samples.cols() as i32, self.samples.rows() as i32);
        command.set_framebuffer(&self.visible_point_path_fbo);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);

        // UPDATE PER-PIXEL STATE

        if iteration == 0 {
            // we are starting a new render, clear all pass data and set the initial search
            // radius

            self.visible_point_a_fbo.clear(0, [0.0, 0.0, 0.0, 0.0]);
            self.visible_point_a_fbo
                .clear(1, [0.0, 0.0, 0.0, self.state.search_radius]);
        }

        // GENERATE PHOTONS

        self.visible_point_pass_data_fbo
            .clear(0, [0.0, 0.0, 0.0, 0.0]);

        let command = self.test_shader.begin_draw();

        command.bind(&self.geometry_buffer, "Geometry");
        command.bind(&self.material_buffer, "Material");
        command.bind(&self.instance_buffer, "Instance");
        command.bind(&self.globals_buffer, "Globals");
        command.bind(&self.raster_buffer, "Raster");
        command.bind(&self.envmap_texture, "envmap_texture");
        command.bind(&self.envmap_marg_cdf, "envmap_marg_cdf");
        command.bind(&self.envmap_cond_cdf, "envmap_cond_cdf");

        command.set_viewport(
            0,
            0,
            self.photon_table_tex.cols() as i32,
            self.photon_table_tex.rows() as i32,
        );
        command.set_framebuffer(&self.photon_fbo);
        self.photon_fbo.clear_ui(0, [0, 0, 0, 0]);

        command.unset_vertex_array();
        command.draw_points_instanced(n, m);

        // GATHER PHOTONS BACK INTO VISIBLE POINTS

        let command = self.program.begin_draw();

        command.bind(&self.material_buffer, "Material");
        command.bind(&self.globals_buffer, "Globals");
        command.bind(&self.photon_table_tex, "photon_table");
        command.bind(&self.visible_point_path1, "visible_point_path_buf1");
        command.bind(&self.visible_point_path2, "visible_point_path_buf2");
        command.bind(&self.visible_point_path3, "visible_point_path_buf3");

        if iteration % 2 == 0 {
            log::info!("accumulating photons using radius in A");
            command.bind(&self.visible_point_data_a, "photon_radius_tex");
        } else {
            log::info!("accumulating photons using radius in B");
            command.bind(&self.visible_point_data_b, "photon_radius_tex");
        }

        command.set_blend_mode(BlendMode::Add);

        command.set_viewport(0, 0, self.samples.cols() as i32, self.samples.rows() as i32);
        command.set_framebuffer(&self.visible_point_pass_data_fbo);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);

        // there is a previous pass; update the per-pixel state

        let command = self.visible_point_update_pixels_shader.begin_draw();
        command.bind(&self.visible_point_pass_data, "new_photon_data_tex");
        command.set_viewport(0, 0, self.samples.cols() as i32, self.samples.rows() as i32);

        if iteration % 2 == 0 {
            // old data is in "a", we want to write in "b"
            log::info!("pass complete, reading from A and writing to B");

            command.bind(&self.visible_point_count_a, "old_photon_count_tex");
            command.bind(&self.visible_point_data_a, "old_photon_data_tex");

            command.set_framebuffer(&self.visible_point_b_fbo);
        } else {
            // old data is in "b", we want to write in "a"
            log::info!("pass complete, reading from B and writing to A");

            command.bind(&self.visible_point_count_b, "old_photon_count_tex");
            command.bind(&self.visible_point_data_b, "old_photon_data_tex");

            command.set_framebuffer(&self.visible_point_a_fbo);
        }

        command.unset_vertex_array();
        command.draw_triangles(0, 1);

        // AVERAGE RADIUS REDUCTION

        // given that we fired N * M photons, with the given grid cell size we would
        // expect the following number of photons to have fallen into each grid
        // on average: N * M * D / C^2 = Np
        // given this information, we can expect the ratio to be:
        // (a + alpha Np) / (a + Np) =
        // self.state.search_radius *= 0.9995;

        log::info!("search radius = {}", self.state.search_radius);
    }

    pub fn render(&mut self) {
        if self.device_lost {
            return;
        }

        if self.state.enable_lens_flare {
            self.render_lens_flare();
        }

        let command = self.present_program.begin_draw();

        if self.state.enable_lens_flare {
            command.bind(&self.render, "samples");
        } else {
            command.bind(&self.samples, "samples");
        }

        command.bind(&self.display_buffer, "Display");

        command.set_viewport(0, 0, self.samples.cols() as i32, self.samples.rows() as i32);

        command.set_canvas_framebuffer();

        command.unset_vertex_array();
        command.draw_triangles(0, 1);
    }

    fn try_restore(&mut self, scene: &mut Scene) -> Result<bool, Error> {
        if self.gl.is_context_lost() {
            return Ok(false);
        }

        self.program.invalidate();
        self.present_program.invalidate();
        self.read_convolution_buffers_shader.invalidate();
        self.fft_shader.invalidate();
        self.load_convolution_buffers_shader.invalidate();
        self.test_shader.invalidate();
        self.visible_point_update_pixels_shader.invalidate();
        self.visible_point_gen_shader.invalidate();
        self.camera_buffer.invalidate();
        self.geometry_buffer.invalidate();
        self.material_buffer.invalidate();
        self.instance_buffer.invalidate();
        self.display_buffer.invalidate();
        self.envmap_marg_cdf.invalidate();
        self.envmap_cond_cdf.invalidate();
        self.envmap_texture.invalidate();
        self.globals_buffer.invalidate();
        self.raster_buffer.invalidate();
        self.samples.invalidate();
        self.samples_fbo.invalidate();
        self.rspectrum_temp1.invalidate();
        self.gspectrum_temp1.invalidate();
        self.bspectrum_temp1.invalidate();

        self.rspectrum_temp2.invalidate();
        self.gspectrum_temp2.invalidate();
        self.bspectrum_temp2.invalidate();

        self.r_aperture_spectrum.invalidate();
        self.g_aperture_spectrum.invalidate();
        self.b_aperture_spectrum.invalidate();

        self.render.invalidate();
        self.fft_pass_data.invalidate();
        self.spectrum_temp1_fbo.invalidate();
        self.spectrum_temp2_fbo.invalidate();
        self.render_fbo.invalidate();
        self.photon_fbo.invalidate();
        self.aperture_fbo.invalidate();

        self.visible_point_count_a.invalidate();
        self.visible_point_count_b.invalidate();
        self.visible_point_data_a.invalidate();
        self.visible_point_data_b.invalidate();
        self.visible_point_path1.invalidate();
        self.visible_point_path2.invalidate();
        self.visible_point_path3.invalidate();
        self.visible_point_pass_data.invalidate();

        self.visible_point_a_fbo.invalidate();
        self.visible_point_b_fbo.invalidate();
        self.visible_point_path_fbo.invalidate();
        self.visible_point_pass_data_fbo.invalidate();

        scene.dirty_all_fields();
        self.device_lost = false;

        Ok(true)
    }

    fn reset_refinement(&mut self) {
        self.samples_fbo.clear(0, [0.0, 0.0, 0.0, 0.0]);
    }
}

#[derive(Debug)]
pub(crate) struct DeviceState {
    pub(crate) rng: ChaCha20Rng,
    pub(crate) filter_rng: Qrng,

    pub(crate) filter: RasterFilter,

    pub(crate) enable_lens_flare: bool,

    pub(crate) frame: u32,

    pub(crate) integrator: Integrator,
    pub(crate) search_radius: f32,
    pub(crate) total_photons: f32,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            rng: ChaCha20Rng::seed_from_u64(0),
            filter_rng: Qrng::new(0),
            filter: RasterFilter::default(),
            enable_lens_flare: false,
            search_radius: 0.0,
            integrator: Integrator::default(),
            total_photons: 0.0,
            frame: 0,
        }
    }
}

impl DeviceState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self, scene: &mut Scene) {
        *self = Self::new();

        self.enable_lens_flare = scene.aperture.is_some();

        self.filter = scene.raster.filter;
        self.integrator = *scene.integrator;
        self.search_radius = scene.integrator.initial_search_radius;
    }

    pub fn update(
        &mut self,
        allocator: &mut Allocator,
        buffer: &mut UniformBuffer<GlobalData>,
        photons_for_pass: u32,
        grid_cell_size: f32,
        hash_cell_cols: u32,
        hash_cell_rows: u32,
    ) {
        // we don't want the first (0, 0) sample from the sequence
        let (mut x, mut y) = self.filter_rng.next::<(f32, f32)>();

        if x == 0.0 && y == 0.0 {
            x = 0.5;
            y = 0.5;
        }

        self.total_photons += photons_for_pass as f32;

        let data: &mut GlobalData = allocator.allocate_one();

        data.filter_delta[0] = 4.0 * self.filter.importance_sample(x) - 2.0;
        data.filter_delta[1] = 4.0 * self.filter.importance_sample(y) - 2.0;
        data.frame_state[0] = self.rng.next_u32();
        data.frame_state[1] = self.rng.next_u32();
        data.frame_state[2] = self.frame;
        data.pass_count = (1 + self.frame) as f32;
        data.photons_for_pass = photons_for_pass as f32;
        data.total_photons = self.total_photons;
        data.grid_cell_size = grid_cell_size;
        data.hash_cell_cols = hash_cell_cols;
        data.hash_cell_rows = hash_cell_rows;

        buffer.write(&data).expect("internal WebGL error");

        self.frame += 1;
    }
}

#[repr(C)]
#[derive(AsBytes, FromBytes, Debug)]
pub(crate) struct GlobalData {
    filter_delta: [f32; 4],
    frame_state: [u32; 4],
    pass_count: f32,
    photons_for_pass: f32,
    total_photons: f32,
    grid_cell_size: f32,
    hash_cell_cols: u32,
    hash_cell_rows: u32,
    padding: [f32; 3],
}
