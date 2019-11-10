use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize, SmartDefault)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RasterFilter {
    #[default]
    BlackmanHarris,
    Dirac,
}

impl RasterFilter {
    pub fn importance_sample(self, t: f32) -> f32 {
        match self {
            Self::Dirac => 0.0, // dirac has a trivial CDF
            _ => self.evaluate_inverse_cdf_via_bisection(t),
        }
    }

    #[allow(clippy::float_cmp)]
    fn evaluate_inverse_cdf_via_bisection(self, t: f32) -> f32 {
        let mut lo = 0.0;
        let mut hi = 1.0;
        let mut last = t;

        loop {
            let mid = (lo + hi) / 2.0;

            let sample = self.evaluate_cdf(mid);

            if sample == last {
                return mid;
            }

            if sample < t {
                lo = mid;
            } else {
                hi = mid;
            }

            last = sample;
        }
    }

    fn evaluate_cdf(self, t: f32) -> f32 {
        match self {
            Self::Dirac => unreachable!(),
            Self::BlackmanHarris => {
                let s1 = 0.216_623_8 * (2.0 * std::f32::consts::PI * t).sin();
                let s2 = 0.031_338_5 * (4.0 * std::f32::consts::PI * t).sin();
                let s3 = 0.001_727_2 * (6.0 * std::f32::consts::PI * t).sin();
                t - s1 + s2 - s3 // integral of the normalized window function
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, SmartDefault)]
pub struct Raster {
    #[default(150)]
    pub width: u32,
    #[default(150)]
    pub height: u32,
    pub filter: RasterFilter,
}
