#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Device;
use coherence_base::model::{Environment, EnvironmentMap};
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

// generates a NORMALIZED [[PDF, CDF], ...] array for the given discrete
// function distribution. the last element has a CDF of 1.0 and a PDF of 0.0
// it also returns the function's integral as well

#[repr(C)]
#[derive(Debug, AsBytes, FromBytes)]
struct PdfCdf {
    cdf: f32,
    pdf: f32,
}

/*

marginal CDF = [
    PdfCdf {
        cdf: 1.0739526,
        pdf: 1.0739526,
    },
    PdfCdf {
        cdf: 1.9881946,
        pdf: 0.91424215,
    },
    PdfCdf {
        cdf: 4.230728,
        pdf: 2.2425334,
    },
    PdfCdf {
        cdf: 7.037249,
        pdf: 2.8065214,
    },
    PdfCdf {
        cdf: 7.557536,
        pdf: 0.5202869,
    },
    PdfCdf {
        cdf: 7.823413,
        pdf: 0.2658771,
    },
    PdfCdf {
        cdf: 7.956306,
        pdf: 0.13289286,
    },
    PdfCdf {
        cdf: 8.0,
        pdf: 0.043694418,
    },
    PdfCdf {
        cdf: 1.0,
        pdf: 0.0,
    },
]

*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo() {
        let data = vec![1.0, 2.0, 3.0, 4.0];

        println!("{:?}", build_normalized_pdf_cdf(&data));
    }
}

fn build_normalized_pdf_cdf(data: &[f32]) -> (Vec<PdfCdf>, f32) {
    let n = data.len() as f32;

    let mut integral = 0.0;

    for &value in data {
        integral += value / n;
    }

    let mut result = Vec::with_capacity(data.len() + 1);
    let mut running = 0.0;

    for &value in data {
        running += value;

        result.push(PdfCdf {
            pdf: value / integral,
            cdf: running / integral / n,
        });
    }

    result.push(PdfCdf { pdf: 0.0, cdf: 1.0 });

    (result, integral)
}

impl Device {
    pub(crate) fn update_environment(&mut self, environment: &Environment) {
        if let Some(map) = &environment.map {
            self.envmap_texture
                .upload(map.width as usize, map.height as usize, &map.pixels);

            // compute the CDF data and load it into our buffers...

            let tile_size = 256; // must divide the width/height

            // STEP 1: compute the filtered data which we'll build the CDF data for
            // use an average luminance measure here

            let mut filtered_data = vec![];
            let mut total = 0.0;
            let mut count = 0;

            for y in 0..map.height / tile_size {
                let mut row = vec![];

                for x in 0..map.width / tile_size {
                    let mut filtered = 0.0;

                    for ty in 0..tile_size {
                        let py = y * tile_size + ty;

                        let v = (py as f32 + 0.5) / (map.height as f32);

                        let weight = (std::f32::consts::PI * v).sin();

                        for tx in 0..tile_size {
                            let px = x * tile_size + tx;

                            let r = map.pixels[(4 * (py * map.width + px) + 0) as usize];
                            let g = map.pixels[(4 * (py * map.width + px) + 1) as usize];
                            let b = map.pixels[(4 * (py * map.width + px) + 2) as usize];

                            filtered += (r * 0.2126 + g * 0.7152 + b * 0.0722) * weight;
                        }
                    }

                    row.push(filtered / (tile_size as f32 * tile_size as f32));
                    total += filtered / (tile_size as f32 * tile_size as f32);
                    count += 1;
                }

                filtered_data.push(row);
            }

            info!("Total value of filtered function = {}", total);

            info!(
                "Average value of filtered function = {}",
                total / (count as f32)
            );

            // STEP 2: build the conditional CDFs for each row, as an array of
            // CDF values ranging from 0 to 1. There will be width +
            // 1 entries in each row. don't normalize them yet, we'll need their
            // integral value to compute the marginal CDF

            let mut conditional_cdfs: Vec<Vec<PdfCdf>> = vec![];
            let mut marginal_function: Vec<f32> = vec![];

            for y in 0..map.height / tile_size {
                let (row, integral) = build_normalized_pdf_cdf(&filtered_data[y as usize]);

                info!("row {} integral = {}", y, integral);

                // the data is just in filtered_data...
                conditional_cdfs.push(row);
                marginal_function.push(integral);
            }

            let (marginal_cdf, x) = build_normalized_pdf_cdf(&marginal_function);

            info!("marginal integral = {}", x);
            info!("marginal CDF = {:#?}", marginal_cdf);
            info!("conditional CDFs = {:#?}", conditional_cdfs);

            info!(
                "w = {}, h = {}",
                map.width / tile_size + 1,
                map.height / tile_size + 1
            );

            // STEP 5: upload the marginal CDF to its own texture

            let marginal_cdf_bytes = marginal_cdf.as_bytes();
            let marginal_cdf_floats: LayoutVerified<_, [f32]> =
                LayoutVerified::new_slice(marginal_cdf_bytes).unwrap();

            self.envmap_marginal_cdf.upload(
                (map.height / tile_size + 1) as usize,
                1,
                &marginal_cdf_floats,
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

            let conditional_cdf_bytes = conditional_cdf_data.as_bytes();
            let conditional_cdf_floats: LayoutVerified<_, [f32]> =
                LayoutVerified::new_slice(conditional_cdf_bytes).unwrap();

            self.envmap_conditional_cdfs.upload(
                (map.width / tile_size + 1) as usize,
                (map.height / tile_size) as usize,
                &conditional_cdf_floats,
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
