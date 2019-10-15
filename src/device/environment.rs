#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Device;
use crate::Environment;
use half::f16;
use img2raw::{ColorSpace, DataFormat, Header};
use js_sys::Error;
use std::collections::HashMap;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

// TODO: pack envmap CDF data into f16 textures (might not be enough precision
// though)

#[repr(C)]
#[derive(Debug, AsBytes, FromBytes)]
struct PdfCdf {
    cdf: f32,
    pdf: f32,
}

fn build_normalized_pdf_cdf(data: &[f32]) -> (Vec<PdfCdf>, f32) {
    let mut integral = 0.0;

    for &value in data {
        integral += value;
    }

    let mut result = Vec::with_capacity(data.len() + 1);
    let mut running = 0.0;

    for &value in data {
        running += value;

        result.push(PdfCdf {
            pdf: value / integral,
            cdf: running / integral,
        });
    }

    result.push(PdfCdf { pdf: 0.0, cdf: 1.0 });

    (result, integral)
}

impl Device {
    pub(crate) fn update_environment(
        &mut self,
        assets: &HashMap<String, Vec<u8>>,
        environment: &Environment,
    ) -> Result<(), Error> {
        if environment.map.is_some() {
            self.program.set_define("HAS_ENVMAP", 1);
        } else {
            self.program.set_define("HAS_ENVMAP", 0);
        }

        if let Some(map) = &environment.map {
            let (header, data) =
                LayoutVerified::<_, Header>::new_from_prefix(assets[&map.pixels].as_slice())
                    .unwrap();

            if (*header).data_format.try_parse() != Some(DataFormat::RGBA16F) {
                return Err(Error::new("expected RGBA16F environment map"));
            }

            if (*header).color_space.try_parse() != Some(ColorSpace::LinearSRGB) {
                return Err(Error::new("expected linear sRGB environment map"));
            }

            let pixels = LayoutVerified::new_slice(data).unwrap();

            let cols = (*header).dimensions[0] as usize;
            let rows = (*header).dimensions[1] as usize;

            self.envmap_texture.upload(cols, rows, &pixels);

            // compute the CDF data and load it into our buffers...

            // STEP 1: compute the filtered data which we'll build the CDF data for
            // use an average luminance measure here

            let mut filtered_data = vec![];
            let mut total = 0.0;

            for y in 0..rows {
                let mut row = vec![];

                let v = (y as f32 + 0.5) / (rows as f32);

                let weight = (std::f32::consts::PI * v).sin(); // / ((cols * rows) as f32);

                for x in 0..cols {
                    let r: f32 = f16::from_bits(pixels[(4 * (y * cols + x) + 0) as usize]).into();
                    let g: f32 = f16::from_bits(pixels[(4 * (y * cols + x) + 1) as usize]).into();
                    let b: f32 = f16::from_bits(pixels[(4 * (y * cols + x) + 2) as usize]).into();

                    let value = (r * 0.2126 + g * 0.7152 + b * 0.0722) * weight;
                    total += value;

                    row.push(value);
                }

                filtered_data.push(row);
            }

            for y in 0..rows {
                for x in 0..cols {
                    filtered_data[y][x] /= total;
                }
            }

            // STEP 2: build the conditional CDFs for each row, as an array of
            // CDF values ranging from 0 to 1. There will be width +
            // 1 entries in each row. don't normalize them yet, we'll need their
            // integral value to compute the marginal CDF

            let mut conditional_cdfs: Vec<Vec<PdfCdf>> = vec![];
            let mut marginal_function: Vec<f32> = vec![];

            for y in 0..rows {
                let (row, integral) = build_normalized_pdf_cdf(&filtered_data[y as usize]);

                // the data is just in filtered_data...
                conditional_cdfs.push(row);
                marginal_function.push(integral);
            }

            let (marginal_cdf, x) = build_normalized_pdf_cdf(&marginal_function);

            // STEP 5: upload the marginal CDF to its own texture

            let marginal_cdf_bytes = marginal_cdf.as_bytes();
            let marginal_cdf_floats: LayoutVerified<_, [f32]> =
                LayoutVerified::new_slice(marginal_cdf_bytes).unwrap();

            self.envmap_marginal_cdf
                .upload((rows + 1) as usize, 1, &marginal_cdf_floats);

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
                (cols + 1) as usize,
                (rows) as usize,
                &conditional_cdf_floats,
            );
        }

        Ok(())
    }
}
