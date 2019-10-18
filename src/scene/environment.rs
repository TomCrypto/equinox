use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EnvironmentMap {
    pub pixels: String,
    pub rotation: f32,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Environment {
    pub map: Option<EnvironmentMap>,
    pub multiplier: [f32; 3],
}
