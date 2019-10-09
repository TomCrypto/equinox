use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Aperture {
    pub aperture_texels: String,
    pub aperture_width: u32,
    pub aperture_height: u32,
}
