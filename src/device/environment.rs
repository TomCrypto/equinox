#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Device;
use crate::Environment;
use half::f16;
use img2raw::{ColorSpace, DataFormat, Header};
use js_sys::Error;
use std::collections::HashMap;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

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

    result.push(PdfCdf { pdf: 0.0, cdf: 0.0 });

    for &value in data {
        running += value;

        result.push(PdfCdf {
            pdf: value / integral,
            cdf: running / integral,
        });
    }

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

            let mut pixels = LayoutVerified::new_slice(data).unwrap().to_vec();

            let cols = (*header).dimensions[0] as usize;
            let rows = (*header).dimensions[1] as usize;

            self.program.set_define("ENVMAP_COLS", cols);
            self.program.set_define("ENVMAP_ROWS", rows);
            self.program
                .set_define("ENVMAP_ROTATION", format!("{:+e}", map.rotation));

            // compute the CDF data and load it into our buffers...

            // STEP 1: compute the filtered data which we'll build the CDF data for
            // use an average luminance measure here

            let mut filtered_data = vec![];
            let mut total = 0.0;

            for y in 0..rows {
                let mut row = vec![];

                let v = (y as f32 + 0.5) / (rows as f32);

                let weight = (std::f32::consts::PI * v).sin();

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

                    pixels[4 * (y * cols + x) + 3] =
                        f16::from_f32(filtered_data[y][x] * (rows as f32) * (cols as f32))
                            .to_bits();
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

                conditional_cdfs.push(row);
                marginal_function.push(integral);
            }

            let (marginal_cdf, _) = build_normalized_pdf_cdf(&marginal_function);

            self.envmap_texture.upload(cols, rows, &pixels);

            // STEP 5: upload the marginal CDF to its own texture

            let marginal_cdf_floats: &mut [u16] = self.allocator.allocate(rows);

            for y in 0..rows {
                marginal_cdf_floats[y] = f16::from_f32(marginal_cdf[y].cdf).to_bits();
            }

            self.envmap_marg_cdf
                .upload(rows as usize, 1, marginal_cdf_floats);

            // STEP 6: upload the conditional CDF to its own texture (one line
            // per CDF)

            let conditional_cdf_floats: &mut [u16] = self.allocator.allocate(cols * rows);

            for y in 0..rows {
                for x in 0..cols {
                    conditional_cdf_floats[y * cols + x] =
                        f16::from_f32(conditional_cdfs[y][x].cdf).to_bits();
                }
            }

            self.envmap_cond_cdf
                .upload(cols as usize, rows as usize, conditional_cdf_floats);
        }

        Ok(())
    }
}
