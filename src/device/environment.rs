#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Device;
use crate::Environment;
use half::f16;
use img2raw::{ColorSpace, DataFormat, Header};
use js_sys::Error;
use std::collections::HashMap;
use zerocopy::LayoutVerified;

impl Device {
    pub(crate) fn update_environment(
        &mut self,
        assets: &HashMap<String, Vec<u8>>,
        environment: &Environment,
    ) -> Result<(), Error> {
        if environment.map.is_some() {
            self.visible_point_gen_shader.set_define("HAS_ENVMAP", 1);
            self.test_shader.set_define("HAS_ENVMAP", 1);
        } else {
            self.visible_point_gen_shader.set_define("HAS_ENVMAP", 0);
            self.test_shader.set_define("HAS_ENVMAP", 0);
        }

        if let Some(map) = &environment.map {
            let (header, data) =
                LayoutVerified::<_, Header>::new_from_prefix(assets[&map.pixels].as_slice())
                    .unwrap();

            if header.data_format.try_parse() != Some(DataFormat::RGBA16F) {
                return Err(Error::new("expected RGBA16F environment map"));
            }

            if header.color_space.try_parse() != Some(ColorSpace::LinearSRGB) {
                return Err(Error::new("expected linear sRGB environment map"));
            }

            if header.dimensions[0] % 2 != 0 || header.dimensions[1] % 2 != 0 {
                return Err(Error::new("environment map must have even dimensions"));
            }

            let pixels = LayoutVerified::new_slice(data).unwrap();

            let cols = header.dimensions[0] as usize;
            let rows = header.dimensions[1] as usize;

            self.visible_point_gen_shader
                .set_define("ENVMAP_COLS", cols);
            self.visible_point_gen_shader
                .set_define("ENVMAP_ROWS", rows);
            self.visible_point_gen_shader
                .set_define("ENVMAP_ROTATION", format!("{:+e}", map.rotation));
            self.test_shader.set_define("ENVMAP_COLS", cols);
            self.test_shader.set_define("ENVMAP_ROWS", rows);
            self.test_shader
                .set_define("ENVMAP_ROTATION", format!("{:+e}", map.rotation));

            let mut luminance = vec![0.0f32; cols * rows];

            compute_envmap_luminance(&pixels, &mut luminance, cols, rows);

            let mut marg_cdf = Vec::with_capacity(rows);

            for y in 0..rows {
                marg_cdf.push(pdf_to_cdf(&mut luminance[y * cols..(y + 1) * cols]));
            }

            let half_data: &mut [u16] = self.allocator.allocate(cols * rows);

            for (fp16, &fp32) in half_data.iter_mut().zip(&luminance) {
                *fp16 = f16::from_f32(fp32).to_bits();
            }

            self.envmap_cond_cdf.upload(cols, rows, half_data);

            pdf_to_cdf(&mut marg_cdf);

            for (fp16, &fp32) in half_data[..rows].iter_mut().zip(&marg_cdf) {
                *fp16 = f16::from_f32(fp32).to_bits();
            }

            self.envmap_marg_cdf.upload(rows, 1, &half_data[..rows]);

            // Pack the per-pixel PDF into the alpha channel of the envmap pixel data. To
            // avoid getting clipped by the FP16 limit, use a 1e-3 multiplier on the PDF.

            let mut envmap_pixels = pixels.to_vec();

            for y in 0..rows {
                let marg_pdf = if y < rows - 1 {
                    marg_cdf[y + 1] - marg_cdf[y]
                } else {
                    1.0 - marg_cdf[y]
                };

                for x in 0..cols {
                    let cond_pdf = if x < cols - 1 {
                        luminance[y * cols + x + 1] - luminance[y * cols + x]
                    } else {
                        1.0 - luminance[y * cols + x]
                    };

                    let pdf = marg_pdf * cond_pdf * 1e-3 * (rows as f32) * (cols as f32);
                    envmap_pixels[4 * (y * cols + x) + 3] = f16::from_f32(pdf).to_bits();
                }
            }

            self.envmap_texture.upload(cols, rows, &envmap_pixels);
        }

        Ok(())
    }
}

fn pdf_to_cdf(data: &mut [f32]) -> f32 {
    let mut integral = 0.0;

    for value in data.iter_mut() {
        let temp = *value;
        *value = integral;
        integral += temp;
    }

    for value in data.iter_mut() {
        *value /= integral;
    }

    integral
}

fn compute_envmap_luminance(pixels: &[u16], luminance: &mut [f32], cols: usize, rows: usize) {
    let mut integral = 0.0;

    for y in 0..rows {
        let weight = ((y as f32 + 0.5) / (rows as f32) * std::f32::consts::PI).sin();

        for x in 0..cols {
            let r = f16::from_bits(pixels[4 * (y * cols + x)]).to_f32();
            let g = f16::from_bits(pixels[4 * (y * cols + x) + 1]).to_f32();
            let b = f16::from_bits(pixels[4 * (y * cols + x) + 2]).to_f32();

            let value = r.mul_add(0.2126, g.mul_add(0.7152, b * 0.0722)) * weight;
            luminance[y * cols + x] = value;
            integral += value;
        }
    }

    for value in luminance {
        *value /= integral;
    }
}
