use crate::device::ToDevice;
use crate::model::Objects;
use zerocopy::{AsBytes, FromBytes};

// TODO: give these proper types later on? atm we just treat them as byte blobs

#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct HierarchyData(u8);

#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct TriangleData(u8);

#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct VertexPositionData(u8);

#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct VertexMappingData(u8);

// TODO: might be good to check for overflow here and stuff
/*
impl ToDevice<[HierarchyData]> for Objects {
    fn to_device(&self, slice: &mut [HierarchyData]) {
        let mut bytes = slice.as_bytes_mut();

        for object in &self.list {
            let (region, rest) = bytes.split_at_mut(object.hierarchy.len());
            region.copy_from_slice(&object.hierarchy);
            bytes = rest;
        }
    }

    fn requested_count(&self) -> usize {
        self.list.iter().map(|obj| obj.hierarchy.len()).sum()
    }
}

impl ToDevice<[TriangleData]> for Objects {
    fn to_device(&self, slice: &mut [TriangleData]) {
        let mut bytes = slice.as_bytes_mut();

        for object in &self.list {
            let (region, rest) = bytes.split_at_mut(object.triangles.len());
            region.copy_from_slice(&object.triangles);
            bytes = rest;
        }
    }

    fn requested_count(&self) -> usize {
        self.list.iter().map(|obj| obj.triangles.len()).sum()
    }
}

impl ToDevice<[VertexPositionData]> for Objects {
    fn to_device(&self, slice: &mut [VertexPositionData]) {
        let mut bytes = slice.as_bytes_mut();

        for object in &self.list {
            let (region, rest) = bytes.split_at_mut(object.positions.len());
            region.copy_from_slice(&object.positions);
            bytes = rest;
        }
    }

    fn requested_count(&self) -> usize {
        self.list.iter().map(|obj| obj.positions.len()).sum()
    }
}

impl ToDevice<[VertexMappingData]> for Objects {
    fn to_device(&self, slice: &mut [VertexMappingData]) {
        let mut bytes = slice.as_bytes_mut();

        for object in &self.list {
            let (region, rest) = bytes.split_at_mut(object.normal_tangent_uv.len());
            region.copy_from_slice(&object.normal_tangent_uv);
            bytes = rest;
        }
    }

    fn requested_count(&self) -> usize {
        self.list
            .iter()
            .map(|obj| obj.normal_tangent_uv.len())
            .sum()
    }
}
*/

/*

generate GLSL code for each of the objects? (or should this be in the renderer; probably in the
renderer)

in here, there would be nothing to do; the renderer simply iterates over all distance fields and
produces the corresponding GLSL code...

would it be "better" to do this here? for consistenyc if anything

now the question is, how do we represent the SDFs in the scene model? there's a way to do it with
generics, and we can box the final thing. What information do we need out of the boxed object?

 - its AABB (needed to compute the BVH)
 - its list of parameters in some order
 - ultimately being able to traverse it as a tree, because if we want the renderer to be able to
 generate the code then this is 100% needed (it needs to understand each primitive and generate
 appropriate code for each part of the distance field graph)

=> would it be nice if it could automatically isolate logically disconnected SDFs and put them
in separate BVH nodes? this isn't doable in general however and would only apply to top-level
"union" operations; no, this is a bad idea

*/

use crate::model::Geometry;

fn emit_function(code: &mut String, index: &mut usize, body: &str) -> String {
    let name = format!("sdf{}", *index);
    *index += 1;

    *code += &format!("float {}(vec3 x, uint inst) {{ {} }}", name, body);

    name
}

fn parameter_access(index: usize) -> String {
    let array_idx = index / 4;

    match index % 4 {
        0 => format!("geometry_values.data[inst + {}U].x", array_idx),
        1 => format!("geometry_values.data[inst + {}U].y", array_idx),
        2 => format!("geometry_values.data[inst + {}U].z", array_idx),
        _ => format!("geometry_values.data[inst + {}U].w", array_idx),
    }
}

// assume the parameters come from a vec4 array called GEOMETRY_PARAMS
// for now use direct indexing, so if parameter index is X then we look it up at
// index X of this array (i.e. array[X / 4].{xyzw})
// then the parameter array in the instances can just be uploaded as-is
// later on, reorganize the parameters so that they get accessed linearly in
// memory if possible

impl Geometry {
    pub fn as_glsl_function(&self, code: &mut String, index: &mut usize) -> String {
        match self {
            Self::UnitSphere => emit_function(code, index, "return length(x) - 1.0;"),
            Self::UnitCube => emit_function(
                code,
                index,
                r#"vec3 d = abs(x) - vec3(1.0); return length(max(d,0.0)) + min(max(d.x,max(d.y,d.z)),0.0);"#,
            ),
            Self::Union { children } => {
                let name1 = children[0].as_glsl_function(code, index);
                let name2 = children[1].as_glsl_function(code, index);

                emit_function(
                    code,
                    index,
                    &format!("return min({}(x, inst), {}(x, inst));", name1, name2),
                )
            }
            Self::Intersection { children } => {
                // TODO: only support 2 children for now...
                let name1 = children[0].as_glsl_function(code, index);
                let name2 = children[1].as_glsl_function(code, index);

                emit_function(
                    code,
                    index,
                    &format!("return max({}(x, inst), {}(x, inst));", name1, name2),
                )
            }
            Self::Scale { factor, f } => {
                let name = f.as_glsl_function(code, index);

                let factor_code = match factor {
                    Parameter::Constant(value) => format!("{:+e}", value),
                    Parameter::Symbolic(index) => parameter_access(*index),
                };

                emit_function(
                    code,
                    index,
                    &format!("float s = {}; return {}(x / s, inst) * s;", factor_code, name,),
                )
            }
            Self::Translate { translation, f } => {
                let name = f.as_glsl_function(code, index);

                let tx = match translation[0] {
                    Parameter::Constant(value) => format!("{:+e}", value),
                    Parameter::Symbolic(index) => parameter_access(index),
                };

                let ty = match translation[1] {
                    Parameter::Constant(value) => format!("{:+e}", value),
                    Parameter::Symbolic(index) => parameter_access(index),
                };

                let tz = match translation[2] {
                    Parameter::Constant(value) => format!("{:+e}", value),
                    Parameter::Symbolic(index) => parameter_access(index),
                };

                let translation_code = format!("vec3({}, {}, {})", tx, ty, tz);

                emit_function(
                    code,
                    index,
                    &format!("return {}(x - {}, inst);", name, translation_code),
                )
            },
            Self::Round { radius, f } => {
                let name = f.as_glsl_function(code, index);

                let radius_code = match radius {
                    Parameter::Constant(value) => format!("{:+e}", value),
                    Parameter::Symbolic(index) => parameter_access(*index),
                };

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
