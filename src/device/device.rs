use js_sys::Error;
use web_sys::WebGl2RenderingContext as Context;

use crate::*;

#[derive(Debug)]
pub struct Device {
    pub(crate) gl: Context,

    pub(crate) present_program: Shader,

    pub(crate) fft_shader: Shader,

    pub(crate) load_signal_tile_shader: Shader,
    pub(crate) read_signal_tile_shader: Shader,

    pub(crate) geometry_buffer: UniformBuffer<[GeometryParameter]>,
    pub(crate) material_buffer: UniformBuffer<[MaterialParameter]>,
    pub(crate) instance_buffer: UniformBuffer<[SceneInstanceNode]>,

    pub(crate) envmap_marg_cdf: Texture<R16F>,
    pub(crate) envmap_cond_cdf: Texture<R16F>,
    pub(crate) envmap_color: Texture<RGBA16F>,

    pub(crate) display_buffer: UniformBuffer<DisplayData>,
    pub(crate) camera_buffer: UniformBuffer<CameraData>,
    pub(crate) integrator_buffer: UniformBuffer<IntegratorData>,
    pub(crate) raster_buffer: UniformBuffer<RasterData>,
    pub(crate) environment_buffer: UniformBuffer<EnvironmentData>,
    pub(crate) gather_quasi_buffer: UniformBuffer<[SamplerDimensionAlpha]>,
    pub(crate) scatter_quasi_buffer: UniformBuffer<[SamplerDimensionAlpha]>,

    pub(crate) fft_signal_tile_r: Texture<RG32F>,
    pub(crate) fft_signal_tile_g: Texture<RG32F>,
    pub(crate) fft_signal_tile_b: Texture<RG32F>,

    pub(crate) fft_filter_tile_r: Vec<Texture<RG32F>>,
    pub(crate) fft_filter_tile_g: Vec<Texture<RG32F>>,
    pub(crate) fft_filter_tile_b: Vec<Texture<RG32F>>,

    pub(crate) fft_temp_tile_r: Texture<RG32F>,
    pub(crate) fft_temp_tile_g: Texture<RG32F>,
    pub(crate) fft_temp_tile_b: Texture<RG32F>,

    pub(crate) fft_signal_fbo: Framebuffer,
    pub(crate) fft_filter_fbo: Vec<Framebuffer>,
    pub(crate) fft_temp_fbo: Framebuffer,

    // Initial convolution signal, saved from the radiance estimate
    pub(crate) convolution_signal: Texture<RGBA16F>,

    // Final convolved render output (real-valued)
    pub(crate) convolution_output: Texture<RGBA16F>,

    pub(crate) composited_render: Texture<RGBA8>,
    pub(crate) composited_fbo: Framebuffer,

    pub(crate) signal_fft_passes: VertexArray<[FFTPassData]>,
    pub(crate) filter_fft_passes: VertexArray<[FFTPassData]>,

    pub(crate) convolution_output_fbo: Framebuffer,

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

            composited_render: Texture::new(gl.clone()),
            composited_fbo: Framebuffer::new(gl.clone()),

            integrator_radiance_estimate: Texture::new(gl.clone()),

            integrator_gather_fbo: Framebuffer::new(gl.clone()),

            load_signal_tile_shader: Shader::new(
                gl.clone(),
                &shader::VS_FULLSCREEN,
                &shader::FS_LOAD_SIGNAL_TILE,
            ),

            read_signal_tile_shader: Shader::new(
                gl.clone(),
                &shader::VS_FULLSCREEN,
                &shader::FS_READ_SIGNAL_TILE,
            ),

            fft_signal_tile_r: Texture::new(gl.clone()),
            fft_signal_tile_g: Texture::new(gl.clone()),
            fft_signal_tile_b: Texture::new(gl.clone()),

            fft_filter_tile_r: vec![],
            fft_filter_tile_g: vec![],
            fft_filter_tile_b: vec![],

            fft_temp_tile_r: Texture::new(gl.clone()),
            fft_temp_tile_g: Texture::new(gl.clone()),
            fft_temp_tile_b: Texture::new(gl.clone()),

            fft_signal_fbo: Framebuffer::new(gl.clone()),
            fft_filter_fbo: vec![],
            fft_temp_fbo: Framebuffer::new(gl.clone()),

            integrator_gather_photons_shader: Shader::new(
                gl.clone(),
                &shader::VS_FULLSCREEN,
                &shader::FS_GATHER_PHOTONS,
            ),
            integrator_photon_table_pos: Texture::new(gl.clone()),
            integrator_photon_table_dir: Texture::new(gl.clone()),
            integrator_photon_table_sum: Texture::new(gl.clone()),
            signal_fft_passes: VertexArray::new(gl.clone()),
            filter_fft_passes: VertexArray::new(gl.clone()),
            integrator_scatter_photons_shader: Shader::new(
                gl.clone(),
                &shader::VS_SCATTER_PHOTONS,
                &shader::FS_SCATTER_PHOTONS,
            ),
            fft_shader: Shader::new(gl.clone(), &shader::VS_FFT_PASS, &shader::FS_FFT_PASS),
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
            envmap_color: Texture::new(gl.clone()),
            envmap_marg_cdf: Texture::new(gl.clone()),
            envmap_cond_cdf: Texture::new(gl.clone()),
            convolution_output: Texture::new(gl.clone()),
            convolution_signal: Texture::new(gl.clone()),
            convolution_output_fbo: Framebuffer::new(gl.clone()),
            integrator_scatter_fbo: Framebuffer::new(gl.clone()),
            device_lost: true,
            state: IntegratorState::default(),
        })
    }

    /// Signals the context was lost.
    pub fn context_lost(&mut self) {
        self.device_lost = true;
    }

    /// Determines whether a device update with a scene may be time-consuming.
    pub fn is_update_expensive(&self, scene: &Scene) -> Result<bool, Error> {
        if self.device_lost {
            return Ok(false);
        }

        scene.validate()?;

        let mut expensive = false;

        expensive |= Dirty::is_dirty(&scene.geometry_list);
        expensive |= Dirty::is_dirty(&scene.environment_map);
        expensive |= Dirty::is_dirty(&scene.aperture);

        Ok(expensive)
    }

    /// Updates this device to render a given scene or returns an error.
    pub fn update(&mut self, scene: &mut Scene) -> Result<bool, Error> {
        if self.device_lost && !self.try_restore(scene)? {
            return Ok(false); // context currently lost
        }

        scene.validate()?;

        // We do nothing with the scene metadata object
        Dirty::clean(&mut scene.metadata, |_| Ok(()))?;

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
            self.update_environment_map(assets, environment_map.as_ref().map(String::as_str))?;

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

            self.composited_render
                .create(raster.width as usize, raster.height as usize);
            self.composited_fbo
                .rebuild(&[&self.composited_render], None)?;

            self.convolution_signal
                .create(raster.width as usize, raster.height as usize);
            self.convolution_output
                .create(raster.width as usize, raster.height as usize);

            let render_cols = raster.width as usize;
            let render_rows = raster.height as usize;

            self.integrator_radiance_estimate
                .create(render_cols, render_rows);

            self.integrator_gather_fbo
                .rebuild(&[&self.integrator_radiance_estimate], None)?;

            self.convolution_output_fbo
                .rebuild(&[&self.convolution_output], None)?;

            Ok(())
        })?;

        // this shader needs to be ready for aperture filter preprocessing
        self.fft_shader.rebuild()?;

        let assets = &scene.assets;

        invalidated |= Dirty::clean(&mut scene.aperture, |aperture| {
            // TODO: create relevant FFT buffers here
            // the BUFFERS themselves only depend on the tile size, but the filter buffers
            // may need to be expanded/shrunk if the raster changes and new
            // tiles are added/removed

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

                // TODO: move this to the lens_flare file
                self.generate_signal_fft_passes(Self::TILE_SIZE);
                self.generate_filter_fft_passes(Self::TILE_SIZE);

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

        if invalidated {
            self.reset_integrator_state(scene);
            // TODO: reset tiled convolution state
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

        // lens flare pass

        let use_lens_flare = false;

        if use_lens_flare {
            // do something
        } else {
            // directly post-process the radiance estimate
        }

        /*

        TODO: add lens flare switch logic here

         - if lens flare is not enabled, just directly use the radiance estimate
         - else, need some custom logic to use the radiance estimate if we have NO
           lens flare data yet, else use the latest convolution buffer
           (do we need yet another temporary buffer here? I don't think so, simply
            don't update the composited render if we don't want to change anything)

        */

        /*if self.state.aperture.is_some() {
            self.render_lens_flare();
        }*/

        // postproc pass; if we get here the output will be updated

        let command = self.present_program.begin_draw();

        command.bind(&self.integrator_radiance_estimate, "samples");

        command.bind(&self.display_buffer, "Display");

        command.set_viewport(
            0,
            0,
            self.integrator_gather_fbo.cols() as i32,
            self.integrator_gather_fbo.rows() as i32,
        );

        command.set_framebuffer(&self.composited_fbo);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);

        Ok(())
    }

    /// Presents the current render state into the context's canvas.
    pub fn present(&mut self) -> Result<(), Error> {
        if self.device_lost {
            return Ok(());
        }

        self.composited_fbo.blit_color_to_canvas();
        Ok(()) // just blit our final color buffer
    }

    fn try_restore(&mut self, scene: &mut Scene) -> Result<bool, Error> {
        if self.gl.is_context_lost() {
            return Ok(false);
        }

        // TODO: invalidate all new FFT fields

        self.composited_render.invalidate();
        self.composited_fbo.invalidate();

        self.present_program.invalidate();
        self.fft_shader.invalidate();
        self.camera_buffer.invalidate();
        self.geometry_buffer.invalidate();
        self.material_buffer.invalidate();
        self.instance_buffer.invalidate();
        self.display_buffer.invalidate();
        self.envmap_marg_cdf.invalidate();
        self.envmap_cond_cdf.invalidate();
        self.envmap_color.invalidate();
        self.integrator_buffer.invalidate();
        self.raster_buffer.invalidate();
        self.environment_buffer.invalidate();

        self.convolution_output.invalidate();
        self.signal_fft_passes.invalidate();
        self.filter_fft_passes.invalidate();
        self.convolution_output_fbo.invalidate();

        self.integrator_photon_table_pos.invalidate();
        self.integrator_photon_table_dir.invalidate();
        self.integrator_photon_table_sum.invalidate();
        self.integrator_scatter_fbo.invalidate();
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
