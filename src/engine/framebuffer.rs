#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{Color, DepthStencil, RenderTarget};
use js_sys::{Array, Error};
use web_sys::{WebGl2RenderingContext as Context, WebGlFramebuffer, WebGlTexture};

pub trait AsAttachment {
    type Target: RenderTarget;

    fn as_attachment(&self) -> Option<&WebGlTexture>;

    fn attachment_dimensions(&self) -> (usize, usize);
}

#[derive(Debug)]
pub struct Framebuffer {
    gl: Context,
    handle: Option<WebGlFramebuffer>,
    cols: usize,
    rows: usize,
}

impl Framebuffer {
    pub fn new(gl: Context) -> Self {
        Self {
            gl,
            handle: None,
            cols: 0,
            rows: 0,
        }
    }

    pub fn handle(&self) -> Option<&WebGlFramebuffer> {
        self.handle.as_ref()
    }

    pub fn invalidate(&mut self) {
        self.handle = None;
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn rebuild(
        &mut self,
        attachments: &[&dyn AsAttachment<Target = Color>],
        depth_stencil: Option<&dyn AsAttachment<Target = DepthStencil>>,
    ) -> Result<(), Error> {
        if let Err(_) | Ok(None) = self.gl.get_extension("EXT_color_buffer_float") {
            return Err(Error::new("extension `EXT_color_buffer_float' missing"));
        }

        if let Err(_) | Ok(None) = self.gl.get_extension("EXT_float_blend") {
            return Err(Error::new("extension `EXT_float_blend' missing"));
        }

        assert!(!attachments.is_empty());

        let (cols, rows) = attachments[0].attachment_dimensions();

        if let Some(framebuffer_handle) = &self.handle {
            self.gl.delete_framebuffer(Some(framebuffer_handle));
        }

        self.handle = self.gl.create_framebuffer();

        self.gl
            .bind_framebuffer(Context::DRAW_FRAMEBUFFER, self.handle.as_ref());

        let array = Array::new();

        for (index, attachment) in attachments.iter().enumerate() {
            let (next_cols, next_rows) = attachment.attachment_dimensions();

            if (next_cols, next_rows) != (cols, rows) {
                panic!("inconsistent framebuffer attachment dimensions");
            }

            let attachment_index = Context::COLOR_ATTACHMENT0 + index as u32;

            self.gl.framebuffer_texture_2d(
                Context::DRAW_FRAMEBUFFER,
                attachment_index,
                Context::TEXTURE_2D,
                attachment.as_attachment(),
                0,
            );

            array.push(&attachment_index.into());
        }

        if let Some(depth_stencil) = depth_stencil {
            self.gl.framebuffer_texture_2d(
                Context::DRAW_FRAMEBUFFER,
                Context::DEPTH_STENCIL_ATTACHMENT,
                Context::TEXTURE_2D,
                depth_stencil.as_attachment(),
                0,
            );
        }

        self.gl.draw_buffers(&array);
        self.cols = cols;
        self.rows = rows;

        Ok(())
    }

    pub fn clear(&self, attachment: usize, color: [f32; 4]) {
        self.gl
            .bind_framebuffer(Context::DRAW_FRAMEBUFFER, self.handle.as_ref());

        self.gl
            .clear_bufferfv_with_f32_array(Context::COLOR, attachment as i32, &color);
    }

    pub fn clear_depth_stencil(&self, depth: f32, stencil: u8) {
        self.gl
            .bind_framebuffer(Context::DRAW_FRAMEBUFFER, self.handle.as_ref());

        self.gl
            .clear_bufferfi(Context::DEPTH_STENCIL, 0, depth, stencil as i32);
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        if let Some(framebuffer_handle) = &self.handle {
            self.gl.delete_framebuffer(Some(framebuffer_handle));
        }
    }
}
