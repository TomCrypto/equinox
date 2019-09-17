#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::ScratchMemory;
use js_sys::{Float32Array, Uint32Array};

use sigma_core::DeviceBuffer;

use zerocopy::LayoutVerified;

use web_sys::{WebGl2RenderingContext as Context, WebGlTexture};

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug)]
pub enum TextureBufferFormat {
    U32x4,
    F32x4,
}

pub struct TextureBuffer {
    gl: Context,
    scratch: Rc<RefCell<ScratchMemory>>,

    handle: Option<WebGlTexture>,
    storage_w: i32,
    storage_h: i32,

    pixels_per_row: i32,

    format: TextureBufferFormat,
}

fn divide_round_up(a: i32, d: i32) -> i32 {
    (a + d - 1) / d
}

impl TextureBuffer {
    pub fn new(
        gl: Context,
        format: TextureBufferFormat,
        scratch: Rc<RefCell<ScratchMemory>>,
    ) -> Self {
        let pixels_per_row = Self::pixels_per_row(&gl);

        Self {
            gl,
            scratch,
            handle: None,
            storage_w: 0,
            storage_h: 0,
            pixels_per_row,
            format,
        }
    }

    pub(crate) fn resource(&self) -> Option<&WebGlTexture> {
        self.handle.as_ref()
    }

    pub fn pixels_per_row(gl: &Context) -> i32 {
        let param = gl.get_parameter(Context::MAX_TEXTURE_SIZE).unwrap();
        param.as_f64().unwrap() as i32 // this really shouldn't ever fail
    }

    fn pixel_size(&self) -> i32 {
        match self.format {
            TextureBufferFormat::U32x4 => 4,
            TextureBufferFormat::F32x4 => 4,
        }
    }

    // ONLY CALLED DURING CONTEXT LOSS (or on first initialization)
    // so we want to do nothing except invalidate the texture and reset the size to
    // zero
    pub(crate) fn reset(&mut self) {
        self.handle = None;
        self.storage_w = 0;
        self.storage_h = 0;
    }

    fn dimensions_for_size(&self, size: usize) -> (i32, i32) {
        assert_eq!(size % (self.pixel_size() as usize), 0);

        let pixels = divide_round_up(size as i32, self.pixel_size()).max(1);

        (
            self.pixels_per_row,
            divide_round_up(pixels, self.pixels_per_row),
        )
    }

    fn allocate_texture_storage(&mut self, storage_w: i32, storage_h: i32) {
        self.gl.delete_texture(self.resource());

        self.handle = self.gl.create_texture();

        self.gl.bind_texture(Context::TEXTURE_2D, self.resource());

        match self.format {
            TextureBufferFormat::U32x4 => {
                self.gl.tex_storage_2d(
                    Context::TEXTURE_2D,
                    1,
                    Context::RGBA32UI,
                    storage_w,
                    storage_h,
                );
            }
            TextureBufferFormat::F32x4 => {
                self.gl.tex_storage_2d(
                    Context::TEXTURE_2D,
                    1,
                    Context::RGBA32F,
                    storage_w,
                    storage_h,
                );
            }
        }

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

        self.storage_w = storage_w;
        self.storage_h = storage_h;
    }

    fn map_update_u32(gl: &Context, width: i32, buffer: &[u8]) {
        let data: LayoutVerified<_, [u32]> = LayoutVerified::new_slice(buffer).unwrap();

        for (y, row) in data.chunks(4 * width as usize).enumerate() {
            let view = unsafe { Uint32Array::view(row) };

            gl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                Context::TEXTURE_2D,
                0,
                0,
                y as i32,
                (row.len() / 4) as i32,
                1,
                Context::RGBA_INTEGER,
                Context::UNSIGNED_INT,
                Some(&view),
            )
            .unwrap();
        }
    }

    fn map_update_f32(gl: &Context, width: i32, buffer: &[u8]) {
        let data: LayoutVerified<_, [f32]> = LayoutVerified::new_slice(buffer).unwrap();

        for (y, row) in data.chunks(4 * width as usize).enumerate() {
            let view = unsafe { Float32Array::view(row) };

            gl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                Context::TEXTURE_2D,
                0,
                0,
                y as i32,
                (row.len() / 4) as i32,
                1,
                Context::RGBA,
                Context::FLOAT,
                Some(&view),
            )
            .unwrap();
        }
    }
}

impl DeviceBuffer for TextureBuffer {
    fn map_update(&mut self, size: usize, f: impl FnOnce(&mut [u8])) {
        let (new_w, new_h) = self.dimensions_for_size(size);

        if new_w != self.storage_w || new_h != self.storage_h {
            self.allocate_texture_storage(new_w, new_h);
        }

        let mut memory = self.scratch.borrow_mut();

        let buffer = memory.access_with_size(size);

        f(buffer);

        self.gl.bind_texture(Context::TEXTURE_2D, self.resource());

        match self.format {
            TextureBufferFormat::U32x4 => Self::map_update_u32(&self.gl, self.storage_w, buffer),
            TextureBufferFormat::F32x4 => Self::map_update_f32(&self.gl, self.storage_w, buffer),
        }
    }
}

impl Drop for TextureBuffer {
    fn drop(&mut self) {
        self.gl.delete_texture(self.resource());
    }
}
