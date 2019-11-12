use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Environment {
    Solid { tint: [f32; 3] },
    Map { tint: [f32; 3], rotation: f32 },
}

impl Default for Environment {
    fn default() -> Self {
        Self::Solid { tint: [1.0; 3] }
    }
}
