#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::engine::{ShaderBind, ShaderBindHandle};
use crate::AlignedMemory;
use coherence_base::device::ToDevice;
use js_sys::{Float32Array, Uint32Array};
use std::marker::PhantomData;
use std::mem::size_of;
use web_sys::{WebGl2RenderingContext as Context, WebGlTexture};
use zerocopy::LayoutVerified;
use zerocopy::{AsBytes, FromBytes};

#[derive(Clone, Copy, Debug)]
pub enum TextureBufferFormat {
    U32x4,
    F32x4,
}

pub struct TextureBuffer<T: ?Sized> {
    gl: Context,

    handle: Option<WebGlTexture>,
    storage_w: i32,
    storage_h: i32,

    pixels_per_row: i32,

    format: TextureBufferFormat,

    phantom: PhantomData<T>,
}

fn divide_round_up(a: i32, d: i32) -> i32 {
    (a + d - 1) / d
}

pub fn pixels_per_texture_buffer_row(gl: &Context) -> i32 {
    let param = gl.get_parameter(Context::MAX_TEXTURE_SIZE).unwrap();
    param.as_f64().unwrap() as i32 // this really shouldn't ever fail
}

impl<T> TextureBuffer<[T]> {
    pub fn new(gl: Context, format: TextureBufferFormat) -> Self {
        let pixels_per_row = pixels_per_texture_buffer_row(&gl);

        assert_eq!(Self::pixel_size(format) % size_of::<T>(), 0);

        Self {
            gl,
            handle: None,
            storage_w: 0,
            storage_h: 0,
            pixels_per_row,
            format,
            phantom: PhantomData,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.handle = None;
        self.storage_w = 0;
        self.storage_h = 0;
    }

    fn pixel_size(format: TextureBufferFormat) -> usize {
        match format {
            TextureBufferFormat::U32x4 => 16,
            TextureBufferFormat::F32x4 => 16,
        }
    }
}

impl<T: AsBytes + FromBytes> TextureBuffer<[T]> {
    pub fn write(&mut self, buffer: &mut AlignedMemory, source: &impl ToDevice<[T]>) {
        let size = source.requested_count() * size_of::<T>();

        self.bind_and_upload(buffer.allocate_bytes(size), |bytes| {
            source.to_device(
                LayoutVerified::<_, [T]>::new_slice_zeroed(bytes)
                    .unwrap()
                    .into_mut_slice(),
            );
        });
    }

    fn bind_and_upload(&mut self, bytes: &mut [u8], writer: impl FnOnce(&mut [u8])) {
        self.reallocate_if_necessary(bytes.len());

        writer(&mut bytes[..]);

        match self.format {
            TextureBufferFormat::U32x4 => self.upload_u32x4(bytes),
            TextureBufferFormat::F32x4 => self.upload_f32x4(bytes),
        }
    }

    fn dimensions_for_size(&self, size: usize) -> (i32, i32) {
        let pixels = divide_round_up(size as i32, Self::pixel_size(self.format) as i32).max(1);

        (
            self.pixels_per_row,
            divide_round_up(pixels, self.pixels_per_row),
        )
    }

    fn reallocate_if_necessary(&mut self, size: usize) {
        let (new_w, new_h) = self.dimensions_for_size(size);

        if new_w != self.storage_w || new_h != self.storage_h {
            self.allocate_texture_storage(new_w, new_h);
        } else {
            self.gl
                .bind_texture(Context::TEXTURE_2D, self.handle.as_ref());
        }
    }

    fn allocate_texture_storage(&mut self, storage_w: i32, storage_h: i32) {
        self.gl.delete_texture(self.handle.as_ref());

        self.handle = self.gl.create_texture();

        self.gl
            .bind_texture(Context::TEXTURE_2D, self.handle.as_ref());

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

    fn upload_u32x4(&self, bytes: &[u8]) {
        let data: LayoutVerified<_, [u32]> = LayoutVerified::new_slice(bytes).unwrap();

        for (y, row) in data.chunks(4 * self.storage_w as usize).enumerate() {
            let view = unsafe { Uint32Array::view(row) };

            self.gl
                .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
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

    fn upload_f32x4(&self, bytes: &[u8]) {
        let data: LayoutVerified<_, [f32]> = LayoutVerified::new_slice(bytes).unwrap();

        for (y, row) in data.chunks(4 * self.storage_w as usize).enumerate() {
            let view = unsafe { Float32Array::view(row) };

            self.gl
                .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
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

impl<T: ?Sized> Drop for TextureBuffer<T> {
    fn drop(&mut self) {
        self.gl.delete_texture(self.handle.as_ref());
    }
}

impl<T: ?Sized> ShaderBind for TextureBuffer<T> {
    fn handle(&self) -> ShaderBindHandle {
        ShaderBindHandle::Texture(self.handle.as_ref())
    }
}
