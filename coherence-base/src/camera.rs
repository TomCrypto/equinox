use cgmath::{Point3, Vector3};
use smart_default::SmartDefault;

#[derive(Clone, Copy, SmartDefault)]
pub enum Aperture {
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

impl Aperture {
    pub fn radius(&self) -> f32 {
        match self {
            Self::Point => 0.0,
            Self::Circle { radius } => *radius,
            Self::Ngon { radius, .. } => *radius,
        }
    }
}

#[derive(SmartDefault)]
pub struct Camera {
    #[default(Point3::new(0.0, 0.0, 0.0))]
    pub position: Point3<f32>,

    #[default(Vector3::new(0.0, 0.0, 1.0))]
    pub direction: Vector3<f32>,

    #[default(Vector3::new(0.0, 1.0, 0.0))]
    pub up_vector: Vector3<f32>,

    #[default(Aperture::Point)]
    pub aperture: Aperture,

    #[default(1.0)]
    pub focal_distance: f32,

    #[default(0.06)]
    pub focal_length: f32,

    #[default(0.024)]
    pub film_height: f32,

    pub aperture_r_spectrum: Vec<f32>,
    pub aperture_g_spectrum: Vec<f32>,
    pub aperture_b_spectrum: Vec<f32>,
    pub aperture_width: u32,
    pub aperture_height: u32,
}
