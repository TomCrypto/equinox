use cgmath::{Quaternion, Vector3};

// TODO: convert usize to Arc<Object> later on once we're further along

pub struct Instance {
    pub object: usize,
    /// Scale of this instance (should be positive).
    pub scale: f32,
    /// Rotation of this instance.
    pub rotation: Quaternion<f32>,
    /// Translation of this instance.
    pub translation: Vector3<f32>,
    /// List of material indices.
    pub materials: Vec<usize>,
}

#[derive(Default)]
pub struct Instances {
    pub list: Vec<Instance>,
}
