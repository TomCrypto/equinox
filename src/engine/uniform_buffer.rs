#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{AsBindTarget, BindTarget};
use std::marker::PhantomData;
use std::mem::size_of;
use web_sys::{WebGl2RenderingContext as Context, WebGlBuffer};
use zerocopy::{AsBytes, FromBytes};

/*

UBOs start without a size, as before. when written to, they get resized as needed. the maximum
size can be queried from a method on the UBO, and this can be queried by the caller before trying
to create the UBO to abort if the UBO is too large.



*/

#[derive(Debug)]
pub struct UniformBuffer<T: ?Sized> {
    gl: Context,
    handle: Option<WebGlBuffer>,
    len: usize,
    phantom: PhantomData<T>,
}

impl<T: AsBytes + FromBytes> UniformBuffer<[T]> {
    pub fn write_array(&mut self, contents: &[T]) {
        if self.len != contents.len() || self.handle.is_none() {
            self.create_and_allocate(size_of::<T>() * contents.len().max(1));
            self.len = contents.len().max(1);
        }

        self.gl
            .bind_buffer(Context::UNIFORM_BUFFER, self.handle.as_ref());

        self.gl.buffer_sub_data_with_i32_and_u8_array(
            Context::UNIFORM_BUFFER,
            0,
            contents.as_bytes(),
        );
    }

    pub fn max_len(&self) -> usize {
        (self
            .gl
            .get_parameter(Context::MAX_UNIFORM_BLOCK_SIZE)
            .unwrap()
            .as_f64()
            .unwrap() as usize
            * 4)
            / size_of::<T>()
    }
}

impl<T: AsBytes + FromBytes> UniformBuffer<T> {
    pub fn write(&mut self, contents: &T) {
        if self.len != 1 || self.handle.is_none() {
            self.create_and_allocate(size_of::<T>());
            self.len = 1;
        }

        self.gl
            .bind_buffer(Context::UNIFORM_BUFFER, self.handle.as_ref());

        self.gl.buffer_sub_data_with_i32_and_u8_array(
            Context::UNIFORM_BUFFER,
            0,
            contents.as_bytes(),
        );
    }
}

impl<T: ?Sized> UniformBuffer<T> {
    pub fn new(gl: Context) -> Self {
        Self {
            gl,
            handle: None,
            len: 0,
            phantom: PhantomData,
        }
    }

    pub fn invalidate(&mut self) {
        self.handle = None;
    }

    pub fn element_count(&self) -> usize {
        self.len
    }

    fn create_and_allocate(&mut self, size: usize) {
        if let Some(buffer_handle) = &self.handle {
            self.gl.delete_buffer(Some(buffer_handle));
        }

        self.handle = self.gl.create_buffer();

        self.gl
            .bind_buffer(Context::UNIFORM_BUFFER, self.handle.as_ref());
        self.gl
            .buffer_data_with_i32(Context::UNIFORM_BUFFER, size as i32, Context::STATIC_DRAW);
    }
}

impl<T: ?Sized> Drop for UniformBuffer<T> {
    fn drop(&mut self) {
        if let Some(buffer_handle) = &self.handle {
            self.gl.delete_buffer(Some(buffer_handle));
        }
    }
}

impl<T: ?Sized> AsBindTarget for UniformBuffer<T> {
    fn bind_target(&self) -> BindTarget {
        BindTarget::UniformBuffer(self.handle.as_ref())
    }
}
