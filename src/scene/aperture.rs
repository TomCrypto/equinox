use crate::Alias;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Aperture {
    pub aperture_r_spectrum: Alias<Vec<f32>>,
    pub aperture_g_spectrum: Alias<Vec<f32>>,
    pub aperture_b_spectrum: Alias<Vec<f32>>,
    pub aperture_width: u32,
    pub aperture_height: u32,
}
