#[allow(unused_imports)]
use log::{debug, info, warn};

use std::marker::PhantomData;
use web_sys::{WebGl2RenderingContext as Context, WebGlBuffer, WebGlVertexArrayObject};
use zerocopy::{AsBytes, FromBytes};

#[derive(Debug)]
pub struct VertexArray<T: ?Sized> {
    gl: Context,
    handle: Option<WebGlBuffer>,
    vao_handle: Option<WebGlVertexArrayObject>,
    size: usize,
    phantom: PhantomData<T>,
}

impl<T: ?Sized> VertexArray<T> {
    pub fn new(gl: Context) -> Self {
        Self {
            gl,
            handle: None,
            vao_handle: None,
            size: 0,
            phantom: PhantomData,
        }
    }
}

impl<T: AsBytes + FromBytes + VertexLayout> VertexArray<[T]> {
    pub fn vertex_count(&self) -> usize {
        self.size / std::mem::size_of::<T>()
    }

    pub fn bind(&self) {
        self.gl.bind_vertex_array(self.vao_handle.as_ref());
    }

    pub fn unbind(&self) {
        self.gl.bind_vertex_array(None);
    }

    pub fn upload(&mut self, data: &[T]) {
        if data.len() != self.vertex_count() || !self.gl.is_buffer(self.handle.as_ref()) {
            self.create_and_allocate(data.len() * std::mem::size_of::<T>());
        }

        self.gl
            .bind_buffer(Context::ARRAY_BUFFER, self.handle.as_ref());

        self.gl
            .buffer_sub_data_with_i32_and_u8_array(Context::ARRAY_BUFFER, 0, data.as_bytes());
    }

    fn create_and_allocate(&mut self, size: usize) {
        self.gl.delete_buffer(self.handle.as_ref());
        self.gl.delete_vertex_array(self.vao_handle.as_ref());

        self.handle = self.gl.create_buffer();
        self.vao_handle = self.gl.create_vertex_array();

        self.gl.bind_vertex_array(self.vao_handle.as_ref());

        self.gl
            .bind_buffer(Context::ARRAY_BUFFER, self.handle.as_ref());
        self.gl
            .buffer_data_with_i32(Context::ARRAY_BUFFER, size as i32, Context::STATIC_DRAW);

        let stride = std::mem::size_of::<T>() as i32;

        for attribute in T::vertex_layout() {
            match attribute.kind {
                VertexAttributeKind::Uint => {
                    self.gl.vertex_attrib_i_pointer_with_i32(
                        attribute.index as u32,
                        1,
                        Context::UNSIGNED_INT,
                        stride,
                        attribute.offset as i32,
                    );
                }
                VertexAttributeKind::UShort4 => {
                    self.gl.vertex_attrib_i_pointer_with_i32(
                        attribute.index as u32,
                        4,
                        Context::UNSIGNED_SHORT,
                        stride,
                        attribute.offset as i32,
                    );
                }
            }

            self.gl.enable_vertex_attrib_array(attribute.index as u32);
        }

        // TODO: can remove this when we are VAO-clean?
        self.gl.bind_vertex_array(None);

        self.size = size;
    }
}

impl<T: ?Sized> Drop for VertexArray<T> {
    fn drop(&mut self) {
        self.gl.delete_buffer(self.handle.as_ref());
        self.gl.delete_vertex_array(self.vao_handle.as_ref());
    }
}

pub trait VertexLayout {
    fn vertex_layout() -> Vec<VertexAttribute>;
}

#[derive(Clone, Copy, Debug)]
pub enum VertexAttributeKind {
    UShort4,
    Uint,
}

#[derive(Clone, Copy, Debug)]
pub struct VertexAttribute {
    pub kind: VertexAttributeKind,
    pub index: usize,
    pub offset: usize,
}

impl VertexAttribute {
    pub fn new(index: usize, offset: usize, kind: VertexAttributeKind) -> Self {
        Self {
            kind,
            index,
            offset,
        }
    }
}
