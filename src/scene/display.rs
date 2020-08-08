use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

pub type CameraResponse = [[f32; 3]; 11];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, SmartDefault, Serialize)]
#[serde(default)]
pub struct Display {
    #[default(0.0)]
    pub exposure: f32,
    #[default(1.0)]
    pub saturation: f32,
    #[default(false)]
    pub lens_flare_enabled: bool,
    #[default(1)]
    pub lens_flare_tiles_per_pass: u32,
    #[default(None)]
    pub render_region: Option<[u32; 4]>,
}
