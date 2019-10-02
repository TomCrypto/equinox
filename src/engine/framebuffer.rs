#[allow(unused_imports)]
use log::{debug, info, warn};

use js_sys::Array;
use web_sys::{WebGl2RenderingContext as Context, WebGlFramebuffer, WebGlTexture};

#[derive(Debug)]
pub enum Attachment<'a> {
    Texture(Option<&'a WebGlTexture>),
}

pub trait AsAttachment {
    fn as_attachment(&self) -> Attachment;
}

#[derive(Debug)]
pub struct Framebuffer {
    gl: Context,
    handle: Option<WebGlFramebuffer>,
}

impl Framebuffer {
    pub fn new(gl: Context) -> Self {
        Self { gl, handle: None }
    }

    pub fn invalidate(&mut self, attachments: &[&dyn AsAttachment]) {
        if let Err(_) | Ok(None) = self.gl.get_extension("EXT_color_buffer_float") {
            panic!("the WebGL2 extension `EXT_color_buffer_float' is unavailable");
        }

        assert!(!attachments.is_empty());

        self.gl.delete_framebuffer(self.handle.as_ref());

        self.handle = self.gl.create_framebuffer();

        self.gl
            .bind_framebuffer(Context::DRAW_FRAMEBUFFER, self.handle.as_ref());

        let array = Array::new();

        for (index, attachment) in attachments.iter().enumerate() {
            let attachment_index = Context::COLOR_ATTACHMENT0 + index as u32;

            match attachment.as_attachment() {
                Attachment::Texture(texture) => {
                    self.gl.framebuffer_texture_2d(
                        Context::DRAW_FRAMEBUFFER,
                        attachment_index,
                        Context::TEXTURE_2D,
                        texture,
                        0,
                    );
                }
            }

            array.push(&attachment_index.into());
        }

        self.gl.draw_buffers(&array);
    }

    pub fn draw(&self, options: DrawOptions) {
        self.gl
            .bind_framebuffer(Context::DRAW_FRAMEBUFFER, self.handle.as_ref());

        Self::perform_draw(&self.gl, options);
    }

    pub fn draw_to_canvas(gl: &Context, options: DrawOptions) {
        gl.bind_framebuffer(Context::DRAW_FRAMEBUFFER, None);

        Self::perform_draw(gl, options);
    }

    pub fn clear(&self, attachment: usize, color: [f32; 4]) {
        self.gl
            .bind_framebuffer(Context::DRAW_FRAMEBUFFER, self.handle.as_ref());

        self.gl
            .clear_bufferfv_with_f32_array(Context::COLOR, attachment as i32, &color);
    }

    pub fn clear_canvas(gl: &Context, color: [f32; 4]) {
        gl.bind_framebuffer(Context::DRAW_FRAMEBUFFER, None);

        gl.clear_color(color[0], color[1], color[2], color[3]);

        gl.clear(Context::COLOR_BUFFER_BIT);
    }

    fn perform_draw(gl: &Context, options: DrawOptions) {
        gl.viewport(
            options.viewport[0],
            options.viewport[1],
            options.viewport[2],
            options.viewport[3],
        );

        if let Some(scissor) = options.scissor {
            gl.enable(Context::SCISSOR_TEST);

            gl.scissor(scissor[0], scissor[1], scissor[2], scissor[3]);
        } else {
            gl.disable(Context::SCISSOR_TEST);
        }

        if let Some(blend) = options.blend {
            gl.enable(Context::BLEND);

            match blend {
                BlendMode::Accumulative { weight } => {
                    gl.blend_equation(Context::FUNC_ADD);
                    gl.blend_func(Context::CONSTANT_ALPHA, Context::ONE_MINUS_CONSTANT_ALPHA);
                    gl.blend_color(0.0, 0.0, 0.0, 1.0 - weight);
                }
            }
        } else {
            gl.disable(Context::BLEND);
        }

        if let Some(range) = options.vertices {
            gl.draw_arrays(Context::TRIANGLES, range.index as i32, range.count as i32);
        } else {
            gl.bind_buffer(Context::ARRAY_BUFFER, None);
            gl.draw_arrays(Context::TRIANGLES, 0, 3);
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        self.gl.delete_framebuffer(self.handle.as_ref());
    }
}

pub enum BlendMode {
    Accumulative { weight: f32 },
}

pub struct DrawOptions {
    pub viewport: [i32; 4],
    pub scissor: Option<[i32; 4]>,
    pub blend: Option<BlendMode>,
    pub vertices: Option<DrawRange>,
}

pub struct DrawRange {
    pub index: usize,
    pub count: usize,
}
