#[allow(unused_imports)]
use log::{debug, error, info, warn};

use js_sys::Error;
use regex::Regex;
use std::collections::HashMap;
use web_sys::{
    WebGl2RenderingContext as Context, WebGlBuffer, WebGlProgram, WebGlShader, WebGlTexture,
};

#[derive(Debug)]
pub struct ShaderBuilder {
    headers: HashMap<&'static str, String>,
    defines: HashMap<&'static str, String>,
    source: &'static str,
}

impl ShaderBuilder {
    pub fn new(source: &'static str) -> Self {
        Self {
            source,
            headers: HashMap::new(),
            defines: HashMap::new(),
        }
    }

    pub fn set_header(&mut self, name: &'static str, header: impl ToString) {
        self.headers.insert(name, header.to_string());
    }

    pub fn set_define(&mut self, name: &'static str, define: impl ToString) {
        self.defines.insert(name, define.to_string());
    }

    /// Returns the final GLSL shader source.
    pub fn generate_source(&self) -> String {
        let pattern = Regex::new(r#"^\s*#\s*include\s+<([[:graph:]]*)>\s*$"#).unwrap();

        let mut source = String::from("#version 300 es\nprecision highp float;\n");
        source.reserve(self.source.len()); // avoid unnecessary data reallocations

        for (name, value) in &self.defines {
            source += "#define ";
            source += name;
            source += " (";
            source += value;
            source += ")\n";
        }

        for line in self.source.lines() {
            if let Some(captures) = pattern.captures(line) {
                let header = captures.get(1).unwrap().as_str();

                if let Some(code) = self.headers.get(header) {
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

    /// Finds the real position of a GLSL source line using file/line markers.
    pub fn determine_real_position(source: &str, line: u32) -> (String, u32) {
        let pattern = Regex::new(r#"^// __POS__ ([^:]+):(\d+)$"#).unwrap();

        let lines: Vec<&str> = source.lines().collect();

        for index in (0..line).rev() {
            if let Some(captures) = pattern.captures(lines[index as usize]) {
                return (
                    captures.get(1).unwrap().as_str().to_owned(),
                    captures.get(2).unwrap().as_str().parse::<u32>().unwrap() + line - index - 1,
                );
            }
        }

        (String::from("<unknown>"), 0)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BindingPoint {
    Texture(u32),
    UniformBlock(u32),
}

#[derive(Debug)]
pub struct Shader {
    gl: Context,
    invalidated: bool,
    handle: Option<WebGlProgram>,
    vertex: ShaderBuilder,
    fragment: ShaderBuilder,

    binds: HashMap<&'static str, BindingPoint>,
}

impl Shader {
    pub fn new(
        gl: Context,
        vertex: ShaderBuilder,
        fragment: ShaderBuilder,
        binds: HashMap<&'static str, BindingPoint>,
    ) -> Self {
        Self {
            gl,
            handle: None,
            vertex,
            fragment,
            binds,
            invalidated: true,
        }
    }

    // as soon as you touch those, the handle is lost?
    // we always need to rebuild the shader inside the update, otherwise we've lost
    // the opportunity to do it unfortunately. so it has to be done inline
    // really

    pub fn vert_shader(&mut self) -> &mut ShaderBuilder {
        self.invalidated = true;
        &mut self.vertex
    }

    pub fn frag_shader(&mut self) -> &mut ShaderBuilder {
        self.invalidated = true;
        &mut self.fragment
    }

    pub fn TEMP_use_program(&self) {
        self.gl.use_program(self.handle.as_ref());
    }

    pub fn TEMP_bind_directly(&self, target: &dyn AsBindTarget, slot: &str) {
        let shader = ActiveShader {
            gl: &self.gl,
            binds: &self.binds,
        };

        shader.bind(target, slot);
    }

    pub fn bind_to_pipeline(&self, callback: impl FnOnce(ActiveShader)) {
        self.gl.use_program(self.handle.as_ref());

        callback(ActiveShader {
            gl: &self.gl,
            binds: &self.binds,
        })
    }

    pub fn invalidate(&mut self) {
        self.invalidated = true;
    }

    /// Rebuilds the shader with the current source.
    pub fn rebuild(&mut self) -> Result<(), Error> {
        if !self.invalidated {
            return Ok(());
        }

        self.gl.delete_program(self.handle.as_ref());
        self.invalidated = false; // even if we fail

        let vert = self.compile_shader(Context::VERTEX_SHADER, &self.vertex)?;
        let frag = self.compile_shader(Context::FRAGMENT_SHADER, &self.fragment)?;

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
                    BindingPoint::Texture(slot) => {
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

    fn compile_shader(
        &self,
        kind: u32,
        source: &ShaderBuilder,
    ) -> Result<Option<WebGlShader>, Error> {
        let shader = self.gl.create_shader(kind);

        if let Some(shader) = &shader {
            let glsl_source = source.generate_source();

            self.gl.shader_source(shader, &glsl_source);
            self.gl.compile_shader(shader);

            if let Some(error) = self.get_shader_build_error(shader) {
                let pattern = Regex::new(r#"0:(\d+):"#).unwrap();

                let error = pattern.replace_all(&error, |caps: &regex::Captures| {
                    let line: u32 = caps.get(1).unwrap().as_str().parse().unwrap();

                    let (file, line) = ShaderBuilder::determine_real_position(&glsl_source, line);

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
}

#[derive(Debug)]
pub struct ActiveShader<'a> {
    gl: &'a Context,
    binds: &'a HashMap<&'static str, BindingPoint>,
}

#[derive(Debug)]
pub enum BindTarget<'a> {
    UniformBuffer(Option<&'a WebGlBuffer>),
    Texture(Option<&'a WebGlTexture>),
}

pub trait AsBindTarget {
    fn bind_target(&self) -> BindTarget;
}

impl ActiveShader<'_> {
    pub fn bind(&self, target: &dyn AsBindTarget, slot: &str) {
        match target.bind_target() {
            BindTarget::UniformBuffer(handle) => self.bind_uniform_buffer(handle, slot),
            BindTarget::Texture(handle) => self.bind_texture(handle, slot),
        }
    }

    fn bind_uniform_buffer(&self, handle: Option<&WebGlBuffer>, slot: &str) {
        if let Some(&BindingPoint::UniformBlock(slot)) = self.binds.get(slot) {
            self.gl
                .bind_buffer_base(Context::UNIFORM_BUFFER, slot, handle);
        } else {
            panic!("slot '{}' does not map to a binding point", slot);
        }
    }

    fn bind_texture(&self, handle: Option<&WebGlTexture>, slot: &str) {
        if let Some(&BindingPoint::Texture(slot)) = self.binds.get(slot) {
            self.gl.active_texture(Context::TEXTURE0 + slot);
            self.gl.bind_texture(Context::TEXTURE_2D, handle);
        } else {
            panic!("slot '{}' does not map to a binding point", slot);
        }
    }
}
