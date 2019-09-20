#[allow(unused_imports)]
use log::{debug, info, warn};

use js_sys::Error;
use std::collections::HashMap;
use std::fmt::Write;
use web_sys::{
    WebGl2RenderingContext as Context, WebGlBuffer, WebGlProgram, WebGlShader, WebGlTexture,
};

pub struct ShaderInput {
    source: String,
}

impl ShaderInput {
    pub fn new(source: &'static str) -> Self {
        Self::with_defines(source, HashMap::new())
    }

    pub fn with_defines(source: &'static str, defines: HashMap<&'static str, String>) -> Self {
        Self {
            source: Self::process_source(source, defines),
        }
    }

    /// Finds the file/line position in the source before preprocessing.
    pub fn determine_true_position(&self, line: u32) -> (String, u32) {
        let pattern = regex::Regex::new(r#"^// __POS__ ([^:]+):(\d+)$"#).unwrap();

        let lines: Vec<&str> = self.source.lines().collect();

        for index in (0..=line).rev() {
            if let Some(captures) = pattern.captures(lines[index as usize]) {
                return (
                    captures.get(1).unwrap().as_str().to_owned(),
                    captures.get(2).unwrap().as_str().parse::<u32>().unwrap() + line - index - 1,
                );
            }
        }

        (String::from("<unknown>"), 0)
    }

    fn process_source(source: &'static str, defines: HashMap<&'static str, String>) -> String {
        let mut header = String::from("#version 300 es\n");

        for (k, v) in defines {
            write!(header, "#define {} {}\n", k, v).unwrap();
        }

        header + source
    }
}

#[derive(Clone, Copy)]
pub enum BindingPoint {
    Texture(u32),
    UniformBlock(u32),
}

pub struct Shader {
    gl: Context,
    handle: Option<WebGlProgram>,
    vertex: ShaderInput,
    fragment: ShaderInput,

    binds: HashMap<&'static str, BindingPoint>,
}

impl Shader {
    pub fn new(
        gl: Context,
        vertex: ShaderInput,
        fragment: ShaderInput,
        binds: HashMap<&'static str, BindingPoint>,
    ) -> Self {
        Self {
            gl,
            handle: None,
            vertex,
            fragment,
            binds,
        }
    }

    pub fn bind_to_pipeline(&self) -> ActiveShader {
        self.gl.use_program(self.handle.as_ref());

        ActiveShader {
            gl: &self.gl,
            binds: &self.binds,
        }
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

    fn compile_shader(&self, kind: u32, input: &ShaderInput) -> Result<Option<WebGlShader>, Error> {
        let pattern = regex::Regex::new(r#"0:(\d+):"#).unwrap();

        let shader = self.gl.create_shader(kind);

        if let Some(shader) = &shader {
            self.gl.shader_source(shader, &input.source);
            self.gl.compile_shader(shader);

            if let Some(error) = self.get_shader_build_error(shader) {
                let error = pattern.replace_all(&error, |caps: &regex::Captures| {
                    let line: u32 = caps.get(1).unwrap().as_str().parse().unwrap();

                    let (file, line) = input.determine_true_position(line);

                    format!("{}:{}:", file, line)
                });

                return Err(Error::new(&error));
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

pub enum ShaderBindHandle<'a> {
    UniformBuffer(Option<&'a WebGlBuffer>),
    Texture(Option<&'a WebGlTexture>),
}

pub trait ShaderBind {
    fn handle(&self) -> ShaderBindHandle;
}

impl ActiveShader<'_> {
    pub fn bind(&self, target: &dyn ShaderBind, slot: &str) {
        match target.handle() {
            ShaderBindHandle::UniformBuffer(handle) => self.bind_uniform_buffer(handle, slot),
            ShaderBindHandle::Texture(handle) => self.bind_texture(handle, slot),
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
