use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Medium {
    pub extinction: [f32; 3],
    pub refractive_index: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Instance {
    pub geometry: String,
    pub material: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, f32>,
    #[serde(default = "true_default")]
    pub sample_explicit: bool,
    #[serde(default = "true_default")]
    pub visible: bool,

    pub medium: Medium,
    pub parent: Option<String>,
}

fn true_default() -> bool {
    true
}
