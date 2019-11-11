use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, SmartDefault, Serialize)]
pub struct Integrator {
    #[default(24)]
    pub hash_table_bits: u32,

    #[default(400_000)]
    pub photons_per_pass: usize,

    #[default(0.5)]
    pub photon_rate: f32,

    #[default(0.05)]
    pub initial_search_radius: f32,

    #[default(1.0)]
    pub capacity_multiplier: f32,

    #[default(6)]
    pub max_hash_cell_bits: u32,

    #[default(0.7)]
    pub alpha: f32,

    #[default(8)]
    pub max_scatter_bounces: u32,

    #[default(8)]
    pub max_gather_bounces: u32,
}
