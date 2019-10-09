use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EnvironmentMap {
    pub pixels: String,
    pub width: u32,
    pub height: u32,
    // TODO: assume equirectangular projection for now (most common by far)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Environment {
    pub map: Option<EnvironmentMap>,
    pub multiplier: [f32; 3],
}
