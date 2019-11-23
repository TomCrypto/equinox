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
            // TODO: this may need to be another knob in the "precision" settings
            code.push("        range.x += dist * (1.0 - PREC * 10.0);".to_owned());
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
    // when renumbering the parameter table further below in `renumber_parameters`.

    fn distance_recursive(&mut self, geometry: &Geometry, index: &mut usize) -> DistanceFn {
        let code = match geometry {
            Geometry::Sphere { radius } => {
                let radius = self.lookup_parameter(radius, index);

                format!("return length(p) - {};", radius)
            }
            Geometry::Ellipsoid { radius } => {
                let radius_x = self.lookup_parameter(&radius[0], index);
                let radius_y = self.lookup_parameter(&radius[1], index);
                let radius_z = self.lookup_parameter(&radius[2], index);

                format!(
                    r#"
                vec3 r = vec3({}, {}, {});

                return (length(p / r) - 1.0) * min(min(r.x, r.y), r.z);
                "#,
                    radius_x, radius_y, radius_z
                )
            }
            Geometry::Cuboid { dimensions } => {
                let dim_x = self.lookup_parameter(&dimensions[0], index);
                let dim_y = self.lookup_parameter(&dimensions[1], index);
                let dim_z = self.lookup_parameter(&dimensions[2], index);

                format!(
                    r#"
                vec3 d = abs(p) - vec3({}, {}, {});
                return length(max(d,0.0)) + min(max(d.x,max(d.y,d.z)),0.0);
            "#,
                    dim_x, dim_y, dim_z
                )
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
                    "return max({}, -{});",
                    lhs_function.call("p"),
                    rhs_function.call("p")
                )
            }
            Geometry::Onion { thickness, f } => {
                let thickness = self.lookup_parameter(thickness, index);

                let function = self.distance_recursive(f, index);

                format!("return abs({}) - {};", function.call("p"), thickness)
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
            Geometry::Rotate { axis, angle, f } => {
                let kx = self.lookup_parameter(&axis[0], index);
                let ky = self.lookup_parameter(&axis[1], index);
                let kz = self.lookup_parameter(&axis[2], index);
                let theta = self.lookup_parameter(angle, index);

                let function = self.distance_recursive(f, index);

                format!(
                    r#"
                    vec3 k = normalize(vec3({}, {}, {}));
                    float theta = -{};
                    float cosTheta = cos(theta);
                    float sinTheta = sin(theta);

                    p = p * cosTheta + cross(k, p) * sinTheta + k * dot(k, p) * (1.0 - cosTheta);

                    return {};
                "#,
                    kx,
                    ky,
                    kz,
                    theta,
                    function.call("p")
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
            Geometry::ForceNumericalNormals { f } => {
                format!("return {};", self.distance_recursive(f, index).call("p"))
            }
        };

        self.register_distance_function(code.trim())
    }

    fn normal_recursive(&mut self, geometry: &Geometry, index: &mut usize) -> Option<NormalFn> {
        let code = match geometry {
            Geometry::Sphere { radius } => {
                let _ = self.lookup_parameter(radius, index);

                Some("return normalize(p);".to_owned())
            }
            Geometry::Ellipsoid { radius } => {
                let radius_x = self.lookup_parameter(&radius[0], index);
                let radius_y = self.lookup_parameter(&radius[1], index);
                let radius_z = self.lookup_parameter(&radius[2], index);

                Some(format!(
                    r#"
                vec3 r = vec3({}, {}, {});

                return normalize(p / (r * r));"#,
                    radius_x, radius_y, radius_z
                ))
            }
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
            Geometry::Rotate { axis, angle, f } => {
                let kx = self.lookup_parameter(&axis[0], index);
                let ky = self.lookup_parameter(&axis[1], index);
                let kz = self.lookup_parameter(&axis[2], index);
                let theta = self.lookup_parameter(angle, index);

                let function = self.normal_recursive(f, index)?;

                Some(format!(
                    r#"
                    vec3 k = normalize(vec3({}, {}, {}));
                    float theta = -{};
                    float cosTheta = cos(theta);
                    float sinTheta = sin(theta);

                    p = p * cosTheta + cross(k, p) * sinTheta + k * dot(k, p) * (1.0 - cosTheta);
                    vec3 n = {};

                    return n * cosTheta - cross(k, n) * sinTheta + k * dot(k, n) * (1.0 - cosTheta);
                "#,
                    kx,
                    ky,
                    kz,
                    theta,
                    function.call("p")
                ))
            }
            Geometry::Scale { factor, f } => {
                let scale = self.lookup_parameter(factor, index);

                let function = self.normal_recursive(f, index)?;

                Some(format!(
                    "return {};",
                    function.call(format!("p / {}", scale))
                ))
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
            Parameter::Constant(number) => format!("{:+e}", number),
            Parameter::Symbolic(_) => self.lookup_symbolic_parameter(index),
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

/// Returns a vector of all symbolic parameter values in the order they are
/// encountered in the geometry. The resulting order will be deterministic.
pub(crate) fn renumber_parameters(geometry: &Geometry) -> Vec<String> {
    let mut parameters = vec![];

    renumber_parameters_recursive(geometry, &mut parameters);

    parameters
}

fn add_parameter(parameters: &mut Vec<String>, parameter: &Parameter) {
    if let Parameter::Symbolic(symbol) = parameter {
        parameters.push(symbol.clone());
    }
}

fn renumber_parameters_recursive(geometry: &Geometry, parameters: &mut Vec<String>) {
    match geometry {
        Geometry::Sphere { radius } => {
            add_parameter(parameters, radius);
        }
        Geometry::Ellipsoid { radius } => {
            add_parameter(parameters, &radius[0]);
            add_parameter(parameters, &radius[1]);
            add_parameter(parameters, &radius[2]);
        }
        Geometry::Cuboid { dimensions } => {
            add_parameter(parameters, &dimensions[0]);
            add_parameter(parameters, &dimensions[1]);
            add_parameter(parameters, &dimensions[2]);
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
        Geometry::Onion { thickness, f } => {
            add_parameter(parameters, thickness);

            renumber_parameters_recursive(f, parameters);
        }
        Geometry::Scale { factor, f } => {
            add_parameter(parameters, factor);

            renumber_parameters_recursive(f, parameters);
        }
        Geometry::Rotate { axis, angle, f } => {
            add_parameter(parameters, &axis[0]);
            add_parameter(parameters, &axis[1]);
            add_parameter(parameters, &axis[2]);
            add_parameter(parameters, angle);

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
        Geometry::ForceNumericalNormals { f } => {
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

    fn name(&self) -> String {
        format!("geo_distance_{}", self.id)
    }
}

#[derive(Clone, Debug)]
pub struct NormalFn {
    id: u32,
    body: String,
}

impl NormalFn {
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

    fn name(&self) -> String {
        format!("geo_normal_{}", self.id)
    }
}
