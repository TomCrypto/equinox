#[allow(unused_imports)]
use log::{debug, info, warn};

use js_sys::Array;
use web_sys::{WebGl2RenderingContext as Context, WebGlFramebuffer, WebGlTexture};

pub enum Attachment<'a> {
    Texture(Option<&'a WebGlTexture>),
}

pub trait AsAttachment {
    fn as_attachment(&self) -> Attachment;
}

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

    pub fn bind_to_pipeline(&mut self) -> BoundFramebuffer {
        self.gl
            .bind_framebuffer(Context::DRAW_FRAMEBUFFER, self.handle.as_ref());

        BoundFramebuffer::new(self)
    }

    pub fn bind_canvas_to_pipeline(gl: &Context) {
        gl.bind_framebuffer(Context::DRAW_FRAMEBUFFER, None);
    }
}

// TODO: some way to unify this with the canvas framebuffer...

pub struct BoundFramebuffer<'a> {
    framebuffer: &'a mut Framebuffer,
}

impl<'a> BoundFramebuffer<'a> {
    fn new(framebuffer: &'a mut Framebuffer) -> Self {
        Self { framebuffer }
    }

    pub fn clear(&mut self, attachment: i32, rgba: &[f32]) {
        self.framebuffer
            .gl
            .clear_bufferfv_with_f32_array(Context::COLOR, attachment, rgba);
    }

    // TODO: APIs to set the blend state, set viewport/scissor, issue draw calls
    // etc.
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        self.gl.delete_framebuffer(self.handle.as_ref());
    }
}
