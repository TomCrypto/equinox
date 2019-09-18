use crate::BoundingBox;

#[derive(Default)]
pub struct Objects {
    pub list: Vec<Object>,
}

// TODO: use actual types for these things later on (with methods to get to them
// from a raw byte array for loading convenience of course... possibly defining
// a custom "all-in-one" object format)
pub struct Object {
    pub hierarchy: Vec<u8>,
    pub triangles: Vec<u8>,

    pub positions: Vec<u8>,
    pub normal_tangent_uv: Vec<u8>,
    pub materials: usize, // TODO: later on, specify default materials...

    pub bbox: BoundingBox,
}
