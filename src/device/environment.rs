#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Device;
use crate::{Asset, Environment};
use half::f16;
use img2raw::{ColorSpace, DataFormat, Header};
use js_sys::Error;
use std::collections::HashMap;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Debug, Default)]
pub struct EnvironmentData {
    cols: i32,
    rows: i32,
    rotation: f32,
    has_envmap: i32,
    tint: [f32; 3],
    padding: [f32; 1],
}

impl Device {
    pub(crate) fn update_environment_map(
        &mut self,
        assets: &HashMap<Asset, Vec<u8>>,
        map: Option<&Asset>,
    ) -> Result<(), Error> {
        if let Some(map) = map {
            let (header, data) =
                LayoutVerified::<_, Header>::new_from_prefix(assets[map].as_slice()).unwrap();

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
        } else {
            self.envmap_cond_cdf.upload(1, 1, &[0]);
            self.envmap_marg_cdf.upload(1, 1, &[0]);
            self.envmap_texture.upload(1, 1, &[0; 4]);
        }

        Ok(())
    }

    pub(crate) fn update_environment(&mut self, environment: &Environment) -> Result<(), Error> {
        let mut shader_data = EnvironmentData::default();

        match environment {
            Environment::Map { tint, rotation } => {
                shader_data.tint[0] = tint[0].max(0.0);
                shader_data.tint[1] = tint[1].max(0.0);
                shader_data.tint[2] = tint[2].max(0.0);
                shader_data.rotation = rotation % (2.0 * std::f32::consts::PI);
                shader_data.has_envmap = 1;
                shader_data.cols = self.envmap_texture.cols() as i32;
                shader_data.rows = self.envmap_texture.rows() as i32;
            }
            Environment::Solid { tint } => {
                shader_data.tint[0] = tint[0].max(0.0);
                shader_data.tint[1] = tint[1].max(0.0);
                shader_data.tint[2] = tint[2].max(0.0);
                shader_data.has_envmap = 0;
            }
        }

        self.environment_buffer.write(&shader_data)
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
