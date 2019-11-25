use crate::Scene;
use js_sys::Error;
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, SmartDefault, Serialize)]
pub struct Integrator {
    #[default(20)]
    pub hash_table_bits: u32,

    #[default(400_000)]
    pub photons_per_pass: usize,

    #[default(0.5)]
    pub photon_rate: f32,

    #[default(0.05)]
    pub max_search_radius: f32,

    #[default(0.01)]
    pub min_search_radius: f32,

    #[default(0.7)]
    pub alpha: f32,

    #[default(8)]
    pub max_scatter_bounces: u32,

    #[default(8)]
    pub max_gather_bounces: u32,
}

impl Integrator {
    pub(crate) fn validate(&self, _scene: &Scene) -> Result<(), Error> {
        Ok(())
    }
}
