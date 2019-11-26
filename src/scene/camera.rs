use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SmartDefault)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ApertureShape {
    #[default]
    Point,
    Circle {
        radius: f32,
    },
    Ngon {
        radius: f32,
        sides: u32,
        rotation: f32,
    },
}

impl ApertureShape {
    pub fn radius(&self) -> f32 {
        match self {
            Self::Point => 0.0,
            Self::Circle { radius } => *radius,
            Self::Ngon { radius, .. } => *radius,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, SmartDefault, Serialize)]
pub struct Camera {
    #[default([0.0; 3])]
    pub position: [f32; 3],

    #[default([0.0, 0.0, 1.0])]
    pub direction: [f32; 3],

    #[default([0.0, 1.0, 0.0])]
    pub up_vector: [f32; 3],

    #[default(ApertureShape::Point)]
    pub aperture: ApertureShape,

    #[default(1.0)]
    pub focal_distance: f32,

    #[default(0.06)]
    pub focal_length: f32,

    #[default(0.024)]
    pub film_height: f32,
}
