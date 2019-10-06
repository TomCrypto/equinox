#[allow(unused_imports)]
use log::{debug, info, warn};

use serde::{Deserialize, Serialize};

// transforms are baked into the SDF nature of the geometry, so it's unnecessary
// to include it here. all we need here is a reference to the geometry, and a
// reference to the material

// what about multiple materials? don't bother for now

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Instance {
    pub geometry: usize,
    pub material: usize,

    pub geometry_values: Vec<f32>,
}
