use crate::Alias;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnvironmentMap {
    pub pixels: Alias<Vec<f32>>,
    pub width: u32,
    pub height: u32,
    // TODO: assume equirectangular projection for now (most common by far)
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Environment {
    pub map: Option<EnvironmentMap>, // TODO: rename MapData
    pub multiplier: [f32; 3],
}
