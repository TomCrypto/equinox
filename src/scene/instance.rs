#[allow(unused_imports)]
use log::{debug, info, warn};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Instance {
    pub geometry: usize,
    pub material: usize,
    #[serde(default)]
    pub parameters: Vec<f32>,
    #[serde(default)]
    pub receiver: bool,
}
