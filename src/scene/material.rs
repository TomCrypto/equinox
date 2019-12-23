use serde::{Deserialize, Serialize};

/*

base formula is:


base + mix(color, texture * multiplier, mix_amount)


base + color + (texture * multiplier - color) * mix_amount

base + color + texture * multiplier * mix_amount - color * mix_amount

base + texture * multiplier * mix_amount + color * (1 - mix_amount)




for the "constant" variant this simply amounts to base = constant, color = black, mix_amount = 0




a + (b - a) * texture

-> b - a is the multiplier, a is the base color

fundamental form is:

a + b texture



to obtain the expression base + color + (texture * multiplier - color) * mix_amount we get:

base + color + texture * multiplier * mix_amount - color * mix_amount

a = base - color * mix_amount
b = multiplier * mix_amount






How to store this in material parameters?

Each SCALAR parameter can occupy an 8-element block, consisting of:

 - the base color (n floats)
 - the scale factor (n floats)
 - the packed texture index + texturing mode (1 float)
 - the mapping scale + offset (3 floats)

for a total of 2n + 4 floats per material parameter

for n = 1, we get 6

for n = 3, we get 10

for n = 4 we get 12

so a good choice would be three 4-float blocks per material parameter

*/

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum TextureMapping {
    Triplanar { scale: f32, offset: [f32; 2] },
    TriplanarStochastic { scale: f32, offset: [f32; 2] },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MaterialParameter<T> {
    Constant(T),
    Textured {
        base: T,
        scale: T,
        texture: String,
        mapping: TextureMapping,
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

    pub fn texture(&self) -> Option<(&str, &TextureMapping)> {
        match self {
            Self::Constant(_) => None,
            Self::Textured {
                texture, mapping, ..
            } => Some((texture, mapping)),
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
