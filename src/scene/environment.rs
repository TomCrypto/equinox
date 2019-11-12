use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EnvironmentMap {
    pub pixels: String,
    #[serde(default)]
    pub rotation: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Environment {
    pub map: Option<EnvironmentMap>,
    pub tint: [f32; 3],
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            map: None,
            tint: [1.0; 3],
        }
    }
}
