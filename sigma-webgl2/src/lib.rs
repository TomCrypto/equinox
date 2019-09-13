use js_sys::Float32Array;
use sigma_core::{DeviceBuffer, Dirty, Scene};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{
    WebGl2RenderingContext, WebGlBuffer, WebGlFramebuffer, WebGlProgram, WebGlShader, WebGlTexture,
};
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

/*

the renderer is built as a series of "passes" that operate over disjoint output buffers... all
buffers are the same size and depend on the window size, and are resized automatically.

a "render pass" is then essentially one shader

we can generically say that each render pass operates over all available buffers
and has its own set of configuration options. it has access to all shared data

*/

pub struct Shader {
    context: WebGl2RenderingContext,
    handle: Option<WebGlProgram>,
    vertex: &'static str,
    fragment: &'static str,
}

impl Shader {
    pub fn new(
        context: WebGl2RenderingContext,
        vertex: &'static str,
        fragment: &'static str,
    ) -> Self {
        Self {
            context,
            handle: None,
            vertex,
            fragment,
        }
    }

    fn build_program(
        context: &WebGl2RenderingContext,
        vertex: &'static str,
        fragment: &'static str,
    ) -> Option<WebGlProgram> {
        let vert_shader =
            Self::compile_shader(context, WebGl2RenderingContext::VERTEX_SHADER, vertex)?;

        let frag_shader =
            Self::compile_shader(context, WebGl2RenderingContext::FRAGMENT_SHADER, fragment)?;

        Self::link_program(context, &vert_shader, &frag_shader)
    }

    fn compile_shader(
        context: &WebGl2RenderingContext,
        shader_type: u32,
        source: &str,
    ) -> Option<WebGlShader> {
        let shader = context.create_shader(shader_type);

        if let Some(shader) = &shader {
            context.shader_source(shader, source);
            context.compile_shader(shader);

            if !context
                .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
                .as_bool()
                .unwrap_or(false)
            {
                error(&format!(
                    "shader build error: {}",
                    context
                        .get_shader_info_log(&shader)
                        .unwrap_or_else(|| String::from("unknown"))
                ));

                panic!("fail");
            }
        }

        shader
    }

    fn link_program(
        context: &WebGl2RenderingContext,
        vert_shader: &WebGlShader,
        frag_shader: &WebGlShader,
    ) -> Option<WebGlProgram> {
        let program = context.create_program();

        if let Some(program) = &program {
            context.attach_shader(program, vert_shader);
            context.attach_shader(program, frag_shader);
            context.link_program(program);

            if !context
                .get_program_parameter(program, WebGl2RenderingContext::LINK_STATUS)
                .as_bool()
                .unwrap_or(false)
            {
                panic!(
                    "shader build error: {}",
                    context
                        .get_program_info_log(&program)
                        .unwrap_or_else(|| String::from("unknown"))
                );
            }
        }

        program
    }

    pub(crate) fn resource(&self) -> Option<&WebGlProgram> {
        self.handle.as_ref()
    }

    pub(crate) fn reset(&mut self) {
        self.handle = Self::build_program(&self.context, self.vertex, self.fragment);
    }
}

// TODO: need to add format here somehow
struct RenderTexture {
    context: WebGl2RenderingContext,
    handle: Option<WebGlTexture>,
    width: i32,
    height: i32,
}

impl RenderTexture {
    pub fn new(context: WebGl2RenderingContext) -> Self {
        Self {
            context,
            handle: None,
            width: 0,
            height: 0,
        }
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        if width != self.width || height != self.height || self.resource().is_none() {
            self.handle = self.context.create_texture();

            self.context
                .bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.resource());

            self.context.tex_storage_2d(
                WebGl2RenderingContext::TEXTURE_2D,
                1,
                WebGl2RenderingContext::RGBA32F,
                width,
                height,
            );

            self.context.tex_parameteri(
                WebGl2RenderingContext::TEXTURE_2D,
                WebGl2RenderingContext::TEXTURE_MAG_FILTER,
                WebGl2RenderingContext::NEAREST as i32,
            );
            self.context.tex_parameteri(
                WebGl2RenderingContext::TEXTURE_2D,
                WebGl2RenderingContext::TEXTURE_MIN_FILTER,
                WebGl2RenderingContext::NEAREST as i32,
            );
            self.context.tex_parameteri(
                WebGl2RenderingContext::TEXTURE_2D,
                WebGl2RenderingContext::TEXTURE_WRAP_S,
                WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
            );
            self.context.tex_parameteri(
                WebGl2RenderingContext::TEXTURE_2D,
                WebGl2RenderingContext::TEXTURE_WRAP_T,
                WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
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
    context: WebGl2RenderingContext,

    cache: HashMap<&'static str, Option<WebGlFramebuffer>>,
}

impl FramebufferCache {
    pub fn new(context: WebGl2RenderingContext) -> Self {
        Self {
            context,
            cache: HashMap::new(),
        }
    }

    pub fn get_framebuffer(
        &mut self,
        name: &'static str,
        textures: &[&RenderTexture],
    ) -> Option<&WebGlFramebuffer> {
        let context = &self.context;

        self.cache
            .entry(name)
            .or_insert_with(|| {
                let fbo = context.create_framebuffer();

                context.bind_framebuffer(WebGl2RenderingContext::DRAW_FRAMEBUFFER, fbo.as_ref());

                for (index, texture) in textures.iter().enumerate() {
                    context.framebuffer_texture_2d(
                        WebGl2RenderingContext::DRAW_FRAMEBUFFER,
                        WebGl2RenderingContext::COLOR_ATTACHMENT0 + index as u32,
                        WebGl2RenderingContext::TEXTURE_2D,
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
pub struct MapMemory {
    memory: Vec<Aligned>,
}

impl MapMemory {
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
    context: WebGl2RenderingContext,
    scratch: Rc<RefCell<MapMemory>>,

    handle: Option<WebGlTexture>,
    width: i32,
    height: i32,
}

impl TextureBuffer {
    pub fn new(context: WebGl2RenderingContext, scratch: Rc<RefCell<MapMemory>>) -> Self {
        Self {
            context,
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
            self.context.delete_texture(self.resource());

            self.handle = self.context.create_texture();

            self.context
                .bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.resource());

            self.context.tex_storage_2d(
                WebGl2RenderingContext::TEXTURE_2D,
                1,
                WebGl2RenderingContext::RGBA32F,
                width,
                height,
            );

            self.context.tex_parameteri(
                WebGl2RenderingContext::TEXTURE_2D,
                WebGl2RenderingContext::TEXTURE_MAG_FILTER,
                WebGl2RenderingContext::NEAREST as i32,
            );
            self.context.tex_parameteri(
                WebGl2RenderingContext::TEXTURE_2D,
                WebGl2RenderingContext::TEXTURE_MIN_FILTER,
                WebGl2RenderingContext::NEAREST as i32,
            );
            self.context.tex_parameteri(
                WebGl2RenderingContext::TEXTURE_2D,
                WebGl2RenderingContext::TEXTURE_WRAP_S,
                WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
            );
            self.context.tex_parameteri(
                WebGl2RenderingContext::TEXTURE_2D,
                WebGl2RenderingContext::TEXTURE_WRAP_T,
                WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
            );

            self.width = width;
            self.height = height;
        }

        self.context
            .bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.resource());

        let float_data: LayoutVerified<_, [f32]> = LayoutVerified::new_slice(buffer).unwrap();

        let mut float_offset: i32 = 0;

        for y in 0..height {
            let float_end = (float_offset + 4 * 4096).min(float_data.len() as i32);
            let pixels = (float_end - float_offset) / 4; // 4 floats per pixel

            let slice = &float_data[float_offset as usize..float_end as usize];

            // safety: we only use this for the next call and that's it
            let typed_array = unsafe { Float32Array::view(slice) };

            self.context
                .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                    WebGl2RenderingContext::TEXTURE_2D,
                    0,
                    0,
                    y,
                    pixels,
                    1,
                    WebGl2RenderingContext::RGBA,
                    WebGl2RenderingContext::FLOAT,
                    Some(&typed_array),
                )
                .unwrap();

            float_offset = float_end;
        }
    }
}

pub struct UniformBuffer {
    context: WebGl2RenderingContext,
    scratch: Rc<RefCell<MapMemory>>,

    handle: Option<WebGlBuffer>,
    size: usize,
}

impl UniformBuffer {
    pub fn new(context: WebGl2RenderingContext, scratch: Rc<RefCell<MapMemory>>) -> Self {
        Self {
            context,
            scratch,
            handle: None,
            size: 0,
        }
    }

    pub(crate) fn resource(&self) -> Option<&WebGlBuffer> {
        self.handle.as_ref()
    }

    pub(crate) fn reset(&mut self) {
        self.handle = self.context.create_buffer();
        self.size = 0;
    }
}

impl DeviceBuffer for UniformBuffer {
    fn map_update(&mut self, size: usize, f: impl FnOnce(&mut [u8])) {
        let mut memory = self.scratch.borrow_mut();

        let buffer = memory.access_with_size(size);

        f(buffer);

        if self.size < size {
            self.context
                .bind_buffer(WebGl2RenderingContext::UNIFORM_BUFFER, self.resource());
            self.context.buffer_data_with_i32(
                WebGl2RenderingContext::UNIFORM_BUFFER,
                size as i32,
                WebGl2RenderingContext::DYNAMIC_DRAW,
            );

            self.size = size;
        }

        self.context
            .bind_buffer(WebGl2RenderingContext::UNIFORM_BUFFER, self.resource());
        self.context.buffer_sub_data_with_i32_and_u8_array(
            WebGl2RenderingContext::UNIFORM_BUFFER,
            0,
            buffer,
        );
    }
}

pub struct Device {
    pub context: WebGl2RenderingContext,

    program: Shader,
    present_program: Shader,

    camera_buffer: UniformBuffer,

    bvh_tex: TextureBuffer,
    tri_tex: TextureBuffer,

    samples: RenderTexture,
    framebuffers: FramebufferCache,

    lost: bool,
}

impl Device {
    /// Attempts to create a new device backed by the given WebGL2 context.
    pub fn new(context: WebGl2RenderingContext) -> Result<Self, JsValue> {
        let scratch = Rc::new(RefCell::new(MapMemory::new()));

        let program = Shader::new(
            context.clone(),
            include_str!("shaders/vert.glsl"),
            include_str!("shaders/frag.glsl"),
        );
        let present_program = Shader::new(
            context.clone(),
            include_str!("shaders/vert.glsl"),
            include_str!("shaders/present.glsl"),
        );

        let bvh_tex = TextureBuffer::new(context.clone(), scratch.clone());
        let tri_tex = TextureBuffer::new(context.clone(), scratch.clone());

        let samples = RenderTexture::new(context.clone());
        let framebuffers = FramebufferCache::new(context.clone());

        let camera_buffer = UniformBuffer::new(context.clone(), scratch.clone());

        Ok(Self {
            context,
            program,
            present_program,
            camera_buffer,
            bvh_tex,
            tri_tex,
            samples,
            framebuffers,
            lost: true,
        })
    }

    /// Signals the context was lost.
    pub fn context_lost(&mut self) {
        self.lost = true;
    }

    /// Configures this device to render a scene.
    pub fn update(&mut self, scene: &mut Scene) {
        if self.lost && self.try_restore(scene) {
            return; // the context is still lost
        }

        let mut invalidated = false;

        // Update all device buffers to ensure they are in sync with the current state
        // of the scene; the scene tracks a few dirty flags to avoid unnecessary work.

        invalidated |= Dirty::clean(&mut scene.camera, |camera| {
            camera.update(&mut self.camera_buffer)
        });

        invalidated |= Dirty::clean(&mut scene.bvh_data, |bvh_data| {
            self.bvh_tex.map_update(bvh_data.len(), |memory| {
                memory.copy_from_slice(&bvh_data);
            });

            let bvh_limit = (bvh_data.len() / (4 * 4 * 2)) as u32;

            // TODO: this is annoying, what can we do about it?
            //  -> remove need for bvh_limit
            //  -> always upload it during refine()

            self.context.use_program(self.program.resource());
            self.context.uniform1ui(
                self.context
                    .get_uniform_location(self.program.resource().unwrap(), "bvh_limit")
                    .as_ref(),
                bvh_limit,
            );
        });

        invalidated |= Dirty::clean(&mut scene.tri_data, |tri_data| {
            self.tri_tex.map_update(tri_data.len(), |memory| {
                memory.copy_from_slice(&tri_data);
            });
        });

        invalidated |= Dirty::clean(&mut scene.dimensions, |dimensions| {
            self.samples.resize(dimensions.0, dimensions.1);
        });

        // If any device contents whatsoever changed, clear the render buffers and begin
        // refining from the start. This might possibly be more selective in the future.

        if invalidated {
            self.clear();
        }
    }

    // TODO: return stats (e.g. measured with performance counters and so on)
    // (make this optional)

    /// Further refines the path-traced render buffer.
    pub fn refine(&mut self) {
        if self.lost {
            return;
        }

        self.context.use_program(self.program.resource());

        let fbo = self
            .framebuffers
            .get_framebuffer("samples", &[&self.samples]);

        self.context
            .bind_framebuffer(WebGl2RenderingContext::DRAW_FRAMEBUFFER, fbo);

        // bind camera buffer
        self.context.bind_buffer(
            WebGl2RenderingContext::UNIFORM_BUFFER,
            self.camera_buffer.resource(),
        ); // TODO: don't need this?
        self.context.bind_buffer_base(
            WebGl2RenderingContext::UNIFORM_BUFFER,
            0,
            self.camera_buffer.resource(),
        );

        // set proper binding points for textures
        self.context.uniform1i(
            self.context
                .get_uniform_location(self.program.resource().unwrap(), "bvh_data")
                .as_ref(),
            0,
        );
        self.context.uniform1i(
            self.context
                .get_uniform_location(self.program.resource().unwrap(), "tri_data")
                .as_ref(),
            1,
        );

        // bind the textures
        self.context
            .active_texture(WebGl2RenderingContext::TEXTURE0);
        self.context
            .bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.bvh_tex.resource());
        self.context
            .active_texture(WebGl2RenderingContext::TEXTURE0 + 1);
        self.context
            .bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.tri_tex.resource());

        let mut random = [0u8; 4];
        let mut value: u32 = 0;

        while value < 1_000_000_000 {
            let window = web_sys::window().unwrap();

            let crypto = window.crypto().unwrap();

            crypto.get_random_values_with_u8_array(&mut random).unwrap();

            value = (random[0] as u32)
                + ((random[1] as u32) << 8)
                + ((random[2] as u32) << 16)
                + ((random[3] as u32) << 24);
        }

        self.context.uniform1ui(
            self.context
                .get_uniform_location(self.program.resource().unwrap(), "seed")
                .as_ref(),
            value,
        );

        self.context
            .bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, None);

        self.context.enable(WebGl2RenderingContext::BLEND);

        self.context
            .blend_equation(WebGl2RenderingContext::FUNC_ADD);

        self.context
            .blend_func(WebGl2RenderingContext::ONE, WebGl2RenderingContext::ONE);

        self.context
            .draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 3);
    }

    /// Displays the current render buffer to the screen.
    pub fn render(&mut self) {
        if self.lost {
            return;
        }

        self.context
            .bind_framebuffer(WebGl2RenderingContext::DRAW_FRAMEBUFFER, None);

        self.context.clear_color(0.0, 0.0, 1.0, 1.0);
        self.context.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

        self.context.use_program(self.present_program.resource());

        // set proper binding points for textures
        self.context.uniform1i(
            self.context
                .get_uniform_location(self.program.resource().unwrap(), "samples")
                .as_ref(),
            0,
        );

        // bind the textures
        self.context
            .active_texture(WebGl2RenderingContext::TEXTURE0);
        self.context
            .bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.samples.resource());

        self.context.disable(WebGl2RenderingContext::BLEND);

        self.context
            .draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 3);
    }

    fn try_restore(&mut self, scene: &mut Scene) -> bool {
        if self.context.is_context_lost() {
            return true; // can't restore
        }

        // Load required WebGL extensions. This is the only actually required extension
        // but it's well supported; explode if it's not available as nothing will work.

        self.try_load_extension("EXT_color_buffer_float");

        // Initialize (or restore after context loss) all WebGL resources. Also perform
        // any one-time configuration e.g. texture sampler uniforms which never change.

        self.camera_buffer.reset();
        self.program.reset();
        self.present_program.reset();
        self.bvh_tex.reset();
        self.tri_tex.reset();
        self.samples.reset();
        self.framebuffers.reset();

        let camera_index = self
            .context
            .get_uniform_block_index(self.program.resource().unwrap(), "Camera");
        self.context
            .uniform_block_binding(self.program.resource().unwrap(), camera_index, 0);

        // Mark the scene as dirty, necessary since all device buffers have been
        // invalidated; the buffers will get updated once we return to `update`.

        scene.dirty_all();
        self.lost = false;

        false
    }

    fn clear(&mut self) {
        let fbo = self
            .framebuffers
            .get_framebuffer("samples", &[&self.samples]);

        self.context
            .bind_framebuffer(WebGl2RenderingContext::DRAW_FRAMEBUFFER, fbo);

        self.context.clear_bufferfv_with_f32_array(
            WebGl2RenderingContext::COLOR,
            0,
            &[0.0, 0.0, 0.0, 0.0],
        );
    }

    fn try_load_extension(&self, name: &str) {
        if let Err(_) | Ok(None) = self.context.get_extension(name) {
            error(&format!("fatal: required extension '{}'", name));
            panic!("required WebGL2 extension {} is missing", name);
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
}
