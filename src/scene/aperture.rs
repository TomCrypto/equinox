use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConvolutionTileSize {
    Lowest,
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, Deserialize, PartialEq, SmartDefault, Serialize)]
pub struct Aperture {
    pub filter: String,

    #[default(ConvolutionTileSize::Medium)]
    pub tile_size: ConvolutionTileSize,
}
