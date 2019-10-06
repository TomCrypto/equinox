#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{AsBindTarget, BindTarget};
use std::marker::PhantomData;
use std::mem::size_of;
use web_sys::{WebGl2RenderingContext as Context, WebGlBuffer};
use zerocopy::{AsBytes, FromBytes};

#[derive(Debug)]
pub struct UniformBuffer<T: ?Sized> {
    gl: Context,
    handle: Option<WebGlBuffer>,
    size: usize,
    phantom: PhantomData<T>,
}

impl<T: AsBytes + FromBytes> UniformBuffer<[T]> {
    pub fn new_array(gl: Context, count: usize) -> Self {
        Self {
            gl,
            handle: None,
            size: count * size_of::<T>(),
            phantom: PhantomData,
        }
    }

    pub fn write_array(&mut self, contents: &[T]) {
        if self.handle.is_none() {
            self.create_and_allocate();
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

impl<T: AsBytes + FromBytes> UniformBuffer<T> {
    pub fn new(gl: Context) -> Self {
        Self {
            gl,
            handle: None,
            size: size_of::<T>(),
            phantom: PhantomData,
        }
    }

    pub fn write(&mut self, contents: &T) {
        if self.handle.is_none() {
            self.create_and_allocate();
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
    pub fn invalidate(&mut self) {
        self.handle = None;
    }

    fn create_and_allocate(&mut self) {
        self.handle = self.gl.create_buffer();

        self.gl
            .bind_buffer(Context::UNIFORM_BUFFER, self.handle.as_ref());
        self.gl.buffer_data_with_i32(
            Context::UNIFORM_BUFFER,
            self.size as i32,
            Context::DYNAMIC_DRAW,
        );
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
