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
pub enum MaterialParameterTexture {
    Single(String),
    Multi { horz: String, vert: String },
}

impl MaterialParameterTexture {
    pub fn horz_texture(&self) -> &str {
        match self {
            Self::Single(texture) => texture,
            Self::Multi { horz, .. } => horz,
        }
    }

    pub fn vert_texture(&self) -> &str {
        match self {
            Self::Single(texture) => texture,
            Self::Multi { vert, .. } => vert,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TexturedMaterialParameter {
    pub base: MaterialParameterType,
    pub factor: MaterialParameterType,
    pub texture: MaterialParameterTexture,
    pub contrast: f32,

    pub uv_scale: f32,
    pub uv_offset: [f32; 2],
    pub uv_rotation: f32,

    pub stochastic: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MaterialParameter {
    Constant(MaterialParameterType),
    Textured(TexturedMaterialParameter),
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
}

impl Material {
    pub fn has_delta_bsdf(&self) -> bool {
        match self {
            Self::Lambertian { .. } => false,
            Self::IdealReflection { .. } => true,
            Self::IdealRefraction { .. } => true,
            Self::Phong { .. } => false,
            Self::Dielectric { .. } => true,
        }
    }

    pub fn is_photon_receiver(&self) -> bool {
        matches!(self, Self::Lambertian { .. })
    }

    /// Returns a list of parameters referenced by this material.
    pub fn parameters(&self) -> Vec<(&str, &MaterialParameter)> {
        match self {
            Self::Lambertian { albedo } => vec![("albedo", &albedo)],
            Self::IdealReflection { reflectance } => vec![("reflectance", &reflectance)],
            Self::IdealRefraction { transmittance } => vec![("transmittance", &transmittance)],
            Self::Phong { albedo, shininess } => {
                vec![("albedo", &albedo), ("shininess", &shininess)]
            }
            Self::Dielectric { base_color } => vec![("base_color", &base_color)],
        }
    }
}
