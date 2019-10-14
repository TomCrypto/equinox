use crate::{Geometry, Parameter};
use std::fmt::Display;

#[derive(Debug, Default)]
pub struct GeometryGlslGenerator {
    functions: Vec<String>,
    next_function_id: u32,
}

impl GeometryGlslGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_distance_function(&mut self, geometry: &Geometry) -> DistanceFn {
        self.distance_recursive(geometry, &mut 0)
    }

    pub fn add_normal_function(&mut self, geometry: &Geometry) -> Option<NormalFn> {
        self.normal_recursive(geometry, &mut 0)
    }

    /// Generates GLSL code for a list of geometries.
    ///
    /// When a normal function is not given, a gradient estimate implementation
    /// will be inserted that should generate reasonable normals in most cases.
    pub fn generate(self, geometries: &[(DistanceFn, Option<NormalFn>)]) -> String {
        let mut code = vec![];

        for function in self.functions {
            code.push(function);
        }

        code.push(
            "bool geo_intersect(uint geometry, uint inst, ray_t ray, inout vec2 range) {"
                .to_owned(),
        );
        code.push("  switch (geometry) {".to_owned());

        for (index, (distance, _)) in geometries.iter().enumerate() {
            code.push(format!("    case {}U:", index));
            code.push("      while (range.x <= range.y) {{".to_owned());
            code.push(format!(
                "        float dist = abs({});",
                distance.call("ray.org + range.x * ray.dir")
            ));
            code.push("        if (dist < PREC * 0.1) {{ return true; }}".to_owned());
            code.push("        range.x += dist;".to_owned());
            code.push("      }}".to_owned());
            code.push("      break;".to_owned());
        }

        code.push("  }".to_owned());
        code.push("  return false;".to_owned());
        code.push("}".to_owned());

        code.push("vec3 geo_normal(uint geometry, uint inst, vec3 p) {".to_owned());
        code.push("  switch (geometry) {".to_owned());

        for (index, (distance, normal)) in geometries.iter().enumerate() {
            code.push(format!("    case {}U:", index));

            if let Some(normal) = normal {
                code.push(format!("      return {};", normal.call("p")));
            } else {
                let estimate = Self::gradient_estimate(&distance);

                code.push(format!("      return {};", estimate));
            }
        }

        code.push("    default:".to_owned());
        code.push("      return vec3(0.0);".to_owned());
        code.push("  }".to_owned());
        code.push("}".to_owned());

        format!("{}\n", code.join("\n"))
    }

    // In the methods below, the parameters must be evaluated in the same order used
    // when renumbering the parameter array further below in `renumber_parameters`.

    fn distance_recursive(&mut self, geometry: &Geometry, index: &mut usize) -> DistanceFn {
        let code = match geometry {
            Geometry::UnitSphere => "return length(p) - 1.0;".to_owned(),
            Geometry::UnitCube => r#"
                vec3 d = abs(p) - vec3(1.0);
                return length(max(d,0.0)) + min(max(d.x,max(d.y,d.z)),0.0);
            "#
            .to_owned(),
            Geometry::Plane { width, length } => {
                let _ = self.lookup_parameter(&width, index);
                let _ = self.lookup_parameter(&length, index);

                "return p.y;".to_owned()
            }
            Geometry::InfiniteRepetition { f, period } => {
                let period_x = self.lookup_parameter(&period[0], index);
                let period_y = self.lookup_parameter(&period[1], index);
                let period_z = self.lookup_parameter(&period[2], index);

                let function = self.distance_recursive(f, index);

                format!(
                    "vec3 c = vec3({}, {}, {});
                    return {};",
                    period_x,
                    period_y,
                    period_z,
                    function.call("mod(p + 0.5 * c, c) - 0.5 * c")
                )
            }
            Geometry::Union { children } => self.nary_operator(children, index, "min"),
            Geometry::Intersection { children } => self.nary_operator(children, index, "max"),
            Geometry::Subtraction { lhs, rhs } => {
                let lhs_function = self.distance_recursive(lhs, index);
                let rhs_function = self.distance_recursive(rhs, index);

                format!(
                    "return max({}, -{}",
                    lhs_function.call("p"),
                    rhs_function.call("p")
                )
            }
            Geometry::Scale { factor, f } => {
                let factor = self.lookup_parameter(factor, index);

                let function = self.distance_recursive(f, index);

                format!(
                    "float s = {}; return {} * s;",
                    factor,
                    function.call("p / s")
                )
            }
            Geometry::Translate { translation, f } => {
                let tx = self.lookup_parameter(&translation[0], index);
                let ty = self.lookup_parameter(&translation[1], index);
                let tz = self.lookup_parameter(&translation[2], index);

                let translation = format!("vec3({}, {}, {})", tx, ty, tz);

                let function = self.distance_recursive(f, index);

                format!("return {};", function.call(format!("p - {}", translation)))
            }
            Geometry::Round { radius, f } => {
                let radius = self.lookup_parameter(radius, index);

                let function = self.distance_recursive(f, index);

                format!("return {} - {};", function.call("p"), radius)
            }
        };

        self.register_distance_function(code.trim())
    }

    fn normal_recursive(&mut self, geometry: &Geometry, index: &mut usize) -> Option<NormalFn> {
        let code = match geometry {
            Geometry::UnitSphere => Some("return normalize(p);".to_owned()),
            Geometry::Plane { .. } => Some("return vec3(0.0, 1.0, 0.0);".to_owned()),
            Geometry::Translate { translation, f } => {
                let tx = self.lookup_parameter(&translation[0], index);
                let ty = self.lookup_parameter(&translation[1], index);
                let tz = self.lookup_parameter(&translation[2], index);

                let translation = format!("vec3({}, {}, {})", tx, ty, tz);

                let function = self.normal_recursive(f, index)?;

                Some(format!(
                    "return {};",
                    function.call(format!("p - {}", translation))
                ))
            }
            Geometry::Scale { factor, f } => {
                let _ = self.lookup_parameter(factor, index);

                return self.normal_recursive(f, index);
            }
            _ => None,
        };

        Some(self.register_normal_function(code?))
    }

    fn nary_operator(&mut self, children: &[Geometry], index: &mut usize, op: &str) -> String {
        assert!(!children.is_empty());

        if children.len() == 1 {
            let function = self.distance_recursive(&children[0], index);

            return format!("return {};", function.call("p"));
        }

        let mut code = String::new();

        for (i, child) in children.iter().enumerate() {
            let function = self.distance_recursive(child, index);

            if i != children.len() - 1 {
                code += &format!("{}({}, ", op, function.call("p"));
            } else {
                code += &function.call("p");
            }
        }

        for _ in 0..children.len() - 1 {
            code += ")";
        }

        format!("return {};", code)
    }

    // TODO: could make the "geometry_buffer" string a parameter possibly

    fn lookup_parameter(&self, parameter: &Parameter, index: &mut usize) -> String {
        match parameter {
            Parameter::Constant { value } => format!("{:+e}", value),
            Parameter::Symbolic { .. } => self.lookup_symbolic_parameter(index),
        }
    }

    fn lookup_symbolic_parameter(&self, index: &mut usize) -> String {
        let result = match *index % 4 {
            0 => format!("geometry_buffer.data[inst + {}U].x", *index / 4),
            1 => format!("geometry_buffer.data[inst + {}U].y", *index / 4),
            2 => format!("geometry_buffer.data[inst + {}U].z", *index / 4),
            _ => format!("geometry_buffer.data[inst + {}U].w", *index / 4),
        };

        *index += 1;

        result
    }

    fn gradient_estimate(distance: &DistanceFn) -> String {
        let x1 = distance.call("vec3(p.x + PREC, p.y, p.z)");
        let y1 = distance.call("vec3(p.x, p.y + PREC, p.z)");
        let z1 = distance.call("vec3(p.x, p.y, p.z + PREC)");
        let x2 = distance.call("vec3(p.x - PREC, p.y, p.z)");
        let y2 = distance.call("vec3(p.x, p.y - PREC, p.z)");
        let z2 = distance.call("vec3(p.x, p.y, p.z - PREC)");

        let dx = format!("{} - {}", x1, x2);
        let dy = format!("{} - {}", y1, y2);
        let dz = format!("{} - {}", z1, z2);

        format!("normalize(vec3({}, {}, {}))", dx, dy, dz)
    }

    fn register_distance_function(&mut self, body: impl Display) -> DistanceFn {
        let function = DistanceFn {
            id: self.generate_id(),
            body: body.to_string(),
        };

        self.functions.push(function.emit());

        function
    }

    fn register_normal_function(&mut self, body: impl Display) -> NormalFn {
        let function = NormalFn {
            id: self.generate_id(),
            body: body.to_string(),
        };

        self.functions.push(function.emit());

        function
    }

    fn generate_id(&mut self) -> u32 {
        self.next_function_id += 1;
        self.next_function_id
    }
}

/// Returns a vector of all symbolic parameter indices in the order they are
/// encountered in the geometry. The returned order is always deterministic.
pub(crate) fn renumber_parameters(geometry: &Geometry) -> Vec<usize> {
    let mut parameters = vec![];

    renumber_parameters_recursive(geometry, &mut parameters);

    parameters
}

fn add_parameter(parameters: &mut Vec<usize>, parameter: &Parameter) {
    if let Parameter::Symbolic { index } = parameter {
        parameters.push(*index);
    }
}

fn renumber_parameters_recursive(geometry: &Geometry, parameters: &mut Vec<usize>) {
    match geometry {
        Geometry::UnitSphere | Geometry::UnitCube => {}
        Geometry::Plane { width, length } => {
            add_parameter(parameters, width);
            add_parameter(parameters, length);
        }
        Geometry::InfiniteRepetition { f, period } => {
            add_parameter(parameters, &period[0]);
            add_parameter(parameters, &period[1]);
            add_parameter(parameters, &period[2]);

            renumber_parameters_recursive(f, parameters);
        }
        Geometry::Union { children } | Geometry::Intersection { children } => children
            .iter()
            .for_each(|child| renumber_parameters_recursive(child, parameters)),
        Geometry::Subtraction { lhs, rhs } => {
            renumber_parameters_recursive(lhs, parameters);
            renumber_parameters_recursive(rhs, parameters);
        }
        Geometry::Scale { factor, f } => {
            add_parameter(parameters, factor);

            renumber_parameters_recursive(f, parameters);
        }
        Geometry::Translate { translation, f } => {
            add_parameter(parameters, &translation[0]);
            add_parameter(parameters, &translation[1]);
            add_parameter(parameters, &translation[2]);

            renumber_parameters_recursive(f, parameters);
        }
        Geometry::Round { f, radius } => {
            add_parameter(parameters, radius);

            renumber_parameters_recursive(f, parameters);
        }
    }
}

#[derive(Clone, Debug)]
pub struct DistanceFn {
    id: u32,
    body: String,
}

impl DistanceFn {
    pub fn name(&self) -> String {
        format!("geo_distance_{}", self.id)
    }

    pub fn call(&self, point: impl Display) -> String {
        format!("{}(inst, {})", self.name(), point)
    }

    pub fn emit(&self) -> String {
        format!(
            "float {}(uint inst, vec3 p) {{ {} }}",
            self.name(),
            self.body
        )
    }
}

#[derive(Clone, Debug)]
pub struct NormalFn {
    id: u32,
    body: String,
}

impl NormalFn {
    pub fn name(&self) -> String {
        format!("geo_normal_{}", self.id)
    }

    pub fn call(&self, point: impl Display) -> String {
        format!("{}(inst, {})", self.name(), point)
    }

    pub fn emit(&self) -> String {
        format!(
            "vec3 {}(uint inst, vec3 p) {{\n{}\n}}",
            self.name(),
            self.body
        )
    }
}
