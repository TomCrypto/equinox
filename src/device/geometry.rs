use crate::{Geometry, GeometryParameter};
use std::collections::HashMap;
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
        self.distance_recursive(geometry, &Self::build_parameter_map(geometry))
    }

    pub fn add_normal_function(&mut self, geometry: &Geometry) -> Option<NormalFn> {
        self.normal_recursive(geometry, &Self::build_parameter_map(geometry))
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
            code.push("        if (dist < PREC) {{ return true; }}".to_owned());
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

    fn build_parameter_map(geometry: &Geometry) -> HashMap<&str, usize> {
        // The +8 offset here is to accommodate the medium information which
        // currently uses the first two parameter blocks for every instance.

        geometry
            .symbolic_parameters()
            .iter()
            .enumerate()
            .map(|(index, &symbol)| (symbol, 8 + index))
            .collect()
    }

    fn distance_recursive(
        &mut self,
        geometry: &Geometry,
        parameters: &HashMap<&str, usize>,
    ) -> DistanceFn {
        let code = match geometry {
            Geometry::Sphere { radius } => {
                let radius = self.lookup_parameter(radius, parameters);

                format!("return length(p) - {};", radius)
            }
            Geometry::Ellipsoid { radius } => {
                let radius_x = self.lookup_parameter(&radius[0], parameters);
                let radius_y = self.lookup_parameter(&radius[1], parameters);
                let radius_z = self.lookup_parameter(&radius[2], parameters);

                format!(
                    r#"
                vec3 r = vec3({}, {}, {});

                return (length(p / r) - 1.0) * min(min(r.x, r.y), r.z);
                "#,
                    radius_x, radius_y, radius_z
                )
            }
            Geometry::Cuboid { dimensions } => {
                let dim_x = self.lookup_parameter(&dimensions[0], parameters);
                let dim_y = self.lookup_parameter(&dimensions[1], parameters);
                let dim_z = self.lookup_parameter(&dimensions[2], parameters);

                format!(
                    r#"
                vec3 d = abs(p) - vec3({}, {}, {});
                return length(max(d, 0.0)) + min(max(d.x, max(d.y, d.z)), 0.0);
            "#,
                    dim_x, dim_y, dim_z
                )
            }
            Geometry::Cylinder { height, radius } => {
                let height = self.lookup_parameter(height, parameters);
                let radius = self.lookup_parameter(radius, parameters);

                format!(
                    r#"
                    vec2 d = abs(vec2(length(p.xz), p.y)) - vec2({}, {});
                    return min(max(d.x, d.y), 0.0) + length(max(d, 0.0));
                    "#,
                    radius, height
                )
            }
            Geometry::Union { children } => self.nary_operator(children, parameters, "min"),
            Geometry::Intersection { children } => self.nary_operator(children, parameters, "max"),
            Geometry::Subtraction { lhs, rhs } => {
                let lhs_function = self.distance_recursive(lhs, parameters);
                let rhs_function = self.distance_recursive(rhs, parameters);

                format!(
                    "return max({}, -{});",
                    lhs_function.call("p"),
                    rhs_function.call("p")
                )
            }
            Geometry::Onion { thickness, child } => {
                let thickness = self.lookup_parameter(thickness, parameters);

                let function = self.distance_recursive(child, parameters);

                format!("return abs({}) - {};", function.call("p"), thickness)
            }
            Geometry::Scale { factor, child } => {
                let factor = self.lookup_parameter(factor, parameters);

                let function = self.distance_recursive(child, parameters);

                format!(
                    "float s = {}; return {} * s;",
                    factor,
                    function.call("p / s")
                )
            }
            Geometry::Rotate { axis, angle, child } => {
                let kx = self.lookup_parameter(&axis[0], parameters);
                let ky = self.lookup_parameter(&axis[1], parameters);
                let kz = self.lookup_parameter(&axis[2], parameters);
                let theta = self.lookup_parameter(angle, parameters);

                let function = self.distance_recursive(child, parameters);

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
            Geometry::Translate { translation, child } => {
                let tx = self.lookup_parameter(&translation[0], parameters);
                let ty = self.lookup_parameter(&translation[1], parameters);
                let tz = self.lookup_parameter(&translation[2], parameters);

                let translation = format!("vec3({}, {}, {})", tx, ty, tz);

                let function = self.distance_recursive(child, parameters);

                format!("return {};", function.call(format!("p - {}", translation)))
            }
            Geometry::Round { radius, child } => {
                let radius = self.lookup_parameter(radius, parameters);

                let function = self.distance_recursive(child, parameters);

                format!("return {} - {};", function.call("p"), radius)
            }
            Geometry::ForceNumericalNormals { child } => format!(
                "return {};",
                self.distance_recursive(child, parameters).call("p")
            ),
            Geometry::CustomModifier { child, code, .. } => {
                let function = self.distance_recursive(child, parameters);

                code.replace("f(", &format!("{}(", function.name()))
            }
            Geometry::Twist {
                amount,
                step,
                child,
            } => {
                let amount = self.lookup_parameter(amount, parameters);
                let step = self.lookup_parameter(step, parameters);

                let function = self.distance_recursive(child, parameters);

                format!(
                    r#"
                    float k = {} / sqrt(p.x * p.x + p.z * p.z);
                    float c = cos(2.0 * 3.14159265 * k * p.y);
                    float s = sin(2.0 * 3.14159265 * k * p.y);
                    vec3 q = vec3(p.x * c + p.z * s, -p.x * s + p.z * c, p.y);
                    return {} * {};
                "#,
                    amount,
                    function.call("q"),
                    step,
                )
            }
        };

        self.register_distance_function(code.trim())
    }

    fn normal_recursive(
        &mut self,
        geometry: &Geometry,
        parameters: &HashMap<&str, usize>,
    ) -> Option<NormalFn> {
        let code = match geometry {
            Geometry::Sphere { .. } => Some("return normalize(p);".to_owned()),
            Geometry::Ellipsoid { radius } => {
                let radius_x = self.lookup_parameter(&radius[0], parameters);
                let radius_y = self.lookup_parameter(&radius[1], parameters);
                let radius_z = self.lookup_parameter(&radius[2], parameters);

                Some(format!(
                    r#"
                vec3 r = vec3({}, {}, {});

                return normalize(p / (r * r));"#,
                    radius_x, radius_y, radius_z
                ))
            }
            Geometry::Translate { translation, child } => {
                let tx = self.lookup_parameter(&translation[0], parameters);
                let ty = self.lookup_parameter(&translation[1], parameters);
                let tz = self.lookup_parameter(&translation[2], parameters);

                let translation = format!("vec3({}, {}, {})", tx, ty, tz);

                let function = self.normal_recursive(child, parameters)?;

                Some(format!(
                    "return {};",
                    function.call(format!("p - {}", translation))
                ))
            }
            Geometry::Rotate { axis, angle, child } => {
                let kx = self.lookup_parameter(&axis[0], parameters);
                let ky = self.lookup_parameter(&axis[1], parameters);
                let kz = self.lookup_parameter(&axis[2], parameters);
                let theta = self.lookup_parameter(angle, parameters);

                let function = self.normal_recursive(child, parameters)?;

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
            Geometry::Scale { factor, child } => {
                let scale = self.lookup_parameter(factor, parameters);

                let function = self.normal_recursive(child, parameters)?;

                Some(format!(
                    "return {};",
                    function.call(format!("p / {}", scale))
                ))
            }
            _ => None,
        };

        Some(self.register_normal_function(code?))
    }

    fn nary_operator(
        &mut self,
        children: &[Geometry],
        parameters: &HashMap<&str, usize>,
        op: &str,
    ) -> String {
        assert!(!children.is_empty());

        if children.len() == 1 {
            let function = self.distance_recursive(&children[0], parameters);

            return format!("return {};", function.call("p"));
        }

        let mut code = String::new();

        for (i, child) in children.iter().enumerate() {
            let function = self.distance_recursive(child, parameters);

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

    fn lookup_parameter(
        &self,
        parameter: &GeometryParameter,
        parameters: &HashMap<&str, usize>,
    ) -> String {
        match parameter {
            GeometryParameter::Constant(number) => format!("{:+e}", number),
            GeometryParameter::Symbolic(symbol) => {
                self.lookup_symbolic_parameter(parameters[symbol.as_str()])
            }
        }
    }

    fn lookup_symbolic_parameter(&self, index: usize) -> String {
        match index % 4 {
            0 => format!("geometry_buffer.data[inst + {}U].x", index / 4),
            1 => format!("geometry_buffer.data[inst + {}U].y", index / 4),
            2 => format!("geometry_buffer.data[inst + {}U].z", index / 4),
            _ => format!("geometry_buffer.data[inst + {}U].w", index / 4),
        }
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
