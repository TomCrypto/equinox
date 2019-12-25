use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MaterialParameter<T> {
    Constant(T),
    Textured {
        base: T,
        scale: T,
        texture: String,
        contrast: f32,

        uv_scale: f32,
        uv_offset: [f32; 2],
        uv_rotation: f32,

        stochastic: bool,
    },
}

impl<T: Copy + Default> MaterialParameter<T> {
    pub fn base(&self) -> T {
        match self {
            Self::Constant(base) => *base,
            Self::Textured { base, .. } => *base,
        }
    }

    pub fn scale(&self) -> T {
        match self {
            Self::Constant(_) => T::default(),
            Self::Textured { scale, .. } => *scale,
        }
    }

    pub fn texture(&self) -> Option<&str> {
        match self {
            Self::Constant(_) => None,
            Self::Textured { texture, .. } => Some(texture),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Materials {
    pub list: Vec<Material>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Material {
    Lambertian {
        albedo: MaterialParameter<[f32; 3]>,
    },
    IdealReflection {
        reflectance: MaterialParameter<[f32; 3]>,
    },
    IdealRefraction {
        transmittance: MaterialParameter<[f32; 3]>,
    },
    Phong {
        albedo: MaterialParameter<[f32; 3]>,
        shininess: MaterialParameter<f32>,
    },
    Dielectric {
        base_color: MaterialParameter<[f32; 3]>,
    },
    OrenNayar {
        albedo: MaterialParameter<[f32; 3]>,
        roughness: MaterialParameter<f32>,
    },
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
