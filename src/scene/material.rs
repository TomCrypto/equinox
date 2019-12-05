use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Materials {
    pub list: Vec<Material>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Material {
    Lambertian { albedo: [f32; 3] },
    IdealReflection { reflectance: [f32; 3] },
    IdealRefraction { transmittance: [f32; 3] },
    Phong { albedo: [f32; 3], shininess: f32 },
    Dielectric { base_color: [f32; 3] },
    OrenNayar { albedo: [f32; 3], roughness: f32 },
}

impl Material {
    pub fn has_delta_bsdf(&self) -> bool {
        match self {
            Self::Lambertian { .. } => false,
            Self::IdealReflection { .. } => true,
            Self::IdealRefraction { .. } => true,
            Self::Phong { .. } => false,
            Self::Dielectric { .. } => true,
            Self::OrenNayar { .. } => false,
        }
    }
}
