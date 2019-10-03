#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::AsVertexArray;
use std::marker::PhantomData;
use std::mem::size_of;
use web_sys::{WebGl2RenderingContext as Context, WebGlBuffer, WebGlVertexArrayObject};
use zerocopy::{AsBytes, FromBytes};

#[derive(Debug)]
pub struct VertexArray<T: ?Sized> {
    gl: Context,
    buf_handle: Option<WebGlBuffer>,
    vao_handle: Option<WebGlVertexArrayObject>,
    vertex_count: usize,
    phantom: PhantomData<T>,
}

impl<T: ?Sized> VertexArray<T> {
    pub fn new(gl: Context) -> Self {
        Self {
            gl,
            buf_handle: None,
            vao_handle: None,
            vertex_count: 0,
            phantom: PhantomData,
        }
    }
}

impl<T: VertexLayout> VertexArray<[T]> {
    pub fn bind(&self) {
        self.gl.bind_vertex_array(self.vao_handle.as_ref());
    }

    pub fn unbind(&self) {
        self.gl.bind_vertex_array(None);
    }

    pub fn upload(&mut self, vertices: &[T]) {
        assert!(!vertices.is_empty());

        if vertices.len() != self.vertex_count || !self.gl.is_buffer(self.buf_handle.as_ref()) {
            self.create_buffer(vertices.len());
        }

        if !self.gl.is_vertex_array(self.vao_handle.as_ref()) {
            self.create_vertex_array();
        }

        self.gl
            .bind_buffer(Context::ARRAY_BUFFER, self.buf_handle.as_ref());

        self.gl.buffer_sub_data_with_i32_and_u8_array(
            Context::ARRAY_BUFFER,
            0,
            vertices.as_bytes(),
        );
    }

    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    fn create_vertex_array(&mut self) {
        if self.gl.is_vertex_array(self.vao_handle.as_ref()) {
            self.gl.delete_vertex_array(self.vao_handle.as_ref());
        }

        self.vao_handle = self.gl.create_vertex_array();

        self.gl.bind_vertex_array(self.vao_handle.as_ref());

        for attribute in T::VERTEX_LAYOUT {
            match attribute.kind {
                VertexAttributeKind::Uint => {
                    self.gl.vertex_attrib_i_pointer_with_i32(
                        attribute.index as u32,
                        1,
                        Context::UNSIGNED_INT,
                        size_of::<T>() as i32,
                        attribute.offset as i32,
                    );
                }
                VertexAttributeKind::UShort4 => {
                    self.gl.vertex_attrib_i_pointer_with_i32(
                        attribute.index as u32,
                        4,
                        Context::UNSIGNED_SHORT,
                        size_of::<T>() as i32,
                        attribute.offset as i32,
                    );
                }
            }

            self.gl
                .bind_buffer(Context::ARRAY_BUFFER, self.buf_handle.as_ref());

            self.gl.enable_vertex_attrib_array(attribute.index as u32);
        }
    }

    fn create_buffer(&mut self, vertex_count: usize) {
        if self.gl.is_buffer(self.buf_handle.as_ref()) {
            self.gl.delete_buffer(self.buf_handle.as_ref());
        }

        self.buf_handle = self.gl.create_buffer();

        self.gl
            .bind_buffer(Context::ARRAY_BUFFER, self.buf_handle.as_ref());

        self.gl.buffer_data_with_i32(
            Context::ARRAY_BUFFER,
            (vertex_count * size_of::<T>()) as i32,
            Context::STATIC_DRAW,
        );

        self.vertex_count = vertex_count;
    }
}

impl<T: VertexLayout> AsVertexArray for VertexArray<[T]> {
    fn vertex_array(&self) -> Option<&WebGlVertexArrayObject> {
        self.vao_handle.as_ref()
    }
}

impl<T: ?Sized> Drop for VertexArray<T> {
    fn drop(&mut self) {
        if self.gl.is_buffer(self.buf_handle.as_ref()) {
            self.gl.delete_buffer(self.buf_handle.as_ref());
        }

        if self.gl.is_vertex_array(self.vao_handle.as_ref()) {
            self.gl.delete_vertex_array(self.vao_handle.as_ref());
        }
    }
}

pub trait VertexLayout: AsBytes + FromBytes {
    const VERTEX_LAYOUT: &'static [VertexAttribute];
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
    pub const fn new(index: usize, offset: usize, kind: VertexAttributeKind) -> Self {
        Self {
            kind,
            index,
            offset,
        }
    }
}
