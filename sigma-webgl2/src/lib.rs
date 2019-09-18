#[allow(unused_imports)]
use log::{debug, info, warn};

use js_sys::Error;
use maplit::hashmap;
use sigma_core::device::*;
use sigma_core::{
    model::{Aperture, RasterFilter},
    Dirty, Scene,
};
use std::mem::size_of;
use web_sys::{WebGl2RenderingContext as Context, WebGlFramebuffer, WebGlTexture};
use zerocopy::{AsBytes, FromBytes};

mod engine;

pub use engine::*;

// TODO: need to add format here somehow
pub struct RenderTexture {
    gl: Context,
    handle: Option<WebGlTexture>,
    width: i32,
    height: i32,
}

impl RenderTexture {
    pub fn new(gl: Context) -> Self {
        Self {
            gl,
            handle: None,
            width: 0,
            height: 0,
        }
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        if width != self.width || height != self.height || self.resource().is_none() {
            self.gl.delete_texture(self.resource());

            self.handle = self.gl.create_texture();

            self.gl.bind_texture(Context::TEXTURE_2D, self.resource());

            self.gl
                .tex_storage_2d(Context::TEXTURE_2D, 1, Context::RGBA32F, width, height);

            self.gl.tex_parameteri(
                Context::TEXTURE_2D,
                Context::TEXTURE_MAG_FILTER,
                Context::NEAREST as i32,
            );
            self.gl.tex_parameteri(
                Context::TEXTURE_2D,
                Context::TEXTURE_MIN_FILTER,
                Context::NEAREST as i32,
            );
            self.gl.tex_parameteri(
                Context::TEXTURE_2D,
                Context::TEXTURE_WRAP_S,
                Context::CLAMP_TO_EDGE as i32,
            );
            self.gl.tex_parameteri(
                Context::TEXTURE_2D,
                Context::TEXTURE_WRAP_T,
                Context::CLAMP_TO_EDGE as i32,
            );

            self.width = width;
            self.height = height;
        }
    }

    pub(crate) fn resource(&self) -> Option<&WebGlTexture> {
        self.handle.as_ref()
    }

    pub(crate) fn reset(&mut self) {
        self.handle = None;
        self.width = 0;
        self.height = 0;
    }
}

impl ShaderBind for RenderTexture {
    fn handle(&self) -> ShaderBindHandle {
        ShaderBindHandle::Texture(self.handle.as_ref())
    }
}

use std::collections::HashMap;

struct FramebufferCache {
    gl: Context,

    cache: HashMap<&'static str, Option<WebGlFramebuffer>>,
}

impl FramebufferCache {
    pub fn new(gl: Context) -> Self {
        Self {
            gl,
            cache: HashMap::new(),
        }
    }

    pub fn get_framebuffer(
        &mut self,
        name: &'static str,
        textures: &[&RenderTexture],
    ) -> Option<&WebGlFramebuffer> {
        let gl = &self.gl;

        self.cache
            .entry(name)
            .or_insert_with(|| {
                let fbo = gl.create_framebuffer();

                gl.bind_framebuffer(Context::DRAW_FRAMEBUFFER, fbo.as_ref());

                for (index, texture) in textures.iter().enumerate() {
                    gl.framebuffer_texture_2d(
                        Context::DRAW_FRAMEBUFFER,
                        Context::COLOR_ATTACHMENT0 + index as u32,
                        Context::TEXTURE_2D,
                        texture.resource(),
                        0,
                    );
                }

                fbo
            })
            .as_ref()
    }

    pub(crate) fn reset(&mut self) {
        self.cache.clear();
    }
}

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

use quasirandom::Qrng;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

pub struct Device {
    pub gl: Context,

    program: Shader,
    present_program: Shader,

    camera_buffer: UniformBuffer<CameraData>,

    instance_buffer: UniformBuffer<[InstanceData]>,
    instance_hierarchy_buffer: UniformBuffer<[SceneHierarchyNode]>,

    globals_buffer: UniformBuffer<GlobalData>,
    raster_buffer: UniformBuffer<RasterData>,

    bvh_tex: TextureBuffer<HierarchyData>,
    tri_tex: TextureBuffer<TriangleData>,

    position_tex: TextureBuffer<VertexPositionData>,
    normal_tex: TextureBuffer<VertexMappingData>,

    samples: RenderTexture,
    framebuffers: FramebufferCache,

    scratch: AlignedMemory,

    lost: bool,

    state: DeviceState,
}

impl Device {
    /// Creates a new device using a WebGL2 context.
    pub fn new(gl: Context) -> Result<Self, Error> {
        Ok(Self {
            scratch: AlignedMemory::new(),
            gl: gl.clone(),
            program: Shader::new(
                gl.clone(),
                include_str!("shaders/vert.glsl").to_string(),
                [
                    &format!("#define TBUF_WIDTH {}", pixels_per_texture_buffer_row(&gl)),
                    include_str!("shaders/random.glsl"),
                    include_str!("shaders/frag.glsl"),
                ]
                .join("\n"),
                hashmap! {
                    "bvh_data" => BindingPoint::Texture(0),
                    "tri_data" => BindingPoint::Texture(1),
                    "position_data" => BindingPoint::Texture(2),
                    "normal_data" => BindingPoint::Texture(3),
                    "Camera" => BindingPoint::UniformBlock(0),
                    "Instances" => BindingPoint::UniformBlock(1),
                    "InstanceHierarchy" => BindingPoint::UniformBlock(4),
                    "Globals" => BindingPoint::UniformBlock(2),
                    "Raster" => BindingPoint::UniformBlock(3),
                },
            ),
            present_program: Shader::new(
                gl.clone(),
                include_str!("shaders/vert.glsl").to_string(),
                include_str!("shaders/present.glsl").to_string(),
                hashmap! {
                    "samples" => BindingPoint::Texture(0),
                },
            ),
            camera_buffer: UniformBuffer::new(gl.clone()),
            bvh_tex: TextureBuffer::new(gl.clone(), TextureBufferFormat::F32x4),
            tri_tex: TextureBuffer::new(gl.clone(), TextureBufferFormat::U32x4),
            position_tex: TextureBuffer::new(gl.clone(), TextureBufferFormat::F32x4),
            normal_tex: TextureBuffer::new(gl.clone(), TextureBufferFormat::U32x4),
            // TODO: get these from the shader?? (not really easily doable I think)
            //  -> #define them in the shader from some shared value obtained from the WebGL
            // context!
            instance_buffer: UniformBuffer::new_array(gl.clone(), 128),
            instance_hierarchy_buffer: UniformBuffer::new_array(gl.clone(), 127),
            raster_buffer: UniformBuffer::new(gl.clone()),
            globals_buffer: UniformBuffer::new(gl.clone()),
            samples: RenderTexture::new(gl.clone()),
            framebuffers: FramebufferCache::new(gl.clone()),
            lost: true,
            state: DeviceState::new(),
        })
    }

    /// Signals the context was lost.
    pub fn context_lost(&mut self) {
        self.lost = true;
    }

    /// Updates this device to render some scene, or returns an error.
    pub fn update(&mut self, scene: &mut Scene) -> Result<(), Error> {
        if self.lost && self.try_restore(scene)? {
            return Ok(()); // context still lost
        }

        let mut invalidated = false;

        invalidated |= Dirty::clean(&mut scene.camera, |camera| {
            self.camera_buffer.write(&mut self.scratch, camera);
        });

        invalidated |= Dirty::clean(&mut scene.objects, |objects| {
            self.bvh_tex.write(&mut self.scratch, objects);
            self.tri_tex.write(&mut self.scratch, objects);
            self.position_tex.write(&mut self.scratch, objects);
            self.normal_tex.write(&mut self.scratch, objects);
        });

        let objects = &scene.objects;

        invalidated |= Dirty::clean(&mut scene.instances, |instances| {
            let instances = InstancesWithObjects {
                instances,
                objects: &objects.list,
            };

            self.instance_buffer
                .write_array(&mut self.scratch, &instances);

            self.instance_hierarchy_buffer
                .write_array(&mut self.scratch, &instances);
        });

        invalidated |= Dirty::clean(&mut scene.raster, |raster| {
            self.raster_buffer.write(&mut self.scratch, raster);

            self.samples
                .resize(raster.width.get() as i32, raster.height.get() as i32);

            self.framebuffers.reset();
        });

        if invalidated {
            self.state.reset(scene);
            self.reset_refinement();
        }

        self.scratch.shrink_to_fit();

        Ok(())
    }

    // TODO: return stats (e.g. measured with performance counters and so on)
    // (make this optional)

    /// Further refines the path-traced render buffer.
    pub fn refine(&mut self) {
        if self.lost {
            return;
        }

        self.gl
            .viewport(0, 0, self.samples.width, self.samples.height);

        let fbo = self
            .framebuffers
            .get_framebuffer("samples", &[&self.samples]);

        self.gl.bind_framebuffer(Context::DRAW_FRAMEBUFFER, fbo);

        // TODO: not happy with this, can we improve it
        self.state
            .update(&mut self.scratch, &mut self.globals_buffer);

        let shader = self.program.bind_to_pipeline();

        shader.bind(&self.camera_buffer, "Camera");
        shader.bind(&self.instance_buffer, "Instances");
        shader.bind(&self.instance_hierarchy_buffer, "InstanceHierarchy");
        shader.bind(&self.globals_buffer, "Globals");
        shader.bind(&self.raster_buffer, "Raster");
        shader.bind(&self.bvh_tex, "bvh_data");
        shader.bind(&self.tri_tex, "tri_data");
        shader.bind(&self.position_tex, "position_data");
        shader.bind(&self.normal_tex, "normal_data");

        self.gl.enable(Context::BLEND);
        self.gl.blend_equation(Context::FUNC_ADD);
        self.gl.blend_func(Context::ONE, Context::ONE);

        self.gl.bind_buffer(Context::ARRAY_BUFFER, None);
        self.gl.draw_arrays(Context::TRIANGLES, 0, 3);
    }

    /// Displays the current render buffer to the screen.
    pub fn render(&mut self) {
        if self.lost {
            return;
        }

        self.gl
            .viewport(0, 0, self.samples.width, self.samples.height);

        self.gl.bind_framebuffer(Context::DRAW_FRAMEBUFFER, None);

        let shader = self.present_program.bind_to_pipeline();

        shader.bind(&self.samples, "samples");

        self.gl.disable(Context::BLEND);

        self.gl.bind_buffer(Context::ARRAY_BUFFER, None);
        self.gl.draw_arrays(Context::TRIANGLES, 0, 3);
    }

    fn try_restore(&mut self, scene: &mut Scene) -> Result<bool, Error> {
        if self.gl.is_context_lost() {
            return Ok(true);
        }

        self.try_load_extension("EXT_color_buffer_float")?;

        self.program.reset()?;
        self.present_program.reset()?;
        self.camera_buffer.reset();
        self.bvh_tex.reset();
        self.tri_tex.reset();
        self.samples.reset();
        self.framebuffers.reset();
        self.instance_buffer.reset();
        self.instance_hierarchy_buffer.reset();
        self.globals_buffer.reset();
        self.raster_buffer.reset();
        self.position_tex.reset();
        self.normal_tex.reset();

        scene.dirty_all();
        self.lost = false;

        Ok(false)
    }

    fn reset_refinement(&mut self) {
        let fbo = self
            .framebuffers
            .get_framebuffer("samples", &[&self.samples]);

        self.gl.bind_framebuffer(Context::DRAW_FRAMEBUFFER, fbo);

        self.gl
            .clear_bufferfv_with_f32_array(Context::COLOR, 0, &[0.0, 0.0, 0.0, 0.0]);
    }

    fn try_load_extension(&self, name: &str) -> Result<(), Error> {
        if let Err(_) | Ok(None) = self.gl.get_extension(name) {
            Err(Error::new(&format!("missing extension '{}'", name)))
        } else {
            Ok(())
        }
    }
}

struct DeviceState {
    rng: ChaCha20Rng,
    filter_rng: Qrng,

    filter: RasterFilter,
    aperture: Aperture,

    frame: u32,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            rng: ChaCha20Rng::seed_from_u64(0),
            filter_rng: Qrng::new(0),
            filter: RasterFilter::default(),
            aperture: Aperture::default(),
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

        self.aperture = scene.camera.aperture;
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
    }
}

#[repr(C)]
#[derive(AsBytes, FromBytes)]
struct GlobalData {
    filter_delta: [f32; 4],
    frame_state: [u32; 4],
}
