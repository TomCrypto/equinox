#[allow(unused_imports)]
use log::{debug, info, warn};

use js_sys::Error;
use std::collections::HashMap;
use web_sys::{WebGl2RenderingContext as Context, WebGlProgram, WebGlShader};

use crate::{RenderTexture, TextureBuffer, UniformBuffer};

#[derive(Clone, Copy)]
pub enum BindingPoint {
    Texture(u32),
    UniformBlock(u32),
}

pub struct Shader {
    gl: Context,
    handle: Option<WebGlProgram>,
    vertex: String,
    fragment: String,

    binds: HashMap<&'static str, BindingPoint>,
}

impl Shader {
    pub fn new(
        gl: Context,
        vertex: String,
        fragment: String,
        binds: HashMap<&'static str, BindingPoint>,
    ) -> Self {
        Self {
            gl,
            handle: None,
            vertex: ["#version 300 es", &vertex].join("\n"),
            fragment: ["#version 300 es", "precision highp float;", &fragment].join("\n"),
            binds,
        }
    }

    pub fn bind_to_pipeline(&self) -> ActiveShader {
        self.gl.use_program(self.resource());

        ActiveShader {
            gl: &self.gl,
            binds: &self.binds,
        }
    }

    pub(crate) fn resource(&self) -> Option<&WebGlProgram> {
        self.handle.as_ref()
    }

    pub(crate) fn reset(&mut self) -> Result<(), Error> {
        let vert = self.compile_shader(Context::VERTEX_SHADER, &self.vertex)?;
        let frag = self.compile_shader(Context::FRAGMENT_SHADER, &self.fragment)?;

        if let (Some(vert), Some(frag)) = (&vert, &frag) {
            self.handle = self.link_program(vert, frag)?;
            self.configure_binds(); // prepare the shader
        } else {
            self.handle = None;
        }

        Ok(())
    }

    fn configure_binds(&self) {
        if let Some(program) = &self.handle {
            self.gl.use_program(Some(program));

            for (&name, &binding_point) in &self.binds {
                match binding_point {
                    BindingPoint::Texture(slot) => {
                        let location = self.gl.get_uniform_location(program, name);

                        if let Some(location) = location {
                            self.gl.uniform1i(Some(&location), slot as i32);
                        } else {
                            panic!("no such binding point in shader");
                        }
                    }
                    BindingPoint::UniformBlock(slot) => {
                        let index = self.gl.get_uniform_block_index(program, name);

                        if index != Context::INVALID_INDEX {
                            self.gl.uniform_block_binding(program, index, slot as u32);
                        } else {
                            panic!("no such binding point in shader");
                        }
                    }
                }
            }
        }
    }

    fn compile_shader(&self, kind: u32, source: &str) -> Result<Option<WebGlShader>, Error> {
        let shader = self.gl.create_shader(kind);

        if let Some(shader) = &shader {
            self.gl.shader_source(shader, source);
            self.gl.compile_shader(shader);

            if let Some(error) = self.get_shader_build_error(shader) {
                return Err(Error::new(&format!("shader build error: {}", error)));
            }
        }

        Ok(shader)
    }

    fn link_program(
        &self,
        vert: &WebGlShader,
        frag: &WebGlShader,
    ) -> Result<Option<WebGlProgram>, Error> {
        let program = self.gl.create_program();

        if let Some(program) = &program {
            self.gl.attach_shader(program, vert);
            self.gl.attach_shader(program, frag);
            self.gl.link_program(program);

            if let Some(error) = self.get_program_link_error(program) {
                return Err(Error::new(&format!("program link error: {}", error)));
            }
        }

        Ok(program)
    }

    fn get_shader_build_error(&self, shader: &WebGlShader) -> Option<String> {
        if self.gl.is_context_lost() {
            return None;
        }

        let status = self
            .gl
            .get_shader_parameter(shader, Context::COMPILE_STATUS);

        if status.as_bool().unwrap_or(false) {
            return None;
        }

        if let Some(error) = self.gl.get_shader_info_log(shader) {
            Some(error)
        } else {
            Some(String::from("unknown shader building error"))
        }
    }

    fn get_program_link_error(&self, program: &WebGlProgram) -> Option<String> {
        if self.gl.is_context_lost() {
            return None;
        }

        let status = self.gl.get_program_parameter(program, Context::LINK_STATUS);

        if status.as_bool().unwrap_or(false) {
            return None;
        }

        if let Some(error) = self.gl.get_program_info_log(program) {
            Some(error)
        } else {
            Some(String::from("unknown program linking error"))
        }
    }
}

pub struct ActiveShader<'a> {
    gl: &'a Context,
    binds: &'a HashMap<&'static str, BindingPoint>,
}

impl ActiveShader<'_> {
    pub fn bind_uniform_buffer(&self, resource: &UniformBuffer, slot: &str) {
        if let Some(&BindingPoint::UniformBlock(slot)) = self.binds.get(slot) {
            self.gl
                .bind_buffer_base(Context::UNIFORM_BUFFER, slot, resource.resource());
        } else {
            panic!("slot '{}' does not map to a binding point", slot);
        }
    }

    pub fn bind_texture_buffer(&self, resource: &TextureBuffer, slot: &str) {
        if let Some(&BindingPoint::Texture(slot)) = self.binds.get(slot) {
            self.gl.active_texture(Context::TEXTURE0 + slot);
            self.gl
                .bind_texture(Context::TEXTURE_2D, resource.resource());
        } else {
            panic!("slot '{}' does not map to a binding point", slot);
        }
    }

    pub fn bind_render_texture(&self, resource: &RenderTexture, slot: &str) {
        if let Some(&BindingPoint::Texture(slot)) = self.binds.get(slot) {
            self.gl.active_texture(Context::TEXTURE0 + slot as u32);
            self.gl
                .bind_texture(Context::TEXTURE_2D, resource.resource());
        } else {
            panic!("slot '{}' does not map to a binding point", slot);
        }
    }
}
