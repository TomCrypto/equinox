#[allow(unused_imports)]
use log::{debug, error, info, warn};

use crate::{shader::ShaderInfo, Framebuffer};
use js_sys::Error;
use regex::Regex;
use std::collections::HashMap;
use web_sys::{
    WebGl2RenderingContext as Context, WebGlBuffer, WebGlProgram, WebGlShader, WebGlTexture,
    WebGlVertexArrayObject,
};

#[derive(Clone, Copy, Debug)]
pub enum BindingPoint {
    TextureUnit(u32),
    UniformBlock(u32),
}

#[derive(Debug)]
pub struct Shader {
    gl: Context,
    invalidated: bool,
    handle: Option<WebGlProgram>,
    vertex: &'static ShaderInfo,
    fragment: &'static ShaderInfo,

    binds: HashMap<&'static str, BindingPoint>,

    headers: HashMap<&'static str, String>,
    defines: HashMap<&'static str, String>,
}

fn merge_sort_dedup(lhs: &[&'static str], rhs: &[&'static str]) -> Vec<&'static str> {
    let mut vec = Vec::with_capacity(lhs.len() + rhs.len());

    vec.extend_from_slice(lhs);
    vec.extend_from_slice(rhs);
    vec.sort_unstable();
    vec.dedup();
    vec
}

impl Shader {
    pub fn new(gl: Context, vertex: &'static ShaderInfo, fragment: &'static ShaderInfo) -> Self {
        let mut headers = HashMap::new();
        let mut defines = HashMap::new();

        for key in merge_sort_dedup(vertex.defines, fragment.defines) {
            defines.insert(key, String::new());
        }

        for key in merge_sort_dedup(vertex.headers, fragment.headers) {
            headers.insert(key, String::new());
        }

        let mut binds = HashMap::new();

        let uniform_blocks = merge_sort_dedup(vertex.uniform_blocks, fragment.uniform_blocks);
        let texture_units = merge_sort_dedup(vertex.texture_units, fragment.texture_units);

        for (index, &key) in uniform_blocks.iter().enumerate() {
            binds.insert(key, BindingPoint::UniformBlock(index as u32));
        }

        for (index, &key) in texture_units.iter().enumerate() {
            binds.insert(key, BindingPoint::TextureUnit(index as u32));
        }

        Self {
            gl,
            handle: None,
            vertex,
            fragment,
            binds,
            headers,
            defines,
            invalidated: true,
        }
    }

    pub fn set_header(&mut self, header: &'static str, value: impl ToString) {
        assert!(self.headers.contains_key(header));

        if self.headers.get(header) != Some(&value.to_string()) {
            self.headers.insert(header, value.to_string());
            self.invalidated = true;
        }
    }

    pub fn set_define(&mut self, define: &'static str, value: impl ToString) {
        assert!(self.defines.contains_key(define));

        if self.defines.get(define) != Some(&value.to_string()) {
            self.defines.insert(define, value.to_string());
            self.invalidated = true;
        }
    }

    pub fn begin_draw(&self) -> DrawCommand {
        DrawCommand::new(self)
    }

    pub fn invalidate(&mut self) {
        self.invalidated = true;
        self.handle = None;
    }

    /// Rebuilds the shader with the current source.
    pub fn rebuild(&mut self) -> Result<(), Error> {
        if !self.invalidated {
            return Ok(());
        }

        if let Some(handle) = &self.handle {
            self.gl.delete_program(Some(handle));
        }

        self.invalidated = false;

        let vert = self.compile_shader(Context::VERTEX_SHADER, self.vertex.code)?;
        let frag = self.compile_shader(Context::FRAGMENT_SHADER, self.fragment.code)?;

        if let (Some(vert), Some(frag)) = (&vert, &frag) {
            self.handle = self.link_program(vert, frag)?;
            self.configure_binds(); // initialize shader
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
                    BindingPoint::TextureUnit(slot) => {
                        let location = self.gl.get_uniform_location(program, name);

                        if let Some(location) = location {
                            self.gl.uniform1i(Some(&location), slot as i32);
                        } else {
                            warn!("no such shader binding point: {}", name);
                        }
                    }
                    BindingPoint::UniformBlock(slot) => {
                        let index = self.gl.get_uniform_block_index(program, name);

                        if index != Context::INVALID_INDEX {
                            self.gl.uniform_block_binding(program, index, slot as u32);
                        } else {
                            warn!("no such shader binding point: {}", name);
                        }
                    }
                }
            }
        }
    }

    fn compile_shader(&self, kind: u32, source: &str) -> Result<Option<WebGlShader>, Error> {
        let shader = self.gl.create_shader(kind);

        if let Some(shader) = &shader {
            let glsl_source = Self::generate_source(source, &self.headers, &self.defines);

            self.gl.shader_source(shader, &glsl_source);
            self.gl.compile_shader(shader);

            if let Some(error) = self.get_shader_build_error(shader) {
                let pattern = Regex::new(r#"0:([0-9]+):"#).unwrap();

                let error = pattern.replace_all(&error, |caps: &regex::Captures| {
                    let line: u32 = caps.get(1).unwrap().as_str().parse().unwrap();

                    let (file, line) = Self::determine_real_position(&glsl_source, line);

                    format!("{}:{}:", file, line)
                });

                error!("{}", error);
                return Err(Error::new("failed to compile shader source"));
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

            self.gl.delete_shader(Some(vert));
            self.gl.delete_shader(Some(frag));

            if let Some(error) = self.get_program_link_error(program) {
                error!("{}", error);
                return Err(Error::new("failed to link shader program"));
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

    fn generate_source(
        glsl_source: &str,
        headers: &HashMap<&'static str, String>,
        defines: &HashMap<&'static str, String>,
    ) -> String {
        let pattern = Regex::new(r#"^#include <([[:graph:]]*)>$"#).unwrap();

        let mut source = String::from(
            r#"#version 300 es
            precision highp float;
            precision highp sampler2DArray;
        "#,
        );
        source.reserve(glsl_source.len());

        for (name, value) in defines {
            source += "#define ";
            source += name;
            source += " (";
            source += value;
            source += ")\n";
        }

        for line in glsl_source.lines() {
            if let Some(captures) = pattern.captures(line) {
                let header = captures.get(1).unwrap().as_str();

                if let Some(code) = headers.get(header) {
                    source += code;
                    source += "\n";
                    continue;
                }
            }

            source += line;
            source += "\n";
        }

        source
    }

    /// Finds the position of a GLSL source line through file/line markers.
    fn determine_real_position(source: &str, line: u32) -> (String, u32) {
        let pattern = Regex::new(r#"^// __POS__ ([^:]+):([0-9]+)$"#).unwrap();

        let lines: Vec<&str> = source.lines().collect();

        for index in (0..line).rev() {
            if let Some(captures) = pattern.captures(lines[index as usize]) {
                return (
                    captures.get(1).unwrap().as_str().to_owned(),
                    captures.get(2).unwrap().as_str().parse::<u32>().unwrap() + line - index - 2,
                );
            }
        }

        (String::from("<unknown>"), 0)
    }
}

#[derive(Debug)]
pub struct DrawCommand<'a> {
    shader: &'a Shader,
}

#[derive(Debug)]
pub enum BindTarget<'a> {
    UniformBuffer(Option<&'a WebGlBuffer>),
    Texture(Option<&'a WebGlTexture>, bool),
}

pub trait AsBindTarget {
    fn bind_target(&self) -> BindTarget;
}

pub trait AsVertexArray {
    fn vertex_array(&self) -> Option<&WebGlVertexArrayObject>;
}

impl<'a> DrawCommand<'a> {
    fn new(shader: &'a Shader) -> Self {
        shader.gl.use_program(shader.handle.as_ref());

        shader.gl.disable(Context::BLEND);
        shader.gl.disable(Context::DEPTH_TEST);
        shader.gl.disable(Context::SCISSOR_TEST);
        shader.gl.disable(Context::STENCIL_TEST);
        shader.gl.viewport(0, 0, 0, 0);

        Self { shader }
    }

    pub fn bind(&self, target: &dyn AsBindTarget, slot: &str) {
        match target.bind_target() {
            BindTarget::UniformBuffer(handle) => self.bind_uniform_buffer(handle, slot),
            BindTarget::Texture(handle, array) => self.bind_texture(handle, slot, array),
        }
    }

    pub fn set_viewport(&self, x: i32, y: i32, w: i32, h: i32) {
        self.shader.gl.viewport(x, y, w, h);
    }

    pub fn set_scissor(&self, x: i32, y: i32, w: i32, h: i32) {
        self.shader.gl.enable(Context::SCISSOR_TEST);
        self.shader.gl.scissor(x, y, w, h);
    }

    pub fn unset_scissor(&self) {
        self.shader.gl.disable(Context::SCISSOR_TEST);
    }

    pub fn set_blend_mode(&self, mode: BlendMode) {
        self.shader.gl.enable(Context::BLEND);

        match mode {
            BlendMode::Accumulate { weight } => {
                self.shader.gl.blend_equation(Context::FUNC_ADD);
                self.shader
                    .gl
                    .blend_func(Context::CONSTANT_ALPHA, Context::ONE_MINUS_CONSTANT_ALPHA);
                self.shader.gl.blend_color(0.0, 0.0, 0.0, weight);
            }
            BlendMode::Add => {
                self.shader.gl.blend_equation(Context::FUNC_ADD);
                self.shader.gl.blend_func(Context::ONE, Context::ONE);
            }
            BlendMode::AlphaPredicatedAdd => {
                self.shader.gl.blend_equation(Context::FUNC_ADD);
                self.shader.gl.blend_func(Context::ONE, Context::SRC_ALPHA);
            }
        }
    }

    pub fn unset_blend_mode(&self) {
        self.shader.gl.disable(Context::BLEND);
    }

    pub fn set_vertex_array(&self, target: &dyn AsVertexArray) {
        self.shader.gl.bind_vertex_array(target.vertex_array());
    }

    pub fn unset_vertex_array(&self) {
        self.shader.gl.bind_vertex_array(None);
    }

    pub fn set_framebuffer(&self, target: &Framebuffer) {
        self.shader
            .gl
            .bind_framebuffer(Context::DRAW_FRAMEBUFFER, target.handle());
    }

    pub fn set_canvas_framebuffer(&self) {
        self.shader
            .gl
            .bind_framebuffer(Context::DRAW_FRAMEBUFFER, None);
    }

    pub fn draw_triangles(&self, index: usize, triangles: usize) {
        self.shader
            .gl
            .draw_arrays(Context::TRIANGLES, 3 * index as i32, 3 * triangles as i32);
    }

    pub fn draw_points(&self, index: usize, points: usize) {
        self.shader
            .gl
            .draw_arrays(Context::POINTS, index as i32, points as i32);
    }

    pub fn set_uniform_ivec2(&self, name: &str, x: i32, y: i32) {
        if let Some(program) = &self.shader.handle {
            let location = self.shader.gl.get_uniform_location(program, name);
            self.shader.gl.uniform2i(location.as_ref(), x, y);
        }
    }

    fn bind_uniform_buffer(&self, handle: Option<&WebGlBuffer>, slot: &str) {
        if let Some(&BindingPoint::UniformBlock(slot)) = self.shader.binds.get(slot) {
            self.shader
                .gl
                .bind_buffer_base(Context::UNIFORM_BUFFER, slot, handle);
        } else {
            panic!("slot '{}' does not map to a binding point", slot);
        }
    }

    fn bind_texture(&self, handle: Option<&WebGlTexture>, slot: &str, array: bool) {
        if let Some(&BindingPoint::TextureUnit(slot)) = self.shader.binds.get(slot) {
            self.shader.gl.active_texture(Context::TEXTURE0 + slot);

            if array {
                self.shader
                    .gl
                    .bind_texture(Context::TEXTURE_2D_ARRAY, handle);
            } else {
                self.shader.gl.bind_texture(Context::TEXTURE_2D, handle);
            }
        } else {
            panic!("slot '{}' does not map to a binding point", slot);
        }
    }
}

pub enum BlendMode {
    Accumulate { weight: f32 },
    Add,
    AlphaPredicatedAdd,
}
