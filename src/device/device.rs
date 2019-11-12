use js_sys::Error;
use maplit::hashmap;
use web_sys::WebGl2RenderingContext as Context;

use crate::*;

#[derive(Debug)]
pub struct Device {
    pub(crate) gl: Context,

    pub(crate) present_program: Shader,

    pub(crate) read_convolution_buffers_shader: Shader,
    pub(crate) fft_shader: Shader,

    pub(crate) geometry_buffer: UniformBuffer<[GeometryParameter]>,
    pub(crate) material_buffer: UniformBuffer<[MaterialParameter]>,
    pub(crate) instance_buffer: UniformBuffer<[SceneInstanceNode]>,

    pub(crate) envmap_marg_cdf: Texture<R16F>,
    pub(crate) envmap_cond_cdf: Texture<R16F>,
    pub(crate) envmap_texture: Texture<RGBA16F>,

    pub(crate) display_buffer: UniformBuffer<DisplayData>,
    pub(crate) camera_buffer: UniformBuffer<CameraData>,
    pub(crate) integrator_buffer: UniformBuffer<IntegratorData>,
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

    pub(crate) photon_hash_table_major: Texture<RGBA8>,
    pub(crate) photon_hash_table_minor: Texture<RGBA16F>,

    pub(crate) photon_fbo: Framebuffer,

    pub(crate) integrator_scatter_photons_shader: Shader,

    pub(crate) integrator_ld_count: Texture<RGBA32F>,
    pub(crate) integrator_li_count: Texture<RGBA16F>,
    pub(crate) integrator_li_range: Texture<RGBA32F>,

    pub(crate) integrator_gather_fbo: Framebuffer,
    pub(crate) integrator_update_fbo: Framebuffer,

    pub(crate) integrator_estimate_radiance_shader: Shader,
    pub(crate) integrator_update_estimates_shader: Shader,

    pub(crate) integrator_gather_photons_shader: Shader,

    pub(crate) allocator: Allocator,

    device_lost: bool,

    pub(crate) state: IntegratorState,
}

impl Device {
    /// Creates a new device using a WebGL2 context.
    pub fn new(gl: &Context) -> Result<Self, Error> {
        Ok(Self {
            allocator: Allocator::new(),
            gl: gl.clone(),

            integrator_ld_count: Texture::new(gl.clone()),
            integrator_li_count: Texture::new(gl.clone()),
            integrator_li_range: Texture::new(gl.clone()),

            integrator_gather_fbo: Framebuffer::new(gl.clone()),
            integrator_update_fbo: Framebuffer::new(gl.clone()),

            integrator_update_estimates_shader: Shader::new(
                gl.clone(),
                shaders::VS_FULLSCREEN,
                shaders::FS_UPDATE_ESTIMATES,
                hashmap! {
                    "Integrator" => BindingPoint::UniformBlock(0),
                    "ld_count_tex" => BindingPoint::Texture(0),
                    "li_count_tex" => BindingPoint::Texture(1),
                },
                hashmap! {},
                hashmap! {},
            ),

            integrator_estimate_radiance_shader: Shader::new(
                gl.clone(),
                shaders::VS_FULLSCREEN,
                shaders::FS_ESTIMATE_RADIANCE,
                hashmap! {
                    "Integrator" => BindingPoint::UniformBlock(0),
                    "ld_count_tex" => BindingPoint::Texture(0),
                    "li_range_tex" => BindingPoint::Texture(1),
                },
                hashmap! {},
                hashmap! {},
            ),

            integrator_gather_photons_shader: Shader::new(
                gl.clone(),
                shaders::VS_FULLSCREEN,
                shaders::FS_GATHER_PHOTONS,
                hashmap! {
                    "Camera" => BindingPoint::UniformBlock(0),
                    "Instance" => BindingPoint::UniformBlock(4),
                    "Geometry" => BindingPoint::UniformBlock(7),
                    "Material" => BindingPoint::UniformBlock(8),
                    "Integrator" => BindingPoint::UniformBlock(2),
                    "Raster" => BindingPoint::UniformBlock(3),
                    "envmap_texture" => BindingPoint::Texture(1),
                    "envmap_marg_cdf" => BindingPoint::Texture(2),
                    "envmap_cond_cdf" => BindingPoint::Texture(3),
                    "li_range_tex" => BindingPoint::Texture(4),
                    "photon_table_major" => BindingPoint::Texture(5),
                    "photon_table_minor" => BindingPoint::Texture(6),
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
                },
            ),
            photon_hash_table_major: Texture::new(gl.clone()),
            photon_hash_table_minor: Texture::new(gl.clone()),
            fft_pass_data: VertexArray::new(gl.clone()),
            integrator_scatter_photons_shader: Shader::new(
                gl.clone(),
                shaders::VS_SCATTER_PHOTONS,
                shaders::FS_SCATTER_PHOTONS,
                hashmap! {
                    "Instance" => BindingPoint::UniformBlock(4),
                    "Geometry" => BindingPoint::UniformBlock(7),
                    "Material" => BindingPoint::UniformBlock(8),
                    "Integrator" => BindingPoint::UniformBlock(2),
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
            integrator_buffer: UniformBuffer::new(gl.clone()),
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
            device_lost: true,
            state: IntegratorState::default(),
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

            self.integrator_gather_photons_shader
                .set_header("geometry-user.glsl", &code);
            self.integrator_scatter_photons_shader
                .set_header("geometry-user.glsl", &code);

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

            if raster.width == 0 || raster.height == 0 {
                return Err(Error::new("raster dimensions must be nonzero"));
            }

            self.samples
                .create(raster.width as usize, raster.height as usize);

            self.render
                .create(raster.width as usize, raster.height as usize);

            self.samples_fbo.rebuild(&[(&self.samples, 0)]);

            // Configure the shaders with the desired resolutions...

            self.load_convolution_buffers_shader
                .set_define("CONV_DIMS", format!("vec2({:+e}, {:+e})", 2048.0, 1024.0));

            self.load_convolution_buffers_shader.set_define(
                "IMAGE_DIMS",
                format!(
                    "vec2({:+e}, {:+e})",
                    raster.width as f32, raster.height as f32
                ),
            );

            self.read_convolution_buffers_shader
                .set_define("CONV_DIMS", format!("vec2({:+e}, {:+e})", 2048.0, 1024.0));

            self.read_convolution_buffers_shader.set_define(
                "IMAGE_DIMS",
                format!(
                    "vec2({:+e}, {:+e})",
                    raster.width as f32, raster.height as f32
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

            let render_cols = raster.width as usize;
            let render_rows = raster.height as usize;

            self.integrator_ld_count.create(render_cols, render_rows);
            self.integrator_li_count.create(render_cols, render_rows);
            self.integrator_li_range.create(render_cols, render_rows);

            self.integrator_gather_fbo.rebuild(&[
                (&self.integrator_ld_count, 0),
                (&self.integrator_li_count, 0),
            ]);

            self.integrator_update_fbo
                .rebuild(&[(&self.integrator_li_range, 0)]);

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

            Ok(())
        })?;

        // These are post-processing settings that don't directly apply to the light
        // transport simulation; we don't need to invalidate any render buffer here.

        Dirty::clean(&mut scene.display, |display| {
            self.update_display(display)?;

            Ok(())
        })?;

        self.present_program.rebuild()?;

        self.integrator_estimate_radiance_shader.rebuild()?;
        self.integrator_update_estimates_shader.rebuild()?;

        self.read_convolution_buffers_shader.rebuild()?;
        self.fft_shader.rebuild()?;
        self.load_convolution_buffers_shader.rebuild()?;
        self.integrator_scatter_photons_shader.rebuild()?;
        self.integrator_gather_photons_shader.rebuild()?;

        if invalidated {
            self.reset_integrator_state(scene);
        }

        self.allocator.shrink_to_watermark();

        Ok(invalidated)
    }

    /// Refines the current render state by performing an SPPM pass.
    pub fn refine(&mut self) -> Result<(), Error> {
        if self.device_lost {
            return Ok(());
        }

        let pass = self.prepare_integrator_pass();

        self.update_integrator_state(&pass)?;
        self.scatter_photons(&pass);
        self.gather_photons();
        self.update_estimates();
        self.estimate_radiance();

        Ok(())
    }

    /// Renders the current render state into the context's canvas.
    pub fn render(&mut self) -> Result<(), Error> {
        if self.device_lost {
            return Ok(());
        }

        if self.state.aperture.is_some() {
            self.render_lens_flare();
        }

        let command = self.present_program.begin_draw();

        if self.state.aperture.is_some() {
            command.bind(&self.render, "samples");
        } else {
            command.bind(&self.samples, "samples");
        }

        command.bind(&self.display_buffer, "Display");

        command.set_viewport(0, 0, self.samples.cols() as i32, self.samples.rows() as i32);

        command.set_canvas_framebuffer();

        command.unset_vertex_array();
        command.draw_triangles(0, 1);

        Ok(())
    }

    fn try_restore(&mut self, scene: &mut Scene) -> Result<bool, Error> {
        if self.gl.is_context_lost() {
            return Ok(false);
        }

        self.present_program.invalidate();
        self.read_convolution_buffers_shader.invalidate();
        self.fft_shader.invalidate();
        self.load_convolution_buffers_shader.invalidate();
        self.integrator_scatter_photons_shader.invalidate();
        self.integrator_gather_photons_shader.invalidate();
        self.camera_buffer.invalidate();
        self.geometry_buffer.invalidate();
        self.material_buffer.invalidate();
        self.instance_buffer.invalidate();
        self.display_buffer.invalidate();
        self.envmap_marg_cdf.invalidate();
        self.envmap_cond_cdf.invalidate();
        self.envmap_texture.invalidate();
        self.integrator_buffer.invalidate();
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

        self.integrator_ld_count.invalidate();
        self.integrator_li_count.invalidate();
        self.integrator_li_range.invalidate();

        self.integrator_gather_fbo.invalidate();
        self.integrator_update_fbo.invalidate();

        self.integrator_estimate_radiance_shader.invalidate();
        self.integrator_update_estimates_shader.invalidate();

        scene.dirty_all_fields();
        self.device_lost = false;

        Ok(true)
    }
}
