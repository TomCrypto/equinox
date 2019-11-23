use js_sys::Error;
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
    pub(crate) environment_buffer: UniformBuffer<EnvironmentData>,
    pub(crate) gather_quasi_buffer: UniformBuffer<[SamplerDimensionAlpha]>,
    pub(crate) scatter_quasi_buffer: UniformBuffer<[SamplerDimensionAlpha]>,

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
    pub(crate) render: Texture<RGBA16F>,

    pub(crate) fft_pass_data: VertexArray<[FFTPassData]>,

    pub(crate) spectrum_temp1_fbo: Framebuffer,
    pub(crate) spectrum_temp2_fbo: Framebuffer,
    pub(crate) render_fbo: Framebuffer,
    pub(crate) aperture_fbo: Framebuffer,

    pub(crate) load_convolution_buffers_shader: Shader,

    pub(crate) integrator_photon_table_pos: Texture<RGBA32F>,
    pub(crate) integrator_photon_table_dir: Texture<RGB10A2>,
    pub(crate) integrator_photon_table_sum: Texture<RGBA16F>,

    pub(crate) integrator_radiance_estimate: Texture<RGBA32F>,

    pub(crate) integrator_scatter_fbo: Framebuffer,
    pub(crate) integrator_gather_fbo: Framebuffer,

    pub(crate) integrator_scatter_photons_shader: Shader,
    pub(crate) integrator_gather_photons_shader: Shader,

    device_lost: bool,

    pub(crate) state: IntegratorState,
}

impl Device {
    /// Creates a new device using a WebGL2 context.
    pub fn new(gl: &Context) -> Result<Self, Error> {
        Ok(Self {
            gl: gl.clone(),

            integrator_radiance_estimate: Texture::new(gl.clone()),

            integrator_gather_fbo: Framebuffer::new(gl.clone()),

            integrator_gather_photons_shader: Shader::new(
                gl.clone(),
                &shader::VS_FULLSCREEN,
                &shader::FS_GATHER_PHOTONS,
            ),
            integrator_photon_table_pos: Texture::new(gl.clone()),
            integrator_photon_table_dir: Texture::new(gl.clone()),
            integrator_photon_table_sum: Texture::new(gl.clone()),
            fft_pass_data: VertexArray::new(gl.clone()),
            integrator_scatter_photons_shader: Shader::new(
                gl.clone(),
                &shader::VS_SCATTER_PHOTONS,
                &shader::FS_SCATTER_PHOTONS,
            ),
            load_convolution_buffers_shader: Shader::new(
                gl.clone(),
                &shader::VS_FULLSCREEN,
                &shader::FS_LOAD_CONVOLUTION_BUFFERS,
            ),
            fft_shader: Shader::new(gl.clone(), &shader::VS_FFT_PASS, &shader::FS_FFT_PASS),
            read_convolution_buffers_shader: Shader::new(
                gl.clone(),
                &shader::VS_FULLSCREEN,
                &shader::FS_READ_CONVOLUTION_BUFFERS,
            ),
            present_program: Shader::new(gl.clone(), &shader::VS_FULLSCREEN, &shader::FS_PRESENT),
            camera_buffer: UniformBuffer::new(gl.clone()),
            geometry_buffer: UniformBuffer::new(gl.clone()),
            material_buffer: UniformBuffer::new(gl.clone()),
            instance_buffer: UniformBuffer::new(gl.clone()),
            gather_quasi_buffer: UniformBuffer::new(gl.clone()),
            scatter_quasi_buffer: UniformBuffer::new(gl.clone()),
            raster_buffer: UniformBuffer::new(gl.clone()),
            display_buffer: UniformBuffer::new(gl.clone()),
            integrator_buffer: UniformBuffer::new(gl.clone()),
            environment_buffer: UniformBuffer::new(gl.clone()),
            envmap_texture: Texture::new(gl.clone()),
            envmap_marg_cdf: Texture::new(gl.clone()),
            envmap_cond_cdf: Texture::new(gl.clone()),
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
            integrator_scatter_fbo: Framebuffer::new(gl.clone()),
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

            for geometry in geometries.values() {
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
        let environment = &mut scene.environment;

        invalidated |= Dirty::clean(&mut scene.environment_map, |environment_map| {
            self.update_environment_map(assets, environment_map.as_ref())?;

            Dirty::dirty(environment);

            Ok(())
        })?;

        invalidated |= Dirty::clean(&mut scene.environment, |environment| {
            self.update_environment(environment)?;

            Ok(())
        })?;

        invalidated |= Dirty::clean(&mut scene.raster, |raster| {
            self.update_raster(raster)?;

            if raster.width == 0 || raster.height == 0 {
                return Err(Error::new("raster dimensions must be nonzero"));
            }

            self.render
                .create(raster.width as usize, raster.height as usize);

            let render_cols = raster.width as usize;
            let render_rows = raster.height as usize;

            self.integrator_radiance_estimate
                .create(render_cols, render_rows);

            self.integrator_gather_fbo
                .rebuild(&[&self.integrator_radiance_estimate], None)?;

            self.render_fbo.rebuild(&[&self.render], None)?;

            self.load_convolution_buffers_shader.set_define(
                "IMAGE_DIMS",
                format!(
                    "vec2({:+e}, {:+e})",
                    raster.width as f32, raster.height as f32
                ),
            );

            self.read_convolution_buffers_shader.set_define(
                "IMAGE_DIMS",
                format!(
                    "vec2({:+e}, {:+e})",
                    raster.width as f32, raster.height as f32
                ),
            );

            self.load_convolution_buffers_shader
                .set_define("CONV_DIMS", format!("vec2({:+e}, {:+e})", 2048.0, 1024.0));

            self.read_convolution_buffers_shader
                .set_define("CONV_DIMS", format!("vec2({:+e}, {:+e})", 2048.0, 1024.0));

            Ok(())
        })?;

        // this shader needs to be ready for aperture filter preprocessing
        self.fft_shader.rebuild()?;

        let assets = &scene.assets;

        invalidated |= Dirty::clean(&mut scene.aperture, |aperture| {
            if let Some(aperture) = aperture {
                self.rspectrum_temp1.create(2048, 1024);
                self.gspectrum_temp1.create(2048, 1024);
                self.bspectrum_temp1.create(2048, 1024);

                self.rspectrum_temp2.create(2048, 1024);
                self.gspectrum_temp2.create(2048, 1024);
                self.bspectrum_temp2.create(2048, 1024);

                self.r_aperture_spectrum.create(2048, 1024);
                self.g_aperture_spectrum.create(2048, 1024);
                self.b_aperture_spectrum.create(2048, 1024);

                self.aperture_fbo.rebuild(
                    &[
                        &self.r_aperture_spectrum,
                        &self.g_aperture_spectrum,
                        &self.b_aperture_spectrum,
                    ],
                    None,
                )?;

                self.spectrum_temp1_fbo.rebuild(
                    &[
                        &self.rspectrum_temp1,
                        &self.gspectrum_temp1,
                        &self.bspectrum_temp1,
                    ],
                    None,
                )?;

                self.spectrum_temp2_fbo.rebuild(
                    &[
                        &self.rspectrum_temp2,
                        &self.gspectrum_temp2,
                        &self.bspectrum_temp2,
                    ],
                    None,
                )?;

                self.prepare_fft_pass_data();

                self.preprocess_filter(
                    &assets[&aperture.aperture_texels],
                    aperture.aperture_width as usize,
                    aperture.aperture_height as usize,
                );
            }

            Ok(())
        })?;

        invalidated |= Dirty::clean(&mut scene.integrator, |integrator| {
            self.update_integrator(integrator)?;

            // TODO: return an error if the texture is too large to create here...
            // (check against the size limits or something)

            let col_bits = integrator.hash_table_bits / 2;
            let row_bits = integrator.hash_table_bits - col_bits;

            let cols = 2usize.pow(col_bits);
            let rows = 2usize.pow(row_bits);

            self.integrator_photon_table_pos.create(cols, rows);
            self.integrator_photon_table_dir.create(cols, rows);
            self.integrator_photon_table_sum.create(cols, rows);

            self.integrator_scatter_fbo.rebuild(
                &[
                    &self.integrator_photon_table_pos,
                    &self.integrator_photon_table_dir,
                    &self.integrator_photon_table_sum,
                ],
                None,
            )?;

            Ok(())
        })?;

        // These are post-processing settings that don't directly apply to the light
        // transport simulation; we don't need to invalidate any render buffer here.

        Dirty::clean(&mut scene.display, |display| {
            self.update_display(display)?;

            Ok(())
        })?;

        self.present_program.rebuild()?;

        self.integrator_scatter_photons_shader.rebuild()?;
        self.integrator_gather_photons_shader.rebuild()?;

        self.read_convolution_buffers_shader.rebuild()?;
        self.load_convolution_buffers_shader.rebuild()?;

        if invalidated {
            self.reset_integrator_state(scene);
        }

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
            command.bind(&self.integrator_radiance_estimate, "samples");
        }

        command.bind(&self.display_buffer, "Display");

        command.set_viewport(
            0,
            0,
            self.integrator_gather_fbo.cols() as i32,
            self.integrator_gather_fbo.rows() as i32,
        );

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
        self.environment_buffer.invalidate();
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

        self.integrator_photon_table_pos.invalidate();
        self.integrator_photon_table_dir.invalidate();
        self.integrator_photon_table_sum.invalidate();
        self.integrator_scatter_fbo.invalidate();
        self.aperture_fbo.invalidate();
        self.gather_quasi_buffer.invalidate();
        self.scatter_quasi_buffer.invalidate();

        self.integrator_radiance_estimate.invalidate();

        self.integrator_gather_fbo.invalidate();

        self.integrator_scatter_photons_shader.invalidate();
        self.integrator_gather_photons_shader.invalidate();

        scene.dirty_all_fields();
        self.device_lost = false;

        Ok(true)
    }
}
