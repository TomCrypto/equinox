#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::AlignedMemory;
use crate::{ShaderBind, ShaderBindHandle};
use std::marker::PhantomData;
use web_sys::{WebGl2RenderingContext as Context, WebGlBuffer};
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

use coherence_base::device::ToDevice;

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
            size: count * std::mem::size_of::<T>(),
            phantom: PhantomData,
        }
    }

    pub fn write_array(&mut self, buffer: &mut AlignedMemory, source: &impl ToDevice<[T]>) {
        self.bind_and_upload(buffer.allocate_bytes(self.size), |bytes| {
            source.to_device(
                LayoutVerified::<_, [T]>::new_slice_zeroed(bytes)
                    .unwrap()
                    .into_mut_slice(),
            );
        });
    }
}

impl<T: AsBytes + FromBytes> UniformBuffer<T> {
    pub fn new(gl: Context) -> Self {
        Self {
            gl,
            handle: None,
            size: std::mem::size_of::<T>(),
            phantom: PhantomData,
        }
    }

    pub fn write(&mut self, buffer: &mut AlignedMemory, source: &impl ToDevice<T>) {
        self.bind_and_upload(buffer.allocate_bytes(self.size), |bytes| {
            source.to_device(
                LayoutVerified::<_, T>::new_zeroed(bytes)
                    .unwrap()
                    .into_mut(),
            );
        });
    }

    // TODO: find a way to remove this later on
    pub fn write_direct(&mut self, buffer: &mut AlignedMemory, writer: impl FnOnce(&mut T)) {
        self.bind_and_upload(buffer.allocate_bytes(self.size), |bytes| {
            writer(
                LayoutVerified::<_, T>::new_zeroed(bytes)
                    .unwrap()
                    .into_mut(),
            );
        });
    }
}

impl<T: ?Sized> UniformBuffer<T> {
    pub(crate) fn reset(&mut self) {
        self.handle = self.gl.create_buffer();

        self.gl
            .bind_buffer(Context::UNIFORM_BUFFER, self.handle.as_ref());
        self.gl.buffer_data_with_i32(
            Context::UNIFORM_BUFFER,
            self.size as i32,
            Context::DYNAMIC_DRAW,
        );
    }

    fn bind_and_upload(&mut self, bytes: &mut [u8], writer: impl FnOnce(&mut [u8])) {
        self.gl
            .bind_buffer(Context::UNIFORM_BUFFER, self.handle.as_ref());

        writer(&mut bytes[..]);

        self.gl
            .buffer_sub_data_with_i32_and_u8_array(Context::UNIFORM_BUFFER, 0, bytes);
    }
}

impl<T: ?Sized> Drop for UniformBuffer<T> {
    fn drop(&mut self) {
        self.gl.delete_buffer(self.handle.as_ref());
    }
}

impl<T: ?Sized> ShaderBind for UniformBuffer<T> {
    fn handle(&self) -> ShaderBindHandle {
        ShaderBindHandle::UniformBuffer(self.handle.as_ref())
    }
}
