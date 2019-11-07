use js_sys::Error;
use maplit::hashmap;
use quasirandom::Qrng;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use web_sys::WebGl2RenderingContext as Context;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(AsBytes, FromBytes, Debug, Clone, Copy)]
pub(crate) struct PixelInfo {
    color: [f32; 3],
    radius: f32,
}

impl PixelInfo {
    fn key(&self) -> f32 {
        if self.color == [0.0; 3] {
            0.0
        } else {
            self.radius
        }
    }
}

impl PartialOrd for PixelInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PixelInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().partial_cmp(&other.key()).unwrap()
    }
}

impl PartialEq for PixelInfo {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for PixelInfo {}

use crate::*;

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

    // textures for the photon hash table (major has the position information)
    pub(crate) photon_hash_table_major: Texture<RGBA16F>,
    pub(crate) photon_hash_table_minor: Texture<RGBA16F>,

    pub(crate) photon_fbo: Framebuffer,

    pub(crate) test_shader: Shader,

    // ping-pong buffers for the visible point data
    pub(crate) visible_point_count_a: Texture<RGBA32F>,
    pub(crate) visible_point_count_b: Texture<RGBA32F>,
    pub(crate) visible_point_data_a: Texture<RGBA32F>,
    pub(crate) visible_point_data_b: Texture<RGBA32F>,
    pub(crate) visible_point_a_readback_fbo: Framebuffer,
    pub(crate) visible_point_b_readback_fbo: Framebuffer,

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

    pub(crate) radius_readback: ReadbackBuffer<[PixelInfo]>,

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
            visible_point_a_readback_fbo: Framebuffer::new(gl.clone()),
            visible_point_b_readback_fbo: Framebuffer::new(gl.clone()),
            radius_readback: ReadbackBuffer::new(gl.clone()),
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
                hashmap! {},
                hashmap! {},
            ),
            photon_hash_table_major: Texture::new(gl.clone()),
            photon_hash_table_minor: Texture::new(gl.clone()),
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
                },
            ),
            load_convolution_buffers_shader: Shader::new(
                gl.clone(),
                shaders::VS_FULLSCREEN,
                shaders::FS_LOAD_CONVOLUTION_BUFFERS,
                hashmap! {
                    "image" => BindingPoint::Texture(0),
                },
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
                    "photon_table_major" => BindingPoint::Texture(4),
                    "photon_table_minor" => BindingPoint::Texture(6),
                    "photon_radius_tex" => BindingPoint::Texture(5),
                    "visible_point_path_buf1" => BindingPoint::Texture(0),
                    "visible_point_path_buf2" => BindingPoint::Texture(1),
                    "visible_point_path_buf3" => BindingPoint::Texture(2),
                },
                hashmap! {},
                hashmap! {
                    "MATERIAL_DATA_COUNT" => "0",
                    "HASH_TABLE_COLS" => "0",
                    "HASH_TABLE_ROWS" => "0",
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

            self.samples_fbo.rebuild(&[(&self.samples, 0)]);

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
                .create_mipped(raster.width.get() as usize, raster.height.get() as usize);
            self.visible_point_data_b
                .create_mipped(raster.width.get() as usize, raster.height.get() as usize);
            self.visible_point_path1
                .create(raster.width.get() as usize, raster.height.get() as usize);
            self.visible_point_path2
                .create(raster.width.get() as usize, raster.height.get() as usize);
            self.visible_point_path3
                .create(raster.width.get() as usize, raster.height.get() as usize);
            self.visible_point_pass_data
                .create(raster.width.get() as usize, raster.height.get() as usize);

            self.visible_point_a_fbo.rebuild(&[
                (&self.visible_point_count_a, 0),
                (&self.visible_point_data_a, 0),
                (&self.samples, 0),
            ]);
            self.visible_point_a_readback_fbo
                .rebuild(&[(&self.visible_point_data_a, 0)]);
            self.visible_point_b_readback_fbo
                .rebuild(&[(&self.visible_point_data_b, 0)]);
            self.visible_point_b_fbo.rebuild(&[
                (&self.visible_point_count_b, 0),
                (&self.visible_point_data_b, 0),
                (&self.samples, 0),
            ]);
            self.visible_point_path_fbo.rebuild(&[
                (&self.visible_point_path1, 0),
                (&self.visible_point_path2, 0),
                (&self.visible_point_path3, 0),
            ]);
            self.visible_point_pass_data_fbo
                .rebuild(&[(&self.visible_point_pass_data, 0)]);

            let (mipped_cols, mipped_rows) = self.visible_point_data_a.level_dimensions(0);

            self.radius_readback.create(mipped_cols * mipped_rows);

            self.render_fbo.rebuild(&[(&self.render, 0)]);
            self.aperture_fbo.rebuild(&[
                (&self.r_aperture_spectrum, 0),
                (&self.g_aperture_spectrum, 0),
                (&self.b_aperture_spectrum, 0),
            ]);

            self.spectrum_temp1_fbo.rebuild(&[
                (&self.rspectrum_temp1, 0),
                (&self.gspectrum_temp1, 0),
                (&self.bspectrum_temp1, 0),
            ]);

            self.spectrum_temp2_fbo.rebuild(&[
                (&self.rspectrum_temp2, 0),
                (&self.gspectrum_temp2, 0),
                (&self.bspectrum_temp2, 0),
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

            self.photon_hash_table_major.create(cols, rows);
            self.photon_hash_table_minor.create(cols, rows);
            self.photon_fbo.rebuild(&[
                (&self.photon_hash_table_major, 0),
                (&self.photon_hash_table_minor, 0),
            ]);

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

        // These are post-processing settings that don't directly apply to the light
        // transport simulation; we don't need to invalidate any render buffer here.

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

    /*

    Optimize n * 2^s, under the constraints:

    0 < n <= target
    0 <= s < max_s

    n * 2^s <= max_load

    */

    // given a target number of photons and an absolute maximum load, calculate the
    // largest n * 2^s <= max_load such that n < target. all else being equal,
    // prefer smaller m if possible.
    fn calculate_photon_batch(&self, max_load: usize, target: usize) -> (usize, usize) {
        let mut best_n = 0;
        let mut best_m = 0;

        for s in 0..self.state.integrator.max_hash_cell_bits {
            let m = 1 << s;
            let n = (max_load / m).min(target);

            if n * m > best_n * best_m {
                best_n = n;
                best_m = m;
            }
        }

        (best_n, best_m)
    }

    fn get_hash_cell_dimensions(mut m: usize) -> (usize, usize) {
        let mut cols = 1;
        let mut rows = 1;

        while m != 1 {
            if m >= 4 {
                cols *= 2;
                rows *= 2;
                m /= 4;
            } else {
                cols *= 2;
                m /= 2;
            }
        }

        (cols, rows)
    }

    /// Further refines the rendering.
    pub fn refine(&mut self) {
        if self.device_lost {
            return;
        }

        // select the grid cell size
        let grid_cell_size = 2.0 * self.state.search_radius;

        let target = ((self.state.integrator.photon_density / grid_cell_size.powi(2)).round()
            as usize)
            .min(self.state.integrator.photons_per_pass)
            .max(1);

        let (n, m) = self.calculate_photon_batch(self.state.integrator.photons_per_pass, target);

        let (hash_cell_cols, hash_cell_rows) = Self::get_hash_cell_dimensions(m);

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
            self.photon_hash_table_major.cols() as i32,
            self.photon_hash_table_minor.rows() as i32,
        );
        command.set_framebuffer(&self.photon_fbo);
        self.photon_fbo.clear(0, [-1.0; 4]);
        self.photon_fbo.clear(1, [-1.0; 4]);

        command.unset_vertex_array();
        command.draw_points_instanced(n, m);

        // GATHER PHOTONS BACK INTO VISIBLE POINTS

        let command = self.program.begin_draw();

        command.bind(&self.material_buffer, "Material");
        command.bind(&self.globals_buffer, "Globals");
        command.bind(&self.photon_hash_table_major, "photon_table_major");
        command.bind(&self.photon_hash_table_minor, "photon_table_minor");
        command.bind(&self.visible_point_path1, "visible_point_path_buf1");
        command.bind(&self.visible_point_path2, "visible_point_path_buf2");
        command.bind(&self.visible_point_path3, "visible_point_path_buf3");

        if iteration % 2 == 0 {
            command.bind(&self.visible_point_data_a, "photon_radius_tex");
        } else {
            command.bind(&self.visible_point_data_b, "photon_radius_tex");
        }

        command.set_viewport(0, 0, self.samples.cols() as i32, self.samples.rows() as i32);
        command.set_framebuffer(&self.visible_point_pass_data_fbo);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);

        // there is a previous pass; update the per-pixel state

        let command = self.visible_point_update_pixels_shader.begin_draw();
        command.bind(&self.visible_point_pass_data, "new_photon_data_tex");
        command.set_viewport(0, 0, self.samples.cols() as i32, self.samples.rows() as i32);

        if iteration % 2 == 0 {
            command.bind(&self.visible_point_count_a, "old_photon_count_tex");
            command.bind(&self.visible_point_data_a, "old_photon_data_tex");

            command.set_framebuffer(&self.visible_point_b_fbo);
        } else {
            command.bind(&self.visible_point_count_b, "old_photon_count_tex");
            command.bind(&self.visible_point_data_b, "old_photon_data_tex");

            command.set_framebuffer(&self.visible_point_a_fbo);
        }

        command.unset_vertex_array();
        command.draw_triangles(0, 1);

        let mut ratio = (self.state.total_photons_per_pixel
            + self.state.integrator.alpha * m as f32)
            / (self.state.total_photons_per_pixel + m as f32);
        ratio = ratio.sqrt();
        self.state.theoretical_radius *= ratio;
        self.state.total_photons_per_pixel += m as f32;

        // don't readback for the first few frames
        if self.state.frame != 0 && self.state.frame % 15 == 0 {
            if self.state.readback_started {
                let radius_data: &mut [PixelInfo] =
                    self.allocator.allocate(self.radius_readback.len);

                if self.radius_readback.end_readback(radius_data) {
                    self.state.readback_started = false;

                    let mut list = Vec::with_capacity(radius_data.len());

                    for i in 0..(radius_data.len()) {
                        if radius_data[i].radius < self.state.search_radius {
                            list.push(radius_data[i].radius);
                        }
                    }

                    if list.len() == 0 {
                        return;
                    }

                    let count = list.len();
                    let index = (count as f32 * 1.0) as usize - 1;

                    let kth_value = *list
                        .partition_at_index_by(index, |lhs, rhs| lhs.partial_cmp(rhs).unwrap())
                        .1;

                    self.state.search_radius = kth_value;

                    log::info!("radius = {}", kth_value);
                }
            } else {
                // no readback yet, perform one

                if iteration % 2 == 0 {
                    self.visible_point_data_b.gen_mipmaps();

                    let (mipped_cols, mipped_rows) = self.visible_point_data_a.level_dimensions(0);

                    self.radius_readback
                        .start_readback(
                            mipped_cols,
                            mipped_rows,
                            &self.visible_point_b_readback_fbo,
                            0,
                        )
                        .unwrap();
                } else {
                    self.visible_point_data_a.gen_mipmaps();

                    let (mipped_cols, mipped_rows) = self.visible_point_data_a.level_dimensions(0);

                    self.radius_readback
                        .start_readback(
                            mipped_cols,
                            mipped_rows,
                            &self.visible_point_a_readback_fbo,
                            0,
                        )
                        .unwrap();
                }

                self.state.readback_started = true;
            }
        }
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
        self.photon_hash_table_major.invalidate();
        self.photon_hash_table_minor.invalidate();
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
    pub(crate) readback_started: bool,
    pub(crate) total_photons_per_pixel: f32,
    pub(crate) theoretical_radius: f32,
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
            readback_started: false,
            total_photons_per_pixel: 0.0,
            theoretical_radius: 0.0,
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
        self.theoretical_radius = scene.integrator.initial_search_radius;
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
        data.hash_cell_col_bits = (hash_cell_cols - 1).count_ones();
        data.alpha = self.integrator.alpha;

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
    hash_cell_col_bits: u32,
    alpha: f32,
    padding: [f32; 1],
}
