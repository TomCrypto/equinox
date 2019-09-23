#[derive(Default)]
pub struct Instances {
    pub list: Vec<Instance>,
}

// transforms are baked into the SDF nature of the geometry, so it's unnecessary
// to include it here. all we need here is a reference to the geometry, and a
// reference to the material

// what about multiple materials? don't bother for now

pub struct Instance {
    pub geometry: usize,
    pub material: usize,

    pub geometry_values: Vec<f32>,
    pub material_values: Vec<f32>,
}
