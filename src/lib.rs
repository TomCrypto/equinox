#![deny(unsafe_code)]

#[allow(unused_imports)]
use log::{debug, info, warn};

macro_rules! export {
    [$( $module:ident ),* $(,)*] => {
        $(
            mod $module;
            pub use self::$module::*;
        )*
    };
}

use js_sys::Error;
use maplit::hashmap;
use quasirandom::Qrng;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use web_sys::WebGl2RenderingContext as Context;
use zerocopy::{AsBytes, FromBytes};

#[derive(Clone, Copy, Debug)]
pub struct RefineStatistics {
    pub frame_time_us: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderStatistics {
    pub frame_time_us: f32,
}

#[derive(Debug)]
pub struct Device {
    pub gl: Context, // TODO: shouldn't be publicly exposed

    program: Shader,
    present_program: Shader,

    copy_from_spectrum_shader: Shader,
    copy_into_spectrum_shader: Shader,
    fft_shader: Shader,
    pointwise_multiply_shader: Shader,
    draw_aperture_shader: Shader,
    direct_copy_shader: Shader,

    fft_buffer: UniformBuffer<FFTData>,

    camera_buffer: UniformBuffer<CameraData>,

    geometry_buffer: UniformBuffer<[GeometryParameter]>,
    material_buffer: UniformBuffer<[MaterialParameter]>,
    instance_buffer: UniformBuffer<[SceneInstanceNode]>,

    display_buffer: UniformBuffer<DisplayData>,

    envmap_marginal_cdf: Texture<RG32F>,
    envmap_conditional_cdfs: Texture<RG32F>,

    envmap_texture: Texture<RGBA32F>,

    globals_buffer: UniformBuffer<GlobalData>,
    raster_buffer: UniformBuffer<RasterData>,

    samples: Texture<RGBA32F>,
    samples_fbo: Framebuffer,

    // Complex-valued spectrums for each render channel
    rspectrum_temp1: Texture<RG32F>,
    gspectrum_temp1: Texture<RG32F>,
    bspectrum_temp1: Texture<RG32F>,
    rspectrum_temp2: Texture<RG32F>,
    gspectrum_temp2: Texture<RG32F>,
    bspectrum_temp2: Texture<RG32F>,

    conv_source: Texture<RGBA32F>,
    conv_fbo: Framebuffer,

    aperture_source: Texture<RG32F>,
    r_aperture_spectrum: Texture<RG32F>,
    g_aperture_spectrum: Texture<RG32F>,
    b_aperture_spectrum: Texture<RG32F>,

    // Final convolved render output (real-valued)
    render: Texture<RGBA32F>,

    spectrum_temp1_fbo: Framebuffer,
    spectrum_temp2_fbo: Framebuffer,
    render_fbo: Framebuffer,
    aperture_fbo: Framebuffer,
    aperture_source_fbo: Framebuffer,

    refine_query: Query,
    render_query: Query,

    allocator: Allocator,

    device_lost: bool,

    state: DeviceState,
}

impl Device {
    /// Creates a new device using a WebGL2 context.
    pub fn new(gl: &Context) -> Result<Self, Error> {
        Ok(Self {
            allocator: Allocator::new(),
            gl: gl.clone(),
            direct_copy_shader: Shader::new(
                gl.clone(),
                ShaderBuilder::new(shaders::VERT),
                ShaderBuilder::new(shaders::DIRECT_COPY),
                hashmap! {
                    "source" => BindingPoint::Texture(0),
                },
            ),
            draw_aperture_shader: Shader::new(
                gl.clone(),
                ShaderBuilder::new(shaders::VERT),
                ShaderBuilder::new(shaders::DRAW_APERTURE),
                hashmap! {},
            ),
            fft_shader: Shader::new(
                gl.clone(),
                ShaderBuilder::new(shaders::VERT),
                ShaderBuilder::new(shaders::FFT),
                hashmap! {
                    "r_spectrum_input" => BindingPoint::Texture(0),
                    "g_spectrum_input" => BindingPoint::Texture(1),
                    "b_spectrum_input" => BindingPoint::Texture(2),
                    "FFT" => BindingPoint::UniformBlock(0),
                },
            ),
            pointwise_multiply_shader: Shader::new(
                gl.clone(),
                ShaderBuilder::new(shaders::VERT),
                ShaderBuilder::new(shaders::POINTWISE_MULTIPLY),
                hashmap! {
                    "r_spectrum_input" => BindingPoint::Texture(0),
                    "g_spectrum_input" => BindingPoint::Texture(1),
                    "b_spectrum_input" => BindingPoint::Texture(2),
                    "r_aperture_input" => BindingPoint::Texture(3),
                    "g_aperture_input" => BindingPoint::Texture(4),
                    "b_aperture_input" => BindingPoint::Texture(5),
                },
            ),
            copy_from_spectrum_shader: Shader::new(
                gl.clone(),
                ShaderBuilder::new(shaders::VERT),
                ShaderBuilder::new(shaders::COPY_FROM_SPECTRUM),
                hashmap! {
                    "r_spectrum" => BindingPoint::Texture(0),
                    "g_spectrum" => BindingPoint::Texture(1),
                    "b_spectrum" => BindingPoint::Texture(2),
                    "add" => BindingPoint::Texture(3),
                    "subtract" => BindingPoint::Texture(4),
                },
            ),
            copy_into_spectrum_shader: Shader::new(
                gl.clone(),
                ShaderBuilder::new(shaders::VERT),
                ShaderBuilder::new(shaders::COPY_INTO_SPECTRUM),
                hashmap! {
                    "source" => BindingPoint::Texture(0),
                },
            ),
            program: Shader::new(
                gl.clone(),
                ShaderBuilder::new(shaders::VERT),
                ShaderBuilder::new(shaders::FRAG),
                hashmap! {
                    "Camera" => BindingPoint::UniformBlock(0),
                    "Instance" => BindingPoint::UniformBlock(4),
                    "Geometry" => BindingPoint::UniformBlock(7),
                    "Material" => BindingPoint::UniformBlock(8),
                    "Globals" => BindingPoint::UniformBlock(2),
                    "Raster" => BindingPoint::UniformBlock(3),
                    "envmap_pix_tex" => BindingPoint::Texture(1),
                    "envmap_marginal_cdf" => BindingPoint::Texture(2),
                    "envmap_conditional_cdfs" => BindingPoint::Texture(3),
                },
            ),
            present_program: Shader::new(
                gl.clone(),
                ShaderBuilder::new(shaders::VERT),
                ShaderBuilder::new(shaders::PRESENT),
                hashmap! {
                    "samples" => BindingPoint::Texture(0),
                    "Display" => BindingPoint::UniformBlock(0),
                },
            ),
            camera_buffer: UniformBuffer::new(gl.clone()),
            geometry_buffer: UniformBuffer::new_array(gl.clone(), 64),
            material_buffer: UniformBuffer::new_array(gl.clone(), 256),
            // TODO: get these from the shader?? (not really easily doable I think)
            //  -> #define them in the shader from some shared value obtained from the WebGL
            // context!
            instance_buffer: UniformBuffer::new_array(gl.clone(), 256),
            fft_buffer: UniformBuffer::new(gl.clone()),
            raster_buffer: UniformBuffer::new(gl.clone()),
            display_buffer: UniformBuffer::new(gl.clone()),
            globals_buffer: UniformBuffer::new(gl.clone()),
            envmap_texture: Texture::new(gl.clone()),
            envmap_marginal_cdf: Texture::new(gl.clone()),
            envmap_conditional_cdfs: Texture::new(gl.clone()),
            samples_fbo: Framebuffer::new(gl.clone()),
            rspectrum_temp1: Texture::new(gl.clone()),
            gspectrum_temp1: Texture::new(gl.clone()),
            bspectrum_temp1: Texture::new(gl.clone()),
            rspectrum_temp2: Texture::new(gl.clone()),
            gspectrum_temp2: Texture::new(gl.clone()),
            bspectrum_temp2: Texture::new(gl.clone()),
            aperture_source: Texture::new(gl.clone()),
            r_aperture_spectrum: Texture::new(gl.clone()),
            g_aperture_spectrum: Texture::new(gl.clone()),
            b_aperture_spectrum: Texture::new(gl.clone()),
            render: Texture::new(gl.clone()),
            render_fbo: Framebuffer::new(gl.clone()),
            aperture_fbo: Framebuffer::new(gl.clone()),
            aperture_source_fbo: Framebuffer::new(gl.clone()),
            spectrum_temp1_fbo: Framebuffer::new(gl.clone()),
            spectrum_temp2_fbo: Framebuffer::new(gl.clone()),
            conv_source: Texture::new(gl.clone()),
            conv_fbo: Framebuffer::new(gl.clone()),
            refine_query: Query::new(gl.clone()),
            render_query: Query::new(gl.clone()),
            samples: Texture::new(gl.clone()),
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
            self.update_camera(camera);

            self.r_aperture_spectrum.upload(
                camera.aperture_width as usize,
                camera.aperture_height as usize,
                &camera.aperture_r_spectrum,
            );

            self.g_aperture_spectrum.upload(
                camera.aperture_width as usize,
                camera.aperture_height as usize,
                &camera.aperture_g_spectrum,
            );

            self.b_aperture_spectrum.upload(
                camera.aperture_width as usize,
                camera.aperture_height as usize,
                &camera.aperture_b_spectrum,
            );
        });

        invalidated |= Dirty::clean(&mut scene.geometries, |geometries| {
            let mut generator = GeometryGlslGenerator::new();

            let mut geometry_functions = vec![];

            for geometry in &geometries.list {
                geometry_functions.push((
                    generator.add_distance_function(geometry),
                    generator.add_normal_function(geometry),
                ));
            }

            self.program.frag_shader().set_header(
                "geometry-user.glsl",
                generator.generate(&geometry_functions),
            );
        });

        let geometry_list = &scene.geometries.list;
        let material_list = &scene.materials.list;

        invalidated |= Dirty::clean(&mut scene.instances, |instances| {
            self.update_instances(&geometry_list, &material_list, instances);
        });

        invalidated |= Dirty::clean(&mut scene.materials, |materials| {
            self.update_materials(materials);
        });

        invalidated |= Dirty::clean(&mut scene.environment, |environment| {
            // TODO: maybe avoid invalidating the shader if the define hasn't actually
            // changed... there is probably a nicer way to do this honestly

            self.update_environment(environment);
        });

        invalidated |= Dirty::clean(&mut scene.raster, |raster| {
            self.update_raster(raster);

            self.samples
                .create(raster.width.get() as usize, raster.height.get() as usize);

            self.render
                .create(raster.width.get() as usize, raster.height.get() as usize);

            self.rspectrum_temp1.create(2048, 1024);
            self.gspectrum_temp1.create(2048, 1024);
            self.bspectrum_temp1.create(2048, 1024);

            self.rspectrum_temp2.create(2048, 1024);
            self.gspectrum_temp2.create(2048, 1024);
            self.bspectrum_temp2.create(2048, 1024);

            self.aperture_source.create(2048, 1024);
            self.r_aperture_spectrum.create(2048, 1024);
            self.g_aperture_spectrum.create(2048, 1024);
            self.b_aperture_spectrum.create(2048, 1024);

            self.conv_source.create(2048, 1024);

            self.samples_fbo.invalidate(&[&self.samples]);
            self.conv_fbo.invalidate(&[&self.conv_source]);

            self.aperture_source_fbo
                .invalidate(&[&self.aperture_source]);

            self.render_fbo.invalidate(&[&self.render]);
            self.aperture_fbo.invalidate(&[
                &self.r_aperture_spectrum,
                &self.g_aperture_spectrum,
                &self.b_aperture_spectrum,
            ]);

            self.spectrum_temp1_fbo.invalidate(&[
                &self.rspectrum_temp1,
                &self.gspectrum_temp1,
                &self.bspectrum_temp1,
            ]);

            self.spectrum_temp2_fbo.invalidate(&[
                &self.rspectrum_temp2,
                &self.gspectrum_temp2,
                &self.bspectrum_temp2,
            ]);

            // TODO: invalidate spectrum FBOs, resize etc...
            // for now let's just always do a 2048x1024 FFT
            // we'll simply resize the input render to always be < 1024 during
            // the spectrum construction phase
            // then the aperture will be < 1024 as well
            // then the final result will simply be composited directly on top
            // of the original data, stretched out with linear
            // filtering to provide extra detail cheaply...
        });

        // These are post-processing settings that don't apply to the path-traced light
        // transport simulation, so we don't need to invalidate the render buffer here.

        Dirty::clean(&mut scene.display, |display| {
            self.update_display(display);
        });

        self.program.rebuild()?;
        self.present_program.rebuild()?;

        self.copy_from_spectrum_shader.rebuild()?;
        self.copy_into_spectrum_shader.rebuild()?;
        self.fft_shader.rebuild()?;
        self.draw_aperture_shader.rebuild()?;
        self.pointwise_multiply_shader.rebuild()?;
        self.direct_copy_shader.rebuild()?;

        if invalidated {
            self.state.reset(scene);
            self.reset_refinement();
        }

        self.allocator.shrink_to_watermark();

        Ok(invalidated)
    }

    /// Further refines the path-traced render buffer.
    pub fn refine(&mut self) -> Option<RefineStatistics> {
        if self.device_lost {
            return None;
        }

        // TODO: not happy with this, can we improve it
        self.state
            .update(&mut self.allocator, &mut self.globals_buffer);

        self.program.bind_to_pipeline(|shader| {
            shader.bind(&self.camera_buffer, "Camera");
            shader.bind(&self.geometry_buffer, "Geometry");
            shader.bind(&self.material_buffer, "Material");
            shader.bind(&self.instance_buffer, "Instance");
            shader.bind(&self.globals_buffer, "Globals");
            shader.bind(&self.raster_buffer, "Raster");
            shader.bind(&self.envmap_texture, "envmap_pix_tex");
            shader.bind(&self.envmap_marginal_cdf, "envmap_marginal_cdf");
            shader.bind(&self.envmap_conditional_cdfs, "envmap_conditional_cdfs");
        });

        let weight = (self.state.frame as f32 - 1.0) / (self.state.frame as f32);

        let refine_query = self.refine_query.query_time_elapsed();

        self.samples_fbo.draw(DrawOptions {
            viewport: [0, 0, self.samples.cols() as i32, self.samples.rows() as i32],
            scissor: None,
            blend: Some(BlendMode::Accumulative { weight }),
        });

        if !Query::is_supported(&self.gl) {
            return None; // no statistics
        }

        Some(RefineStatistics {
            frame_time_us: refine_query.end().unwrap_or_default() / 1000.0,
        })
    }

    /// Displays the current render buffer to the screen.
    pub fn render(&mut self) -> Option<RenderStatistics> {
        if self.device_lost {
            return None;
        }

        self.render_lens_flare();

        self.present_program.bind_to_pipeline(|shader| {
            shader.bind(&self.render, "samples");
            shader.bind(&self.display_buffer, "Display");
        });

        let render_query = self.render_query.query_time_elapsed();

        Framebuffer::draw_to_canvas(
            &self.gl,
            DrawOptions {
                viewport: [0, 0, self.samples.cols() as i32, self.samples.rows() as i32],
                scissor: None,
                blend: None,
            },
        );

        if !Query::is_supported(&self.gl) {
            return None; // no statistics
        }

        Some(RenderStatistics {
            frame_time_us: render_query.end().unwrap_or_default() / 1000.0,
        })
    }

    fn try_restore(&mut self, scene: &mut Scene) -> Result<bool, Error> {
        if self.gl.is_context_lost() {
            return Ok(false);
        }

        // Unfortunately the isProgram call is extremely slow so we can't use it to
        // lazily check for context loss on the programs; manually invalidate them.

        self.program.invalidate();
        self.present_program.invalidate();
        self.copy_into_spectrum_shader.invalidate();
        self.copy_from_spectrum_shader.invalidate();
        self.fft_shader.invalidate();
        self.draw_aperture_shader.invalidate();
        self.pointwise_multiply_shader.invalidate();
        self.direct_copy_shader.invalidate();

        self.refine_query.reset();
        self.render_query.reset();

        scene.dirty_all_fields();
        self.device_lost = false;

        Ok(true)
    }

    fn reset_refinement(&mut self) {
        self.samples_fbo.clear(0, [0.0, 0.0, 0.0, 0.0]);
    }
}

#[derive(Debug)]
struct DeviceState {
    rng: ChaCha20Rng,
    filter_rng: Qrng,

    filter: RasterFilter,

    frame: u32,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            rng: ChaCha20Rng::seed_from_u64(0),
            filter_rng: Qrng::new(0),
            filter: RasterFilter::default(),
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

        self.filter = scene.raster.filter;
    }

    pub fn update(&mut self, allocator: &mut Allocator, buffer: &mut UniformBuffer<GlobalData>) {
        // we don't want the first (0, 0) sample from the sequence
        let (mut x, mut y) = self.filter_rng.next::<(f32, f32)>();

        if x == 0.0 && y == 0.0 {
            x = 0.5;
            y = 0.5;
        }

        let data: &mut GlobalData = allocator.allocate_one();

        data.filter_delta[0] = 2.0 * self.filter.importance_sample(x) - 1.0;
        data.filter_delta[1] = 2.0 * self.filter.importance_sample(y) - 1.0;
        data.frame_state[0] = self.rng.next_u32();
        data.frame_state[1] = self.rng.next_u32();
        data.frame_state[2] = self.frame;

        buffer.write(&data);

        self.frame += 1;
    }
}

#[repr(C)]
#[derive(AsBytes, FromBytes, Debug)]
struct GlobalData {
    filter_delta: [f32; 4],
    frame_state: [u32; 4],
}

use cgmath::prelude::*;
use cgmath::{Point3, Vector3};
use serde::{de::DeserializeOwned, Serialize};
use std::num::NonZeroU32;
use wasm_bindgen::prelude::*;
use web_sys::WebGl2RenderingContext;

#[wasm_bindgen]
pub struct WasmRunner {
    device: Device,
    scene: Scene,

    render_stats: Option<RenderStatistics>,
    refine_stats: Option<RefineStatistics>,
}

#[wasm_bindgen]
pub fn initialize_logging() {
    console_error_panic_hook::set_once();
    console_log::init().unwrap();
}

#[wasm_bindgen]
impl WasmRunner {
    #[wasm_bindgen(constructor)]
    pub fn new(context: &WebGl2RenderingContext) -> Result<WasmRunner, JsValue> {
        Ok(Self {
            device: Device::new(context)?,
            scene: Scene::default(),
            render_stats: None,
            refine_stats: None,
        })
    }

    pub fn scene_json(&self) -> Result<JsValue, JsValue> {
        Self::to_json(&self.scene)
    }

    pub fn set_camera_from_json(&mut self, json: &JsValue) -> Result<(), JsValue> {
        self.scene.camera = Self::from_json(json)?;

        Ok(())
    }

    pub fn set_materials_from_json(&mut self, json: &JsValue) -> Result<(), JsValue> {
        self.scene.materials = Self::from_json(json)?;

        Ok(())
    }

    pub fn context_lost(&mut self) {
        self.device.context_lost();
    }

    pub fn context(&self) -> WebGl2RenderingContext {
        self.device.gl.clone()
    }

    fn to_json<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
        Ok(JsValue::from_serde(value).map_err(|err| Error::new(&err.to_string()))?)
    }

    fn from_json<T: DeserializeOwned>(json: &JsValue) -> Result<T, JsValue> {
        Ok(json
            .into_serde()
            .map_err(|err| Error::new(&err.to_string()))?)
    }

    pub fn update(&mut self) -> Result<bool, JsValue> {
        Ok(self.device.update(&mut self.scene)?)
    }

    pub fn refine(&mut self) {
        self.refine_stats = self.device.refine();
    }

    pub fn render(&mut self) {
        self.render_stats = self.device.render();
    }

    pub fn get_refine_frame_time(&self) -> Option<f32> {
        Some(self.refine_stats?.frame_time_us)
    }

    pub fn get_render_frame_time(&self) -> Option<f32> {
        Some(self.render_stats?.frame_time_us)
    }

    pub fn move_camera(&mut self, forward: f32, sideways: f32) {
        let sideways_vector = self
            .scene
            .camera
            .up_vector
            .cross(self.scene.camera.direction)
            .normalize();

        let direction = self.scene.camera.direction;

        self.scene.camera.position += forward * direction + sideways_vector * sideways;
    }

    pub fn set_camera_position(&mut self, x: f32, y: f32, z: f32) {
        self.scene.camera.position = Point3::new(x, y, z);
    }

    pub fn set_camera_direction(&mut self, x: f32, y: f32, z: f32) {
        self.scene.camera.direction = Vector3::new(x, y, z);
    }

    pub fn set_camera_aperture(&mut self, radius: f32) {
        self.scene.camera.aperture = Aperture::Circle { radius };
    }

    pub fn set_aperture_data(&mut self, r: &[f32], g: &[f32], b: &[f32], width: u32, height: u32) {
        self.scene.camera.aperture_width = width;
        self.scene.camera.aperture_height = height;
        self.scene.camera.aperture_r_spectrum = Alias::new("r", r.to_vec());
        self.scene.camera.aperture_g_spectrum = Alias::new("g", g.to_vec());
        self.scene.camera.aperture_b_spectrum = Alias::new("b", b.to_vec());
    }

    pub fn set_envmap(&mut self, data: &[f32], cols: usize, rows: usize) {
        self.scene.environment.multiplier = [1.0, 1.0, 1.0];

        self.scene.environment.map = Some(EnvironmentMap {
            pixels: Alias::new("envmap", data.to_vec()),
            width: cols as u32,
            height: rows as u32,
        });
    }

    pub fn setup_test_scene(&mut self) {
        self.scene.geometries.list.push(Geometry::Plane {
            width: Parameter::Constant { value: 10.0 },
            length: Parameter::Constant { value: 4.0 },
        });

        self.scene.geometries.list.push(Geometry::Translate {
            f: Box::new(Geometry::UnitSphere),
            translation: [
                Parameter::Constant { value: 0.0 },
                Parameter::Constant { value: 1.01 },
                Parameter::Constant { value: 0.0 },
            ],
        });

        self.scene.geometries.list.push(Geometry::Union {
            children: vec![
                Geometry::Translate {
                    f: Box::new(Geometry::Scale {
                        f: Box::new(Geometry::UnitSphere),
                        factor: Parameter::Constant { value: 0.333 },
                    }),
                    translation: [
                        Parameter::Constant { value: 2.0 },
                        Parameter::Constant { value: 0.0 },
                        Parameter::Constant { value: 0.2 },
                    ],
                },
                Geometry::Translate {
                    f: Box::new(Geometry::Scale {
                        f: Box::new(Geometry::UnitSphere),
                        factor: Parameter::Constant { value: 0.333 },
                    }),
                    translation: [
                        Parameter::Constant { value: 2.0 },
                        Parameter::Constant { value: 0.0 },
                        Parameter::Constant { value: -0.2 },
                    ],
                },
            ],
        });

        // white lambertian
        self.scene.materials.list.push(Material::Lambertian {
            albedo: [0.9, 0.9, 0.9],
        });

        self.scene.materials.list.push(Material::Lambertian {
            albedo: [0.25, 0.25, 0.75],
        });

        self.scene.materials.list.push(Material::Phong {
            albedo: [0.9, 0.9, 0.9],
            shininess: 1024.0,
        });

        /*self.scene.instances.list.push(Instance {
            geometry: 0,
            material: 0,
            geometry_values: vec![],
        });*/

        /*self.scene.instances.list.push(Instance {
            geometry: 1,
            material: 1,
            geometry_values: vec![],
            material_values: vec![0.8, 0.8, 0.8, 0.0],
        });*/

        self.scene.instances.list.push(Instance {
            geometry: 1,
            material: 2,
            geometry_values: vec![],
        });

        /*self.scene.instances.list.push(Instance {
            geometry: 1,
            material: 3,
            geometry_values: vec![],
            material_values: vec![0.8, 0.8, 0.8, 1.55],
        });*/

        /*self.scene.instances.list.push(Instance {
            geometry: 2,
            material: 1,
            geometry_values: vec![],
        });*/
    }

    pub fn set_display_exposure(&mut self, exposure: f32) {
        self.scene.display.exposure = exposure;
    }

    pub fn set_display_saturation(&mut self, saturation: f32) {
        self.scene.display.saturation = saturation;
    }

    pub fn set_camera_response(&mut self, value: &str) {
        if value == "none" {
            self.scene.display.camera_response = None;
        } else {
            match value.parse::<u32>().unwrap() {
                0 => self.scene.display.camera_response = Some(AGFA_AGFACOLOR_HDC_100_PLUS),
                1 => self.scene.display.camera_response = Some(AGFA_ADVANTIX_100),
                2 => self.scene.display.camera_response = Some(AGFA_AGFACOLOR_FUTURA_100),
                3 => self.scene.display.camera_response = Some(AGFA_AGFACOLOR_FUTURA_II_100),
                4 => self.scene.display.camera_response = Some(AGFA_AGFACHROME_CT_PRECISA_100),
                5 => self.scene.display.camera_response = Some(AGFA_AGFACHROME_RSX2_050),
                6 => self.scene.display.camera_response = Some(CANON_OPTURA_981111),
                7 => self.scene.display.camera_response = Some(KODAK_DSCS_3151),
                8 => self.scene.display.camera_response = Some(KODAK_EKTACHROME_64T),
                9 => self.scene.display.camera_response = Some(KODAK_EKTACHROME_64),
                10 => self.scene.display.camera_response = Some(KODAK_MAX_ZOOM_800),
                11 => self.scene.display.camera_response = Some(KODAK_PORTRA_100T),
                12 => self.scene.display.camera_response = Some(FUJIFILM_FCI),
                13 => self.scene.display.camera_response = Some(AGFA_AGFACOLOR_VISTA_100),
                _ => unreachable!(),
            }
        }
    }

    pub fn add_other_object(&mut self) -> usize {
        // elongated cube

        self.scene.geometries.list.push(Geometry::Translate {
            translation: [
                Parameter::Constant { value: 1.5 },
                Parameter::Constant { value: 0.0 },
                Parameter::Constant { value: 0.0 },
            ],
            f: Box::new(Geometry::Scale {
                factor: Parameter::Constant { value: 0.5 },
                f: Box::new(Geometry::UnitCube),
            }),
        });

        self.scene.geometries.list.len() - 1
    }

    pub fn add_object(&mut self) -> usize {
        // for now, just add a sphere
        self.scene.geometries.list.push(Geometry::Union {
            children: vec![
                Geometry::Translate {
                    translation: [
                        Parameter::Symbolic { index: 1 },
                        Parameter::Symbolic { index: 2 },
                        Parameter::Symbolic { index: 3 },
                    ],
                    f: Box::new(Geometry::Scale {
                        factor: Parameter::Symbolic { index: 0 },
                        f: Box::new(Geometry::Round {
                            f: Box::new(Geometry::UnitCube),
                            radius: Parameter::Constant { value: 0.125 },
                        }),
                    }),
                },
                Geometry::Translate {
                    translation: [
                        Parameter::Symbolic { index: 5 },
                        Parameter::Symbolic { index: 6 },
                        Parameter::Symbolic { index: 7 },
                    ],
                    f: Box::new(Geometry::Scale {
                        factor: Parameter::Symbolic { index: 4 },
                        f: Box::new(Geometry::UnitSphere),
                    }),
                },
            ],
        });

        self.scene.geometries.list.len() - 1
    }

    pub fn set_dimensions(&mut self, width: u32, height: u32) {
        self.scene.raster.width = NonZeroU32::new(width).unwrap();
        self.scene.raster.height = NonZeroU32::new(height).unwrap();
        self.scene.raster.filter = RasterFilter::BlackmanHarris;
    }

    pub fn set_focal_distance(&mut self, value: f32) {
        self.scene.camera.focal_distance = value;
    }

    pub fn set_focal_length(&mut self, value: f32) {
        self.scene.camera.focal_length = value;
    }

    pub fn instance_count(&mut self) -> usize {
        self.scene.instances.list.len()
    }

    pub fn remove_instance(&mut self, index: usize) {
        self.scene.instances.list.remove(index);
    }
}

export![device, engine, scene];

/// GLSL shaders.
pub mod shaders {
    include!(concat!(env!("OUT_DIR"), "/glsl_shaders.rs"));
}
