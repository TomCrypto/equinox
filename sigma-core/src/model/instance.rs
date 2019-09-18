use cgmath::{Quaternion, Vector3};

// TODO: convert usize to Arc<Object> later on once we're further along

pub struct Instance {
    pub object: usize,
    pub scale: f32,
    pub rotation: Quaternion<f32>,
    pub translation: Vector3<f32>,
}

#[derive(Default)]
pub struct Instances {
    pub list: Vec<Instance>,
}
