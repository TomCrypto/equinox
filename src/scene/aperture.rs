use crate::Alias;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Aperture {
    pub aperture_texels: Alias<Vec<u8>>,
    pub aperture_width: u32,
    pub aperture_height: u32,
}
