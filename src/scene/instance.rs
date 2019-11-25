use crate::Scene;
use js_sys::Error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Instance {
    pub geometry: String,
    pub material: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, f32>,
    #[serde(default = "true_default")]
    pub photon_receiver: bool,
    #[serde(default = "true_default")]
    pub sample_explicit: bool,
    #[serde(default = "true_default")]
    pub visible: bool,
}

fn true_default() -> bool {
    true
}

impl Instance {
    pub(crate) fn validate(&self, scene: &Scene) -> Result<(), Error> {
        if !scene.geometry_list.contains_key(&self.geometry) {
            return Err(Error::new(&format!("no such geometry: {}", self.geometry)));
        }

        if !scene.material_list.contains_key(&self.material) {
            return Err(Error::new(&format!("no such material: {}", self.material)));
        }

        Ok(())
    }
}
