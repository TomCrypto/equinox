use itertools::Position;
use js_sys::Error;
use web_sys::WebGl2RenderingContext as Context;

use crate::*;

pub struct Device {
    pub(crate) gl: Context,

    pub(crate) blit_to_canvas_shader: Shader,
    pub(crate) decompose_signal_shader: Shader,
    pub(crate) execute_fft_pass_shader: Shader,
    pub(crate) load_filter_tile_shader: Shader,
    pub(crate) load_signal_tile_shader: Shader,
    pub(crate) post_process_shader: Shader,
    pub(crate) read_signal_tile_shader: Shader,

    pub(crate) geometry_buffer: UniformBuffer<[GeometryParamData]>,
    pub(crate) material_buffer: UniformBuffer<[MaterialParamData]>,
    pub(crate) instance_buffer: UniformBuffer<[SceneInstanceNode]>,

    pub(crate) envmap_marg_cdf: Texture<R16F>,
    pub(crate) envmap_cond_cdf: Texture<R16F>,
    pub(crate) envmap_color: Texture<RGBA16F>,

    pub(crate) material_textures: Texture<SRGB_S3TC_DXT1>,
    pub(crate) loaded_textures: Vec<String>,

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

    pub(crate) fft_buffer_tile_r: Texture<RG32F>,
    pub(crate) fft_buffer_tile_g: Texture<RG32F>,
    pub(crate) fft_buffer_tile_b: Texture<RG32F>,

    pub(crate) fft_signal_fbo: Framebuffer,
    pub(crate) fft_filter_fbo: Vec<Framebuffer>,
    pub(crate) fft_buffer_fbo: Framebuffer,

    pub(crate) convolution_signal: Texture<RGBA16F>,
    pub(crate) convolution_output: Texture<RGBA16F>,

    pub(crate) composited_render: Texture<RGBA8>,
    pub(crate) composited_fbo: Framebuffer,

    pub(crate) signal_fft_passes: VertexArray<[FFTPassData]>,
    pub(crate) filter_fft_passes: VertexArray<[FFTPassData]>,

    pub(crate) convolution_signal_fbo: Framebuffer,
    pub(crate) convolution_output_fbo: Framebuffer,

    pub(crate) integrator_photon_table_pos: Texture<RGBA32F>,
    pub(crate) integrator_photon_table_dir: Texture<RGB10A2>,
    pub(crate) integrator_photon_table_sum: Texture<RGBA16F>,

    pub(crate) integrator_radiance_estimate: Texture<RGBA32F>,

    pub(crate) integrator_scatter_fbo: Framebuffer,
    pub(crate) integrator_gather_fbo: Framebuffer,

    pub(crate) integrator_scatter_photons_shader: Shader,
    pub(crate) integrator_gather_photons_shader: Shader,

    pub(crate) placeholder_texture: Texture<R8>,
    pub(crate) placeholder_texture_array: Texture<R8>,

    device_lost: bool,

    pub(crate) state: IntegratorState,
    pub(crate) postproc: PostProcState,
}

impl Device {
    /// Creates a new device using a WebGL2 context.
    pub fn new(gl: &Context) -> Result<Self, Error> {
        Ok(Self {
            gl: gl.clone(),

            material_textures: Texture::new(gl.clone()),
            loaded_textures: vec![],

            placeholder_texture: Texture::new(gl.clone()),
            placeholder_texture_array: Texture::new(gl.clone()),

            composited_render: Texture::new(gl.clone()),
            composited_fbo: Framebuffer::new(gl.clone()),
            blit_to_canvas_shader: Shader::new(
                gl.clone(),
                &shader::VS_FULLSCREEN,
                &shader::FS_BLIT_TO_CANVAS,
            ),
            decompose_signal_shader: Shader::new(
                gl.clone(),
                &shader::VS_FULLSCREEN,
                &shader::FS_DECOMPOSE_SIGNAL,
            ),
            integrator_radiance_estimate: Texture::new(gl.clone()),
            integrator_gather_fbo: Framebuffer::new(gl.clone()),
            load_filter_tile_shader: Shader::new(
                gl.clone(),
                &shader::VS_FULLSCREEN,
                &shader::FS_LOAD_FILTER_TILE,
            ),
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
            fft_buffer_tile_r: Texture::new(gl.clone()),
            fft_buffer_tile_g: Texture::new(gl.clone()),
            fft_buffer_tile_b: Texture::new(gl.clone()),
            fft_signal_fbo: Framebuffer::new(gl.clone()),
            fft_filter_fbo: vec![],
            fft_buffer_fbo: Framebuffer::new(gl.clone()),
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
            execute_fft_pass_shader: Shader::new(
                gl.clone(),
                &shader::VS_EXECUTE_FFT_PASS,
                &shader::FS_EXECUTE_FFT_PASS,
            ),
            post_process_shader: Shader::new(
                gl.clone(),
                &shader::VS_FULLSCREEN,
                &shader::FS_POST_PROCESS,
            ),
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
            convolution_signal_fbo: Framebuffer::new(gl.clone()),
            convolution_output_fbo: Framebuffer::new(gl.clone()),
            integrator_scatter_fbo: Framebuffer::new(gl.clone()),
            device_lost: true,
            state: IntegratorState::default(),
            postproc: PostProcState::default(),
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

    /// Returns the texture compression family supported by this device.
    pub fn supported_texture_compression(&mut self) -> Result<TextureCompression, Error> {
        if let Some(texture_compression) = supported_texture_compression(&self.gl) {
            Ok(texture_compression)
        } else {
            Err(Error::new("no texture compression format supported"))
        }
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
        let mut reset_tiles = false;

        self.placeholder_texture.upload(1, 1, &[0]);
        self.placeholder_texture_array.upload_array(1, 1, &[&[0]]);

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

        let assets = &scene.assets;

        invalidated |= Dirty::clean(&mut scene.material_list, |materials| {
            self.update_materials(materials, assets)?;

            Dirty::dirty(instances);

            Ok(())
        })?;

        let geometry_list = &scene.geometry_list;
        let material_list = &scene.material_list;

        invalidated |= Dirty::clean(&mut scene.instance_list, |instances| {
            self.update_instances(geometry_list, material_list, instances)?;

            Ok(())
        })?;

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

            let render_cols = raster.width as usize;
            let render_rows = raster.height as usize;

            self.composited_render.create(render_cols, render_rows);
            self.composited_fbo
                .rebuild(&[&self.composited_render], None)?;

            self.convolution_signal.create(render_cols, render_rows);
            self.convolution_output.create(render_cols, render_rows);

            self.integrator_radiance_estimate
                .create(render_cols, render_rows);

            self.integrator_gather_fbo
                .rebuild(&[&self.integrator_radiance_estimate], None)?;

            self.convolution_output_fbo
                .rebuild(&[&self.convolution_output], None)?;

            self.convolution_signal_fbo
                .rebuild(&[&self.convolution_signal], None)?;

            Ok(())
        })?;

        // These two shaders need to be available
        self.execute_fft_pass_shader.rebuild()?;
        self.load_filter_tile_shader.rebuild()?;

        let assets = &scene.assets;

        reset_tiles |= Dirty::clean(&mut scene.aperture, |aperture| {
            self.fft_filter_fbo.clear();
            self.fft_filter_tile_r.clear();
            self.fft_filter_tile_g.clear();
            self.fft_filter_tile_b.clear();

            if let Some(aperture) = aperture {
                self.update_aperture_filter(aperture, assets)?;
            } else {
                self.fft_signal_fbo.invalidate();
                self.fft_buffer_fbo.invalidate();
                self.fft_signal_tile_r.reset();
                self.fft_signal_tile_g.reset();
                self.fft_signal_tile_b.reset();
                self.fft_buffer_tile_r.reset();
                self.fft_buffer_tile_g.reset();
                self.fft_buffer_tile_b.reset();
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

        reset_tiles |= Dirty::clean(&mut scene.display, |display| {
            self.update_display(display)?;

            Ok(())
        })?;

        self.post_process_shader.rebuild()?;
        self.blit_to_canvas_shader.rebuild()?;
        self.decompose_signal_shader.rebuild()?;
        self.load_signal_tile_shader.rebuild()?;
        self.read_signal_tile_shader.rebuild()?;

        self.integrator_scatter_photons_shader.rebuild()?;
        self.integrator_gather_photons_shader.rebuild()?;

        if invalidated {
            self.reset_integrator_state(scene);
        }

        if invalidated || reset_tiles {
            self.reset_convolution_state(scene);
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

        if self.postproc.display.lens_flare_enabled && self.state.current_pass >= 2 {
            let tile_size = self.current_tile_size();

            let filter_size = self.current_filter_size();

            for _ in 0..self.postproc.display.lens_flare_tiles_per_pass {
                let value = self.postproc.convolution_tiles.next().unwrap();

                if let Position::First(_) | Position::Only(_) = value {
                    self.convolution_output_fbo.clear(0, [0.0, 0.0, 0.0, 1.0]);
                    self.copy_radiance_estimate_to_convolution_signal();
                }

                let (signal_tile, (filter_index, filter_tile)) = value.into_inner();

                let dx = (filter_tile.x + filter_tile.w / 2) as i32 - filter_size as i32 / 2;
                let dy = (filter_tile.y + filter_tile.h / 2) as i32 - filter_size as i32 / 2;

                let padding = tile_size as i32 / 2;

                self.load_signal_tile(signal_tile);
                self.convolve_tile(filter_index);

                self.composite_tile(
                    signal_tile.x as i32 - padding + dx,
                    signal_tile.y as i32 - padding + dy,
                    signal_tile.w as i32 + padding * 2,
                    signal_tile.h as i32 + padding * 2,
                );

                if let Position::Last(_) | Position::Only(_) = value {
                    self.post_process(&self.convolution_output);
                    break; // skip immediately convolving again
                }
            }
        } else {
            self.post_process(&self.integrator_radiance_estimate);
        }

        Ok(())
    }

    // TODO: move this to somewhere else, maybe a post_processing.rs
    fn post_process(&self, texture: &dyn AsBindTarget) {
        let command = self.post_process_shader.begin_draw();

        command.bind(texture, "samples");

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
    }

    /// Presents the current render state into the context's canvas.
    pub fn present(&mut self) -> Result<(), Error> {
        if self.device_lost {
            return Ok(());
        }

        let command = self.blit_to_canvas_shader.begin_draw();

        command.bind(&self.composited_render, "render");

        command.set_canvas_framebuffer();

        command.set_viewport(
            0,
            0,
            self.composited_render.cols() as i32,
            self.composited_render.rows() as i32,
        );

        command.unset_vertex_array();
        command.draw_triangles(0, 1);

        Ok(())
    }

    fn try_restore(&mut self, scene: &mut Scene) -> Result<bool, Error> {
        if self.gl.is_context_lost() {
            return Ok(false);
        }

        for fbo in &mut self.fft_filter_fbo {
            fbo.invalidate();
        }

        for texture in &mut self.fft_filter_tile_r {
            texture.invalidate();
        }

        for texture in &mut self.fft_filter_tile_g {
            texture.invalidate();
        }

        for texture in &mut self.fft_filter_tile_b {
            texture.invalidate();
        }

        self.fft_signal_fbo.invalidate();
        self.fft_buffer_fbo.invalidate();
        self.fft_signal_tile_r.invalidate();
        self.fft_signal_tile_g.invalidate();
        self.fft_signal_tile_b.invalidate();
        self.fft_buffer_tile_r.invalidate();
        self.fft_buffer_tile_g.invalidate();
        self.fft_buffer_tile_b.invalidate();

        self.composited_render.invalidate();
        self.composited_fbo.invalidate();

        self.blit_to_canvas_shader.invalidate();

        self.material_textures.invalidate();

        self.placeholder_texture.invalidate();
        self.placeholder_texture_array.invalidate();

        self.load_signal_tile_shader.invalidate();
        self.load_filter_tile_shader.invalidate();
        self.read_signal_tile_shader.invalidate();
        self.decompose_signal_shader.invalidate();

        self.post_process_shader.invalidate();
        self.execute_fft_pass_shader.invalidate();
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
        self.convolution_signal.invalidate();
        self.signal_fft_passes.invalidate();
        self.filter_fft_passes.invalidate();
        self.convolution_output_fbo.invalidate();
        self.convolution_signal_fbo.invalidate();

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
