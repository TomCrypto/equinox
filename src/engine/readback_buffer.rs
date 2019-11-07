#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Framebuffer;
use js_sys::Error;
use std::marker::PhantomData;
use std::mem::size_of;
use web_sys::{WebGl2RenderingContext as Context, WebGlBuffer, WebGlSync};
use zerocopy::{AsBytes, FromBytes};

#[derive(Debug)]
pub struct ReadbackBuffer<T: ?Sized> {
    gl: Context,
    handle: Option<WebGlBuffer>,
    sync_handle: Option<WebGlSync>,
    pub len: usize,
    phantom: PhantomData<T>,
}

impl<T: AsBytes + FromBytes> ReadbackBuffer<[T]> {
    pub fn create(&mut self, len: usize) {
        if self.len != len || self.handle.is_none() {
            self.create_and_allocate(len * size_of::<T>());
            self.len = len;
        }
    }

    pub fn start_readback(
        &mut self,
        cols: usize,
        rows: usize,
        framebuffer: &Framebuffer,
        attachment: usize,
    ) -> Result<(), Error> {
        if let Some(sync_handle) = &self.sync_handle {
            self.gl.delete_sync(Some(sync_handle));
        }

        // TODO: how to expose the FBO handle safely??
        // TODO: how to pass in the pixel type/format?

        //info!("Readback started (sync inserted)");

        self.gl.bind_framebuffer(Context::DRAW_FRAMEBUFFER, None);
        self.gl
            .bind_framebuffer(Context::READ_FRAMEBUFFER, framebuffer.handle.as_ref());
        self.gl
            .read_buffer(Context::COLOR_ATTACHMENT0 + attachment as u32);

        self.gl
            .bind_buffer(Context::PIXEL_PACK_BUFFER, self.handle.as_ref());

        self.gl.read_pixels_with_i32(
            0,
            0,
            cols as i32,
            rows as i32,
            Context::RGBA,
            Context::FLOAT,
            0,
        )?;

        self.gl.bind_framebuffer(Context::READ_FRAMEBUFFER, None);
        self.gl.bind_buffer(Context::PIXEL_PACK_BUFFER, None);

        self.sync_handle = self.gl.fence_sync(Context::SYNC_GPU_COMMANDS_COMPLETE, 0);

        Ok(())
    }

    pub fn end_readback(&mut self, data: &mut [T]) -> bool {
        if let Some(sync_handle) = &self.sync_handle {
            if self
                .gl
                .get_sync_parameter(sync_handle, Context::SYNC_STATUS)
                == Context::SIGNALED
            {
                self.gl
                    .bind_buffer(Context::PIXEL_PACK_BUFFER, self.handle.as_ref());

                self.gl.get_buffer_sub_data_with_i32_and_u8_array(
                    Context::PIXEL_PACK_BUFFER,
                    0,
                    data.as_bytes_mut(),
                );

                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

impl<T: ?Sized> ReadbackBuffer<T> {
    pub fn new(gl: Context) -> Self {
        Self {
            gl,
            handle: None,
            sync_handle: None,
            len: 0,
            phantom: PhantomData,
        }
    }

    pub fn invalidate(&mut self) {
        self.handle = None;
        self.sync_handle = None;
    }

    fn create_and_allocate(&mut self, size: usize) {
        if let Some(buffer_handle) = &self.handle {
            self.gl.delete_buffer(Some(buffer_handle));
        }

        self.handle = self.gl.create_buffer();

        self.gl
            .bind_buffer(Context::PIXEL_PACK_BUFFER, self.handle.as_ref());
        self.gl.buffer_data_with_i32(
            Context::PIXEL_PACK_BUFFER,
            size as i32,
            Context::STREAM_READ,
        );
    }
}

impl<T: ?Sized> Drop for ReadbackBuffer<T> {
    fn drop(&mut self) {
        if let Some(buffer_handle) = &self.handle {
            self.gl.delete_buffer(Some(buffer_handle));
        }

        if let Some(sync_handle) = &self.sync_handle {
            self.gl.delete_sync(Some(sync_handle));
        }
    }
}
