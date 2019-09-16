#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::ScratchMemory;

use sigma_core::DeviceBuffer;

use web_sys::{WebGl2RenderingContext as Context, WebGlBuffer};

use std::cell::RefCell;
use std::rc::Rc;

pub struct UniformBuffer {
    gl: Context,
    scratch: Rc<RefCell<ScratchMemory>>,

    handle: Option<WebGlBuffer>,
    size: usize,

    fixed_size: Option<usize>,
}

impl UniformBuffer {
    pub fn new(gl: Context, scratch: Rc<RefCell<ScratchMemory>>) -> Self {
        Self {
            gl,
            scratch,
            handle: None,
            size: 0,
            fixed_size: None,
        }
    }

    pub fn with_fixed_size(
        gl: Context,
        scratch: Rc<RefCell<ScratchMemory>>,
        fixed_size: usize,
    ) -> Self {
        Self {
            gl,
            scratch,
            handle: None,
            size: 0,
            fixed_size: Some(fixed_size),
        }
    }

    pub(crate) fn resource(&self) -> Option<&WebGlBuffer> {
        self.handle.as_ref()
    }

    pub(crate) fn reset(&mut self) {
        self.handle = self.gl.create_buffer();

        if let Some(fixed_size) = self.fixed_size {
            self.allocate_buffer(fixed_size);
        } else {
            self.size = 0;
        }
    }

    fn allocate_buffer(&mut self, size: usize) {
        self.gl
            .bind_buffer(Context::UNIFORM_BUFFER, self.resource());
        self.gl
            .buffer_data_with_i32(Context::UNIFORM_BUFFER, size as i32, Context::DYNAMIC_DRAW);

        self.size = size;
    }

    fn map_update_dynamic(&mut self, size: usize, f: impl FnOnce(&mut [u8])) {
        if self.size < size {
            self.allocate_buffer(size);
        }

        let mut memory = self.scratch.borrow_mut();

        let buffer = memory.access_with_size(size);

        f(buffer);

        self.gl
            .bind_buffer(Context::UNIFORM_BUFFER, self.resource());
        self.gl
            .buffer_sub_data_with_i32_and_u8_array(Context::UNIFORM_BUFFER, 0, buffer);
    }

    fn map_update_fixed(&mut self, size: usize, f: impl FnOnce(&mut [u8])) {
        if self.size < size {
            panic!("uniform buffer does not have enough capacity");
        }

        let mut memory = self.scratch.borrow_mut();

        let buffer = memory.access_with_size(self.size);

        f(&mut buffer[..size]);

        self.gl
            .bind_buffer(Context::UNIFORM_BUFFER, self.resource());
        self.gl
            .buffer_sub_data_with_i32_and_u8_array(Context::UNIFORM_BUFFER, 0, buffer);
    }
}

impl DeviceBuffer for UniformBuffer {
    fn map_update(&mut self, size: usize, f: impl FnOnce(&mut [u8])) {
        if self.fixed_size.is_some() {
            self.map_update_fixed(size, f);
        } else {
            self.map_update_dynamic(size, f);
        }
    }
}
