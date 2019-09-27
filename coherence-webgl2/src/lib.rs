#[allow(unused_imports)]
use log::{debug, info, warn};

mod shaders {
    include!(concat!(env!("OUT_DIR"), "/glsl_shaders.rs"));
}

#[macro_export]
macro_rules! export {
    [$( $module:ident ),*] => {
        $(
            mod $module;
            pub use self::$module::*;
        )*
    };
}

/// Types and definitions to model a scene to be ray-traced.
pub mod render {
    export![environment, object];
}

use coherence_base::device::*;
use coherence_base::{model::RasterFilter, Dirty, Scene};
use js_sys::Error;
use maplit::hashmap;
use quasirandom::Qrng;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use render::*;
use std::mem::size_of;
use web_sys::WebGl2RenderingContext as Context;
use zerocopy::{AsBytes, FromBytes};

mod engine;

pub use engine::*;

#[repr(align(64), C)]
#[derive(FromBytes, AsBytes, Clone)]
struct Aligned([u8; 64]);

impl Default for Aligned {
    fn default() -> Self {
        Self([0; 64])
    }
}

/// WebGL doesn't have buffer mapping so just allocate one big resident buffer
/// for all our needs. this is shared everywhere and is just used as an
/// intermediate buffer for large copy operations.
#[derive(Default)]
pub struct AlignedMemory {
    memory: Vec<Aligned>,
}

impl AlignedMemory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allocate_bytes(&mut self, size: usize) -> &mut [u8] {
        let blocks = (size + size_of::<Aligned>() - 1) / size_of::<Aligned>();
        self.memory.resize_with(blocks, Aligned::default); // preallocate data

        &mut self.memory.as_mut_slice().as_bytes_mut()[..size]
    }

    pub fn shrink_to_fit(&mut self) {
        self.memory.shrink_to_fit();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RefineStatistics {
    pub frame_time_us: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderStatistics {
    pub frame_time_us: f32,
}

pub struct Device {
    pub gl: Context,

    program: Shader,
    present_program: Shader,

    camera_buffer: UniformBuffer<CameraData>,

    geometry_buffer: UniformBuffer<[GeometryParameter]>,
    material_buffer: UniformBuffer<[MaterialParameter]>,
    instance_buffer: UniformBuffer<[SceneInstanceNode]>,

    envmap_cdf_tex: TextureBuffer<[EnvironmentMapCdfData]>,

    envmap_marginal_cdf: TextureImage<RG32F>,
    envmap_conditional_cdfs: TextureImage<RG32F>,

    envmap_texture: TextureImage<RGBA32F>,

    globals_buffer: UniformBuffer<GlobalData>,
    raster_buffer: UniformBuffer<RasterData>,

    samples: TextureImage<RGBA32F>,
    samples_fbo: Framebuffer,

    refine_query: Query,
    render_query: Query,

    scratch: AlignedMemory,

    device_lost: bool,

    state: DeviceState,
}

impl Device {
    /// Creates a new device using a WebGL2 context.
    pub fn new(gl: &Context) -> Result<Self, Error> {
        // hashmap! { "TBUF_WIDTH" => format!("{}", pixels_per_texture_buffer_row(&gl))
        // }

        Ok(Self {
            scratch: AlignedMemory::new(),
            gl: gl.clone(),
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
                    "envmap_cdf_tex" => BindingPoint::Texture(0),
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
                },
            ),
            camera_buffer: UniformBuffer::new(gl.clone()),
            geometry_buffer: UniformBuffer::new_array(gl.clone(), 64),
            material_buffer: UniformBuffer::new_array(gl.clone(), 256),
            // TODO: get these from the shader?? (not really easily doable I think)
            //  -> #define them in the shader from some shared value obtained from the WebGL
            // context!
            instance_buffer: UniformBuffer::new_array(gl.clone(), 256),
            raster_buffer: UniformBuffer::new(gl.clone()),
            globals_buffer: UniformBuffer::new(gl.clone()),
            envmap_cdf_tex: TextureBuffer::new(gl.clone(), TextureBufferFormat::RGBA32F),
            envmap_texture: TextureImage::new(gl.clone()),
            envmap_marginal_cdf: TextureImage::new(gl.clone()),
            envmap_conditional_cdfs: TextureImage::new(gl.clone()),
            samples_fbo: Framebuffer::new(gl.clone()),
            refine_query: Query::new(gl.clone()),
            render_query: Query::new(gl.clone()),
            samples: TextureImage::new(gl.clone()),
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
            self.camera_buffer.write(&mut self.scratch, camera);
        });

        invalidated |= Dirty::clean(&mut scene.objects, |objects| {
            let mut generator = GeometryGlslGenerator::new();

            let mut geometries = vec![];

            for geometry in &objects.list {
                geometries.push((
                    generator.add_distance_function(geometry),
                    generator.add_normal_function(geometry),
                ));
            }

            self.program
                .frag_shader()
                .set_header("geometry-user.glsl", generator.generate(&geometries));
        });

        let objects = &scene.objects;

        invalidated |= Dirty::clean(&mut scene.instances, |instances| {
            let instances = InstancesWithObjects {
                instances,
                objects: &objects.list,
            };

            self.instance_buffer
                .write_array(&mut self.scratch, &instances);

            self.geometry_buffer
                .write_array(&mut self.scratch, &instances);

            self.material_buffer
                .write_array(&mut self.scratch, &instances);
        });

        invalidated |= Dirty::clean(&mut scene.materials, |materials| {
            /*self.material_buffer
            .write_array(&mut self.scratch, materials);*/
        });

        invalidated |= Dirty::clean(&mut scene.environment, |environment| {
            // TODO: maybe avoid invalidating the shader if the define hasn't actually
            // changed... there is probably a nicer way to do this honestly

            self.update_environment(environment);

            if let Some(map) = &environment.map {
                self.program.frag_shader().set_define("HAS_ENVMAP", "1");

                self.envmap_cdf_tex.write(&mut self.scratch, map);
            // self.envmap_pix_tex.write(&mut self.scratch, map);
            } else {
                self.program.frag_shader().set_define("HAS_ENVMAP", "0");
            }
        });

        invalidated |= Dirty::clean(&mut scene.raster, |raster| {
            self.raster_buffer.write(&mut self.scratch, raster);

            self.samples
                .create(raster.width.get() as usize, raster.height.get() as usize);

            self.samples_fbo.invalidate(&[&self.samples]);
        });

        self.program.rebuild()?;
        self.present_program.rebuild()?;

        if invalidated {
            self.state.reset(scene);
            self.reset_refinement();
        }

        self.scratch.shrink_to_fit();

        Ok(invalidated)
    }

    /// Further refines the path-traced render buffer.
    pub fn refine(&mut self) -> Option<RefineStatistics> {
        if self.device_lost {
            return None;
        }

        // TODO: not happy with this, can we improve it
        self.state
            .update(&mut self.scratch, &mut self.globals_buffer);

        self.program.bind_to_pipeline(|shader| {
            shader.bind(&self.camera_buffer, "Camera");
            shader.bind(&self.geometry_buffer, "Geometry");
            shader.bind(&self.material_buffer, "Material");
            shader.bind(&self.instance_buffer, "Instance");
            shader.bind(&self.globals_buffer, "Globals");
            shader.bind(&self.raster_buffer, "Raster");
            shader.bind(&self.envmap_cdf_tex, "envmap_cdf_tex");
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

        self.present_program.bind_to_pipeline(|shader| {
            shader.bind(&self.samples, "samples");
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

    pub fn update(&mut self, scratch: &mut AlignedMemory, buffer: &mut UniformBuffer<GlobalData>) {
        // we don't want the first (0, 0) sample from the sequence
        let (mut x, mut y) = self.filter_rng.next::<(f32, f32)>();

        if x == 0.0 && y == 0.0 {
            x = 0.5;
            y = 0.5;
        }

        buffer.write_direct(scratch, |data| {
            data.filter_delta[0] = 2.0 * self.filter.importance_sample(x) - 1.0;
            data.filter_delta[1] = 2.0 * self.filter.importance_sample(y) - 1.0;
            data.frame_state[0] = self.rng.next_u32();
            data.frame_state[1] = self.rng.next_u32();
            data.frame_state[2] = self.frame;
        });

        self.frame += 1;

        // info!("frame = {}", self.frame);
    }
}

#[repr(C)]
#[derive(AsBytes, FromBytes)]
struct GlobalData {
    filter_delta: [f32; 4],
    frame_state: [u32; 4],
}
