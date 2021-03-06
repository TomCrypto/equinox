use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, SmartDefault, Serialize)]
#[serde(default)]
pub struct Integrator {
    #[default(20)]
    pub hash_table_bits: u32,

    #[default(400_000)]
    pub photons_per_pass: usize,

    #[default(0.05)]
    pub max_search_radius: f32,

    #[default(0.01)]
    pub min_search_radius: f32,

    #[default(0.7)]
    pub alpha: f32,

    #[default(5)]
    pub max_scatter_bounces: u32,

    #[default(5)]
    pub max_gather_bounces: u32,

    #[default(1e-3)]
    pub geometry_precision: f32,

    #[default(5.0)]
    pub geometry_pushback: f32,
}
