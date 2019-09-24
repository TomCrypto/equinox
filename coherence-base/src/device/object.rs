use crate::model::Geometry;

fn emit_function(code: &mut String, index: &mut usize, body: &str) -> String {
    let name = format!("sdf{}", *index);
    *index += 1;

    *code += &format!("float {}(vec3 x, uint inst) {{ {} }}", name, body);

    name
}

fn parameter_access(index: &mut usize) -> String {
    let array_idx = *index / 4;

    let result = match *index % 4 {
        0 => format!("geometry_buffer.data[inst + {}U].x", array_idx),
        1 => format!("geometry_buffer.data[inst + {}U].y", array_idx),
        2 => format!("geometry_buffer.data[inst + {}U].z", array_idx),
        _ => format!("geometry_buffer.data[inst + {}U].w", array_idx),
    };

    *index += 1;

    result
}

// assume the parameters come from a vec4 array called GEOMETRY_PARAMS
// TODO: this logic really needs to be moved to the renderer!!

impl Geometry {
    pub fn as_glsl_function(
        &self,
        code: &mut String,
        index: &mut usize,
        parameter_index: &mut usize,
    ) -> String {
        match self {
            Self::UnitSphere => emit_function(code, index, "return length(x) - 1.0;"),
            Self::UnitCube => emit_function(
                code,
                index,
                r#"vec3 d = abs(x) - vec3(1.0); return length(max(d,0.0)) + min(max(d.x,max(d.y,d.z)),0.0);"#,
            ),
            Self::Union { children } => {
                let name1 = children[0].as_glsl_function(code, index, parameter_index);
                let name2 = children[1].as_glsl_function(code, index, parameter_index);

                emit_function(
                    code,
                    index,
                    &format!("return min({}(x, inst), {}(x, inst));", name1, name2),
                )
            }
            Self::Intersection { children } => {
                // TODO: only support 2 children for now...
                let name1 = children[0].as_glsl_function(code, index, parameter_index);
                let name2 = children[1].as_glsl_function(code, index, parameter_index);

                emit_function(
                    code,
                    index,
                    &format!("return max({}(x, inst), {}(x, inst));", name1, name2),
                )
            }
            Self::Scale { factor, f } => {
                let factor_code = match factor {
                    Parameter::Constant(value) => format!("{:+e}", value),
                    Parameter::Symbolic(_) => parameter_access(parameter_index),
                };

                let name = f.as_glsl_function(code, index, parameter_index);

                emit_function(
                    code,
                    index,
                    &format!("float s = {}; return {}(x / s, inst) * s;", factor_code, name,),
                )
            }
            Self::Translate { translation, f } => {
                let tx = match translation[0] {
                    Parameter::Constant(value) => format!("{:+e}", value),
                    Parameter::Symbolic(_) => parameter_access(parameter_index),
                };

                let ty = match translation[1] {
                    Parameter::Constant(value) => format!("{:+e}", value),
                    Parameter::Symbolic(_) => parameter_access(parameter_index),
                };

                let tz = match translation[2] {
                    Parameter::Constant(value) => format!("{:+e}", value),
                    Parameter::Symbolic(_) => parameter_access(parameter_index),
                };

                let translation_code = format!("vec3({}, {}, {})", tx, ty, tz);

                let name = f.as_glsl_function(code, index, parameter_index);

                emit_function(
                    code,
                    index,
                    &format!("return {}(x - {}, inst);", name, translation_code),
                )
            },
            Self::Round { radius, f } => {
                let radius_code = match radius {
                    Parameter::Constant(value) => format!("{:+e}", value),
                    Parameter::Symbolic(_) => parameter_access(parameter_index),
                };

                let name = f.as_glsl_function(code, index, parameter_index);

                emit_function(
                    code,
                    index,
                    &format!("return {}(x, inst) - {};", name, radius_code),
                )
            }
        }
    }
}

use crate::model::Parameter;

/*

switch (index) {
    case 0:


}

design it like this: each invocation will generate a new appropriate GLSL function recursively
and then simply return its name.

*/
