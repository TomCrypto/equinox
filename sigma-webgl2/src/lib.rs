use js_sys::{Error, Float32Array};
use log::info;
use maplit::hashmap;
use sigma_core::{DeviceBuffer, Dirty, Scene};
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::{WebGl2RenderingContext as Context, WebGlBuffer, WebGlFramebuffer, WebGlTexture};
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

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

/// WebGL doesn't have buffer mapping so just allocate one big resident buffer
/// for all our needs. this is shared everywhere and is just used as an
/// intermediate buffer for large copy operations.
pub struct ScratchMemory {
    memory: Vec<Aligned>,
}

impl ScratchMemory {
    pub fn new() -> Self {
        Self { memory: vec![] }
    }

    pub fn access_with_size(&mut self, size: usize) -> &mut [u8] {
        self.memory.resize(
            (size + std::mem::size_of::<Aligned>() - 1) / std::mem::size_of::<Aligned>(),
            Aligned([0; 64]),
        );

        &mut self.memory.as_mut_slice().as_bytes_mut()[..size]
    }
}

pub struct TextureBuffer {
    gl: Context,
    scratch: Rc<RefCell<ScratchMemory>>,

    handle: Option<WebGlTexture>,
    width: i32,
    height: i32,
}

impl TextureBuffer {
    pub fn new(gl: Context, scratch: Rc<RefCell<ScratchMemory>>) -> Self {
        Self {
            gl,
            scratch,
            handle: None,
            width: 0,
            height: 0,
        }
    }

    pub fn pixel_count(&self) -> i32 {
        self.width * self.height
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

impl DeviceBuffer for TextureBuffer {
    fn map_update(&mut self, size: usize, f: impl FnOnce(&mut [u8])) {
        let pixel_count = (size / (4 * 4)) as i32;

        let width = 4096;
        // make sure we always have a valid texture
        let height = ((pixel_count + 4095) / 4096).max(1);

        let mut memory = self.scratch.borrow_mut();

        let buffer = memory.access_with_size(size);

        f(buffer);

        // if the new height is > than the current height, we need to reallocate =
        // recreate
        if height > self.height {
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

        self.gl.bind_texture(Context::TEXTURE_2D, self.resource());

        let float_data: LayoutVerified<_, [f32]> = LayoutVerified::new_slice(buffer).unwrap();

        let mut float_offset: i32 = 0;

        for y in 0..height {
            let float_end = (float_offset + 4 * 4096).min(float_data.len() as i32);
            let pixels = (float_end - float_offset) / 4; // 4 floats per pixel

            let slice = &float_data[float_offset as usize..float_end as usize];

            // safety: we only use this for the next call and that's it
            let typed_array = unsafe { Float32Array::view(slice) };

            self.gl
                .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                    Context::TEXTURE_2D,
                    0,
                    0,
                    y,
                    pixels,
                    1,
                    Context::RGBA,
                    Context::FLOAT,
                    Some(&typed_array),
                )
                .unwrap();

            float_offset = float_end;
        }
    }
}

pub struct UniformBuffer {
    gl: Context,
    scratch: Rc<RefCell<ScratchMemory>>,

    handle: Option<WebGlBuffer>,
    size: usize,
}

impl UniformBuffer {
    pub fn new(gl: Context, scratch: Rc<RefCell<ScratchMemory>>) -> Self {
        Self {
            gl,
            scratch,
            handle: None,
            size: 0,
        }
    }

    pub(crate) fn resource(&self) -> Option<&WebGlBuffer> {
        self.handle.as_ref()
    }

    pub(crate) fn reset(&mut self) {
        self.handle = self.gl.create_buffer();
        self.size = 0;
    }
}

impl DeviceBuffer for UniformBuffer {
    fn map_update(&mut self, size: usize, f: impl FnOnce(&mut [u8])) {
        let mut memory = self.scratch.borrow_mut();

        let buffer = memory.access_with_size(size);

        f(buffer);

        if self.size < size {
            self.gl
                .bind_buffer(Context::UNIFORM_BUFFER, self.resource());
            self.gl.buffer_data_with_i32(
                Context::UNIFORM_BUFFER,
                size as i32,
                Context::DYNAMIC_DRAW,
            );

            self.size = size;
        }

        self.gl
            .bind_buffer(Context::UNIFORM_BUFFER, self.resource());
        self.gl
            .buffer_sub_data_with_i32_and_u8_array(Context::UNIFORM_BUFFER, 0, buffer);
    }
}

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

pub struct Device {
    pub gl: Context,

    program: Shader,
    present_program: Shader,

    camera_buffer: UniformBuffer,

    instance_buffer: UniformBuffer,

    bvh_tex: TextureBuffer,
    tri_tex: TextureBuffer,

    samples: RenderTexture,
    framebuffers: FramebufferCache,

    lost: bool,

    rng: ChaCha20Rng,
}

impl Device {
    /// Creates a new device using a WebGL2 context.
    pub fn new(gl: Context) -> Result<Self, Error> {
        let scratch = Rc::new(RefCell::new(ScratchMemory::new()));

        Ok(Self {
            gl: gl.clone(),
            program: Shader::new(
                gl.clone(),
                include_str!("shaders/vert.glsl"),
                include_str!("shaders/frag.glsl"),
                hashmap! {
                    "bvh_data" => BindingPoint::Texture(0),
                    "tri_data" => BindingPoint::Texture(1),
                    "Camera" => BindingPoint::UniformBlock(0),
                    "Instances" => BindingPoint::UniformBlock(1),
                },
            ),
            present_program: Shader::new(
                gl.clone(),
                include_str!("shaders/vert.glsl"),
                include_str!("shaders/present.glsl"),
                hashmap! {
                    "samples" => BindingPoint::Texture(0),
                },
            ),
            camera_buffer: UniformBuffer::new(gl.clone(), scratch.clone()),
            bvh_tex: TextureBuffer::new(gl.clone(), scratch.clone()),
            tri_tex: TextureBuffer::new(gl.clone(), scratch.clone()),
            instance_buffer: UniformBuffer::new(gl.clone(), scratch.clone()),
            samples: RenderTexture::new(gl.clone()),
            framebuffers: FramebufferCache::new(gl.clone()),
            lost: true,
            rng: ChaCha20Rng::seed_from_u64(0),
        })
    }

    /// Signals the context was lost.
    pub fn context_lost(&mut self) {
        self.lost = true;
    }

    /// Updates this device to render a scene.
    ///
    /// Returns an error if a graphics error occurs.
    pub fn update(&mut self, scene: &mut Scene) -> Result<(), Error> {
        if self.lost && self.try_restore(scene)? {
            return Ok(()); // context still lost
        }

        let mut invalidated = false;

        invalidated |= Dirty::clean(&mut scene.camera, |camera| {
            camera.update(&mut self.camera_buffer)
        });

        invalidated |= Dirty::clean(&mut scene.objects, |objects| {
            objects.update_hierarchy(&mut self.bvh_tex);
            objects.update_triangles(&mut self.tri_tex);
        });

        let objects = &scene.objects;

        invalidated |= Dirty::clean(&mut scene.instances, |instances| {
            instances.update(objects, &mut self.instance_buffer);

            // TODO: need a solution for this?
            // (might be able to derive it from the shader eventually, but this will do for
            // now)

            let shader = self.program.bind_to_pipeline();

            shader.set_uniform(instances.list.len() as u32, "instance_count");
        });

        invalidated |= Dirty::clean(&mut scene.frame, |frame| {
            self.samples
                .resize(frame.width.get() as i32, frame.height.get() as i32);
        });

        if invalidated {
            self.rng = ChaCha20Rng::seed_from_u64(scene.frame.seed);
            self.clear(); // also clear all the path-traced buffers
        }

        Ok(())
    }

    // TODO: return stats (e.g. measured with performance counters and so on)
    // (make this optional)

    /// Further refines the path-traced render buffer.
    pub fn refine(&mut self) {
        if self.lost {
            return;
        }

        let fbo = self
            .framebuffers
            .get_framebuffer("samples", &[&self.samples]);

        self.gl.bind_framebuffer(Context::DRAW_FRAMEBUFFER, fbo);

        let shader = self.program.bind_to_pipeline();

        shader.bind_uniform_buffer(&self.camera_buffer, "Camera");
        shader.bind_uniform_buffer(&self.instance_buffer, "Instances");
        shader.bind_texture_buffer(&self.bvh_tex, "bvh_data");
        shader.bind_texture_buffer(&self.tri_tex, "tri_data");

        // TODO: should be a seed in some uniform buffer somewhere..

        self.gl.uniform1ui(
            self.gl
                .get_uniform_location(self.program.resource().unwrap(), "seed")
                .as_ref(),
            self.rng.next_u32(),
        );

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

        self.gl.bind_framebuffer(Context::DRAW_FRAMEBUFFER, None);

        let shader = self.present_program.bind_to_pipeline();

        shader.bind_render_texture(&self.samples, "samples");

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

        scene.dirty_all();
        self.lost = false;

        Ok(false)
    }

    fn clear(&mut self) {
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
