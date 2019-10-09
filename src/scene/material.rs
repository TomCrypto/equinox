use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Materials {
    pub list: Vec<Material>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Material {
    Lambertian {
        albedo: [f32; 3],
    },
    IdealReflection {
        reflectance: [f32; 3],
    },
    IdealRefraction {
        transmittance: [f32; 3],
        refractive_index: f32,
    },
    Phong {
        albedo: [f32; 3],
        shininess: f32,
    },
}
