#[allow(unused_imports)]
use log::{debug, info, warn};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Instance {
    pub geometry: String,
    pub material: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, f32>,
    #[serde(default = "true_default")]
    pub photon_receiver: bool,
    #[serde(default = "true_default")]
    pub sample_explicit: bool,
    #[serde(default = "true_default")]
    pub visible: bool,
}

fn true_default() -> bool {
    true
}
