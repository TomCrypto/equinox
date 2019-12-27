#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{Device, Environment};
use half::f16;
use img2raw::{ColorSpace, DataFormat, Header};
use js_sys::Error;
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
        assets: &dyn Fn(&str) -> Result<Vec<u8>, Error>,
        map: Option<&str>,
    ) -> Result<(), Error> {
        if let Some(map) = map {
            let asset_data = assets(map)?;

            let (header, data) =
                LayoutVerified::<_, Header>::new_from_prefix(asset_data.as_slice()).unwrap();

            if header.data_format.try_parse() != Some(DataFormat::RGBE8) {
                return Err(Error::new("expected RGBE8 environment map"));
            }

            if header.color_space.try_parse() != Some(ColorSpace::LinearSRGB) {
                return Err(Error::new("expected linear sRGB environment map"));
            }

            if header.dimensions[0] == 0 || header.dimensions[1] == 0 {
                return Err(Error::new("invalid environment map dimensions"));
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

            let mut half_data = vec![0; cols * rows];

            for (fp16, &fp32) in half_data.iter_mut().zip(&luminance) {
                *fp16 = f16::from_f32(fp32).to_bits();
            }

            self.envmap_cond_cdf.upload(cols, rows, &half_data);

            pdf_to_cdf(&mut marg_cdf);

            for (fp16, &fp32) in half_data[..rows].iter_mut().zip(&marg_cdf) {
                *fp16 = f16::from_f32(fp32).to_bits();
            }

            self.envmap_marg_cdf.upload(rows, 1, &half_data[..rows]);

            // Pack the per-pixel PDF into the alpha channel of the envmap pixel data. To
            // avoid getting clipped by the very low FP16 limit, divide this PDF by 1024.

            let mut envmap_pixels = vec![0u16; pixels.len()];
            rgbe8_pixels_to_f16(&pixels, &mut envmap_pixels);

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

                    let pdf = marg_pdf * cond_pdf * (rows as f32) * (cols as f32) / 1024.0;
                    envmap_pixels[4 * (y * cols + x) + 3] = f16::from_f32(pdf).to_bits();
                }
            }

            self.envmap_color.upload(cols, rows, &envmap_pixels);
        } else {
            self.envmap_cond_cdf.reset();
            self.envmap_marg_cdf.reset();
            self.envmap_color.reset();
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
                shader_data.cols = self.envmap_color.cols() as i32;
                shader_data.rows = self.envmap_color.rows() as i32;
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

fn compute_envmap_luminance(pixels: &[u8], luminance: &mut [f32], cols: usize, rows: usize) {
    let mut integral = 0.0;

    for y in 0..rows {
        let weight = ((y as f32 + 0.5) / (rows as f32) * std::f32::consts::PI).sin();

        for x in 0..cols {
            let (r, g, b) = unpack_rgbe8(&pixels[4 * (y * cols + x)..4 * (y * cols + x) + 4]);

            let value = r.mul_add(0.2126, g.mul_add(0.7152, b * 0.0722)) * weight;
            luminance[y * cols + x] = value;
            integral += value;
        }
    }

    for value in luminance {
        *value /= integral;
    }
}

fn rgbe8_pixels_to_f16(src_pixels: &[u8], dst_pixels: &mut [u16]) {
    for (rgbe8, half) in src_pixels.chunks(4).zip(dst_pixels.chunks_mut(4)) {
        let (r, g, b) = unpack_rgbe8(rgbe8);

        half[0] = f16::from_f32(r).to_bits();
        half[1] = f16::from_f32(g).to_bits();
        half[2] = f16::from_f32(b).to_bits();
        half[3] = 0;
    }
}

fn unpack_rgbe8(rgbe: &[u8]) -> (f32, f32, f32) {
    if rgbe[3] == 0 {
        return (0.0, 0.0, 0.0);
    }

    let f = 2.0f32.powi(rgbe[3] as i32 - 128 - 8);

    let r = (rgbe[0] as f32 * f).max(0.0).min(65500.0);
    let g = (rgbe[1] as f32 * f).max(0.0).min(65500.0);
    let b = (rgbe[2] as f32 * f).max(0.0).min(65500.0);

    (r, g, b)
}
