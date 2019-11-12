use crate::Asset;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Aperture {
    pub aperture_texels: Asset,
    pub aperture_width: u32,
    pub aperture_height: u32,
}
