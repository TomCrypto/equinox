use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MaterialParameterType {
    Scalar(f32),
    Vector([f32; 3]),
}

impl MaterialParameterType {
    pub fn as_vec3(&self) -> [f32; 3] {
        match self {
            Self::Scalar(c) => [*c; 3],
            Self::Vector(v) => *v,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MaterialParameter {
    Constant(MaterialParameterType),
    Textured {
        base: MaterialParameterType,
        scale: MaterialParameterType,
        texture: String,
        contrast: f32,

        uv_scale: f32,
        uv_offset: [f32; 2],
        uv_rotation: f32,

        stochastic: bool,
    },
}

impl MaterialParameter {
    pub fn base(&self) -> [f32; 3] {
        match self {
            Self::Constant(base) => base.as_vec3(),
            Self::Textured { base, .. } => base.as_vec3(),
        }
    }

    pub fn scale(&self) -> [f32; 3] {
        match self {
            Self::Constant(_) => [0.0; 3],
            Self::Textured { scale, .. } => scale.as_vec3(),
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
        albedo: MaterialParameter,
    },
    IdealReflection {
        reflectance: MaterialParameter,
    },
    IdealRefraction {
        transmittance: MaterialParameter,
    },
    Phong {
        albedo: MaterialParameter,
        shininess: MaterialParameter,
    },
    Dielectric {
        base_color: MaterialParameter,
    },
    OrenNayar {
        albedo: MaterialParameter,
        roughness: MaterialParameter,
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
