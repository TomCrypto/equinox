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
    pub handle: Option<WebGlFramebuffer>, // TODO: make private
}

impl Framebuffer {
    pub fn new(gl: Context) -> Self {
        Self { gl, handle: None }
    }

    pub fn handle(&self) -> Option<&WebGlFramebuffer> {
        self.handle.as_ref()
    }

    pub fn invalidate(&mut self) {
        self.handle = None;
    }

    pub fn rebuild(&mut self, attachments: &[&dyn AsAttachment]) {
        if let Err(_) | Ok(None) = self.gl.get_extension("EXT_color_buffer_float") {
            panic!("the WebGL2 extension `EXT_color_buffer_float' is unavailable");
        }

        assert!(!attachments.is_empty());

        if let Some(framebuffer_handle) = &self.handle {
            self.gl.delete_framebuffer(Some(framebuffer_handle));
        }

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

    pub fn clear(&self, attachment: usize, color: [f32; 4]) {
        self.gl
            .bind_framebuffer(Context::DRAW_FRAMEBUFFER, self.handle.as_ref());

        self.gl
            .clear_bufferfv_with_f32_array(Context::COLOR, attachment as i32, &color);
    }

    pub fn clear_ui(&self, attachment: usize, value: [u32; 4]) {
        self.gl
            .bind_framebuffer(Context::DRAW_FRAMEBUFFER, self.handle.as_ref());

        self.gl
            .clear_bufferuiv_with_u32_array(Context::COLOR, attachment as i32, &value);
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        if let Some(framebuffer_handle) = &self.handle {
            self.gl.delete_framebuffer(Some(framebuffer_handle));
        }
    }
}
