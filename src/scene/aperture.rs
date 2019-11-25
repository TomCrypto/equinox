use crate::{Asset, Scene};
use js_sys::Error;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Aperture {
    pub aperture_texels: Asset,
    pub aperture_width: u32,
    pub aperture_height: u32,
}

impl Aperture {
    pub(crate) fn validate(&self, scene: &Scene) -> Result<(), Error> {
        if !scene.assets.contains_key(&self.aperture_texels) {
            return Err(Error::new("aperture_texels not in assets"));
        }

        Ok(())
    }
}
