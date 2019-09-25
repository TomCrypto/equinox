#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Device;
use coherence_base::model::{Environment, EnvironmentMap};

// need functions to write the environment map into the renderer?

impl Device {
    pub(crate) fn update_environment(&mut self, environment: &Environment) {
        if let Some(map) = &environment.map {
            self.envmap_texture
                .upload(map.width as usize, map.height as usize, &map.pixels);

            // compute the CDF data and load it into our buffers...

            let tile_size = 4; // must divide the width/height

            // STEP 1: compute the filtered data which we'll build the CDF data for
            // use an average luminance measure here

            let mut filtered_data = vec![];

            for y in 0..map.height / tile_size {
                let mut row = vec![];

                for x in 0..map.width / tile_size {
                    let mut filtered = 0.0;

                    for ty in 0..tile_size {
                        let py = y * tile_size + ty;

                        let v = (py as f32 + 0.5) / (map.height as f32);

                        let weight = (std::f32::consts::PI * v).sin()
                            / (tile_size as f32 * tile_size as f32);

                        for tx in 0..tile_size {
                            let px = x * tile_size + tx;

                            let r = map.pixels[(4 * (py * map.width + px) + 0) as usize];
                            let g = map.pixels[(4 * (py * map.width + px) + 1) as usize];
                            let b = map.pixels[(4 * (py * map.width + px) + 2) as usize];

                            filtered += (r * 0.2126 + g * 0.7152 + b * 0.0722) * weight;
                        }
                    }

                    row.push(filtered);
                }

                filtered_data.push(row);
            }

            // STEP 2: build the conditional CDFs for each row, as an array of
            // CDF values ranging from 0 to 1. There will be width +
            // 1 entries in each row. don't normalize them yet, we'll need their
            // integral value to compute the marginal CDF

            let mut conditional_cdfs = vec![];

            for y in 0..map.height / tile_size {
                let mut conditional_cdf = vec![0.0];
                let mut integral = 0.0;

                for x in 0..map.width / tile_size {
                    integral += filtered_data[y as usize][x as usize];
                    conditional_cdf.push(integral);
                }

                conditional_cdfs.push(conditional_cdf);
            }

            // STEP 3: build the marginal CDF, as an array of CDF values ranging
            // from 0 to 1. There will be height + 1 entries in this
            // marginal CDF. Don't normalize it yet.

            let mut marginal_cdf = vec![0.0];
            let mut integral = 0.0;

            for y in 0..map.height / tile_size {
                integral += conditional_cdfs[y as usize].last().unwrap();
                marginal_cdf.push(integral);
            }

            // STEP 4: normalize all CDFs to [0, 1]

            let len = marginal_cdf.len() - 1;

            for (index, value) in marginal_cdf.iter_mut().enumerate() {
                if index != len {
                    let conditional_integral = *conditional_cdfs[index].last().unwrap();

                    for value2 in &mut conditional_cdfs[index] {
                        *value2 /= conditional_integral;
                    }
                }

                *value /= integral;
            }

            info!("marginal CDF = {:#?}", marginal_cdf);
            info!("conditional CDFs = {:#?}", conditional_cdfs);

            info!(
                "w = {}, h = {}",
                map.width / tile_size + 1,
                map.height / tile_size + 1
            );

            // STEP 5: upload the marginal CDF to its own texture

            self.envmap_marginal_cdf.upload(
                (map.height / tile_size + 1) as usize,
                1,
                &marginal_cdf,
            );

            info!(
                "envmap_marginal_cdf ROWS = {}",
                self.envmap_marginal_cdf.rows()
            );
            info!(
                "envmap_marginal_cdf COLS = {}",
                self.envmap_marginal_cdf.cols()
            );

            // STEP 6: upload the conditional CDF to its own texture (one line
            // per CDF)

            let mut conditional_cdf_data = vec![];

            for mut conditional_cdf in conditional_cdfs {
                conditional_cdf_data.append(&mut conditional_cdf);
            }

            self.envmap_conditional_cdfs.upload(
                (map.width / tile_size + 1) as usize,
                (map.height / tile_size) as usize,
                &conditional_cdf_data,
            );

            info!(
                "envmap_conditional_cdfs ROWS = {}",
                self.envmap_conditional_cdfs.rows()
            );
            info!(
                "envmap_conditional_cdfs COLS = {}",
                self.envmap_conditional_cdfs.cols()
            );
        }
    }
}
