#[allow(unused_imports)]
use log::{debug, info, warn};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Instance {
    pub geometry: usize,
    pub material: usize,
    pub parameters: Vec<f32>,
}
