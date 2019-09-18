use crate::model::Object;
use crate::BoundingBox;
use crate::Dirty;
use cgmath::{Point3, Quaternion, Vector3};
use smart_default::SmartDefault;

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
