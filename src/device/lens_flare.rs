#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Device;
use crate::Framebuffer;
use crate::{Texture, RG32F, RGBA32F};
use crate::{VertexArray, VertexAttribute, VertexAttributeKind, VertexLayout};
use rustfft::{num_complex::Complex, FFTplanner};
use zerocopy::{AsBytes, FromBytes};

#[repr(align(8), C)]
#[derive(AsBytes, FromBytes, Debug)]
pub struct FFTPassData {
    pub horizontal: u16,
    pub direction: u16,
    pub subtransform_size: u16,
    pub convolve: u16,
}

impl VertexLayout for FFTPassData {
    const VERTEX_LAYOUT: &'static [VertexAttribute] =
        &[VertexAttribute::new(0, 0, VertexAttributeKind::UShort4)];
}

// TODO: possible speed-ups:
//  - larger radix to reduce number of iterations (not viable without compute
//    shaders, or doing some horrible stuff with vertex shader quads)
//  - complex-to-real/real-to-complex FFT speedups (not really worth the code
//    complexity increase as it only applies to the outermost dimension)

fn bilinear_interpolation(psf: &[f32], width: usize, height: usize, mut x: f32, mut y: f32) -> f32 {
    if x < 0.0 || y < 0.0 {
        return 0.0;
    }

    if x > 1.0 || y > 1.0 {
        return 0.0;
    }

    x *= width as f32;
    y *= height as f32;
    x -= 0.5;
    y -= 0.5;

    let mut x_first = x.floor() as usize;
    let mut x_after = x_first + 1;

    let mut y_first = y.floor() as usize;
    let mut y_after = y_first + 1;

    let mut x_frac = x.fract();
    let mut y_frac = y.fract();

    if x_first >= width - 1 {
        x_frac = 0.0;
        x_first = width - 1;
        x_after = 0;
    }

    if y_first >= height - 1 {
        y_frac = 0.0;
        y_first = height - 1;
        y_after = 0;
    }

    let s00 = psf[y_first * width + x_first] * (1.0 - x_frac) * (1.0 - y_frac);
    let s01 = psf[y_first * width + x_after] * x_frac * (1.0 - y_frac);
    let s10 = psf[y_after * width + x_first] * (1.0 - x_frac) * y_frac;
    let s11 = psf[y_after * width + x_after] * x_frac * y_frac;

    s00 + s01 + s10 + s11
}

impl Device {
    pub(crate) fn render_lens_flare(&mut self) {
        let mut location = self.load_into_convolution_buffers(&self.integrator_radiance_estimate);
        self.perform_convolution(&mut location);
        self.load_convolved_render_from_convolution_buffers(&mut location);
    }

    pub(crate) fn preprocess_filter(
        &mut self,
        aperture_grayscale: &[u8],
        cols: usize,
        rows: usize,
    ) {
        /*

        Goals of this method:

         - populate the convolution filter buffers

        Steps:

         1. compute the FFT of the aperture grayscale data at the provided resolution
         2. compute the point spread function from it by taking the magnitude at each point
         3. construct the filter for each wavelength
         4. do a GPU FFT pass using the existing logic to produce the convolution filters

        4 is already implemented since we do the same thing during convolution
        ideally 1/2 will be done on the GPU later on, but it's not a priority
        3 may as well be done on the GPU right away

        */

        // STEP 1. compute the 2D FFT of the aperture grayscale data

        let row_fft = FFTplanner::<f32>::new(false).plan_fft(cols);
        let col_fft = FFTplanner::<f32>::new(false).plan_fft(rows);

        let mut aperture_input = Vec::with_capacity(aperture_grayscale.len());

        for i in 0..(rows * cols) {
            aperture_input.push(Complex::from(aperture_grayscale[i] as f32 / 255.0));
        }

        // so sad that we can't just process in-place here

        for i in 0..rows {
            let row = &mut aperture_input[cols * i..cols * i + cols];

            let mut output = vec![Complex::<f32>::default(); row.len()];

            row_fft.process(row, &mut output);

            row.copy_from_slice(&output);
        }

        for i in 0..cols {
            let mut col = Vec::with_capacity(rows);

            for j in 0..rows {
                col.push(aperture_input[j * cols + i]);
            }

            let mut output = vec![Complex::<f32>::default(); col.len()];

            col_fft.process(&mut col, &mut output);

            for j in 0..rows {
                aperture_input[j * cols + i] = output[j];
            }
        }

        // TODO: this might be done by the prior FFT later on
        let norm = (rows as f32 * cols as f32).sqrt();

        // STEP 2: compute the point spread function
        // this is where we offset it so that it is centered on (W - 1) / 2, (H - 1) / 2

        // I also think this can be (rows - 1) / 2 * 2 + 1
        let psf_rows = if rows % 2 == 0 { rows - 1 } else { rows };
        let psf_cols = if cols % 2 == 0 { cols - 1 } else { cols };

        let mut psf = Vec::with_capacity(psf_rows * psf_cols);

        for y in 0..psf_rows {
            for x in 0..psf_cols {
                let px = (x + (cols - 1) / 2 + 2) % cols;
                let py = (y + (rows - 1) / 2 + 2) % rows;

                psf.push(aperture_input[py * cols + px].norm_sqr() / norm);
            }
        }

        // STEP 3: generate the filter (note: this is where we scale according
        // to aspect ratio and where we scale up to the convolution resolution)

        const WAVELENGTHS: &[u8] = &[
            1, 0, 3, 0, 2, 0, 5, 0, 2, 0, 5, 0, 2, 0, 5, 0, 2, 0, 6, 0, 2, 0, 6, 0, 2, 0, 6, 0, 2,
            0, 7, 0, 2, 0, 7, 0, 3, 0, 8, 0, 3, 0, 8, 0, 3, 0, 9, 0, 3, 0, 9, 0, 3, 0, 9, 0, 4, 0,
            10, 0, 4, 0, 10, 0, 4, 0, 11, 0, 5, 0, 11, 0, 5, 0, 12, 0, 6, 0, 13, 0, 6, 0, 13, 0, 6,
            0, 14, 0, 7, 0, 15, 0, 7, 0, 15, 0, 8, 1, 16, 0, 9, 1, 17, 0, 9, 1, 19, 0, 10, 1, 20,
            0, 11, 1, 21, 0, 12, 1, 23, 0, 12, 1, 24, 0, 13, 1, 26, 0, 14, 1, 27, 0, 15, 1, 29, 0,
            16, 1, 31, 0, 17, 2, 33, 0, 19, 2, 35, 0, 20, 2, 37, 0, 21, 2, 39, 0, 23, 3, 41, 0, 24,
            3, 44, 0, 26, 3, 47, 0, 27, 4, 50, 0, 29, 4, 53, 0, 30, 4, 56, 0, 31, 5, 59, 0, 32, 5,
            63, 0, 33, 6, 66, 0, 35, 7, 69, 0, 36, 7, 73, 0, 37, 8, 76, 0, 37, 8, 80, 0, 39, 9, 83,
            0, 40, 9, 86, 0, 40, 9, 89, 0, 41, 10, 92, 0, 41, 11, 97, 0, 42, 12, 100, 0, 42, 12,
            104, 0, 42, 13, 107, 0, 42, 14, 112, 0, 42, 15, 115, 0, 42, 16, 118, 0, 41, 17, 121, 0,
            41, 17, 124, 0, 41, 19, 127, 0, 39, 22, 127, 0, 37, 25, 125, 0, 35, 28, 124, 0, 33, 31,
            124, 0, 31, 35, 123, 0, 29, 37, 121, 0, 26, 39, 120, 0, 24, 41, 119, 0, 22, 43, 118, 0,
            20, 45, 116, 0, 17, 48, 115, 0, 15, 49, 113, 0, 13, 51, 112, 0, 11, 53, 110, 0, 9, 55,
            108, 0, 9, 56, 107, 0, 9, 57, 105, 0, 9, 59, 104, 0, 9, 60, 103, 0, 9, 61, 102, 0, 10,
            63, 102, 0, 10, 64, 102, 0, 11, 66, 102, 0, 11, 67, 102, 0, 11, 69, 102, 0, 12, 71,
            103, 0, 12, 73, 103, 0, 12, 74, 104, 0, 13, 76, 104, 0, 14, 78, 106, 0, 14, 80, 106, 0,
            14, 81, 106, 0, 15, 84, 107, 0, 15, 86, 107, 0, 15, 88, 107, 0, 16, 90, 108, 0, 16, 91,
            108, 0, 16, 93, 108, 0, 16, 95, 108, 0, 16, 98, 108, 0, 17, 100, 109, 0, 17, 102, 109,
            0, 18, 104, 109, 0, 19, 106, 110, 0, 19, 108, 110, 0, 20, 110, 111, 0, 20, 112, 111, 0,
            21, 115, 111, 0, 21, 117, 111, 0, 22, 119, 111, 0, 23, 121, 112, 0, 23, 123, 112, 0,
            23, 125, 112, 0, 23, 128, 113, 0, 23, 131, 113, 0, 23, 134, 113, 0, 23, 136, 114, 0,
            23, 140, 114, 0, 25, 142, 115, 0, 25, 145, 115, 0, 26, 148, 116, 0, 26, 150, 116, 0,
            27, 153, 116, 0, 27, 157, 116, 0, 28, 159, 116, 0, 28, 162, 116, 0, 29, 164, 116, 0,
            29, 168, 116, 0, 29, 171, 116, 0, 29, 173, 115, 0, 29, 176, 115, 0, 29, 179, 115, 0,
            30, 182, 114, 0, 30, 185, 113, 0, 31, 187, 112, 0, 31, 190, 112, 0, 32, 193, 111, 0,
            32, 195, 110, 0, 33, 197, 109, 0, 33, 199, 108, 0, 33, 201, 108, 0, 33, 203, 107, 0,
            33, 205, 105, 0, 33, 208, 103, 0, 33, 210, 101, 0, 33, 212, 100, 0, 33, 214, 98, 0, 33,
            216, 95, 0, 34, 218, 92, 0, 34, 220, 90, 0, 34, 222, 87, 0, 35, 224, 84, 0, 39, 226,
            80, 0, 42, 227, 76, 0, 46, 229, 73, 0, 49, 230, 70, 0, 54, 231, 66, 0, 58, 232, 61, 0,
            64, 233, 57, 0, 69, 234, 51, 0, 73, 235, 46, 0, 79, 237, 41, 0, 85, 236, 39, 0, 92,
            236, 37, 0, 99, 235, 36, 0, 106, 235, 35, 0, 112, 235, 34, 0, 119, 234, 34, 0, 124,
            233, 34, 0, 129, 233, 34, 0, 135, 232, 34, 0, 140, 231, 34, 0, 145, 230, 33, 0, 149,
            230, 33, 0, 154, 229, 33, 0, 158, 228, 33, 0, 162, 226, 33, 0, 167, 225, 33, 0, 170,
            224, 33, 0, 174, 223, 33, 0, 178, 222, 33, 0, 182, 220, 33, 0, 186, 219, 33, 0, 189,
            218, 33, 0, 192, 217, 33, 0, 197, 216, 33, 0, 200, 215, 33, 0, 203, 214, 33, 0, 207,
            212, 32, 0, 210, 211, 32, 0, 214, 209, 32, 0, 217, 207, 32, 0, 220, 206, 32, 0, 223,
            204, 32, 0, 226, 201, 32, 0, 230, 200, 32, 0, 233, 198, 32, 0, 236, 196, 32, 0, 237,
            194, 35, 0, 238, 191, 39, 0, 239, 188, 44, 0, 240, 185, 48, 0, 241, 183, 52, 0, 242,
            181, 55, 0, 242, 179, 57, 0, 244, 176, 60, 0, 244, 174, 63, 0, 245, 172, 65, 0, 245,
            170, 67, 0, 245, 167, 68, 0, 245, 164, 70, 0, 245, 162, 71, 0, 245, 160, 72, 0, 246,
            158, 73, 0, 246, 156, 73, 0, 246, 154, 74, 0, 246, 151, 74, 0, 246, 149, 74, 0, 247,
            147, 74, 0, 247, 145, 74, 0, 248, 142, 73, 0, 248, 140, 72, 0, 248, 138, 72, 0, 249,
            135, 71, 0, 249, 132, 70, 0, 249, 130, 69, 0, 249, 127, 68, 0, 249, 125, 67, 0, 250,
            122, 66, 0, 250, 120, 64, 0, 250, 116, 62, 0, 250, 113, 60, 0, 250, 110, 58, 0, 250,
            108, 57, 0, 251, 104, 55, 0, 252, 100, 53, 0, 252, 97, 51, 0, 252, 94, 49, 0, 253, 91,
            46, 0, 252, 87, 42, 0, 252, 82, 40, 0, 252, 79, 37, 0, 252, 75, 34, 0, 252, 70, 31, 0,
            251, 66, 28, 0, 250, 62, 25, 0, 249, 58, 22, 0, 248, 53, 19, 0, 247, 48, 15, 0, 245,
            46, 14, 0, 242, 43, 14, 0, 240, 40, 13, 0, 238, 37, 13, 0, 235, 35, 13, 0, 230, 34, 14,
            0, 226, 34, 16, 0, 222, 34, 17, 0, 218, 34, 19, 0, 213, 34, 20, 0, 209, 34, 21, 0, 205,
            34, 21, 0, 201, 34, 22, 0, 197, 34, 22, 0, 193, 35, 23, 0, 189, 35, 22, 0, 184, 35, 20,
            0, 180, 35, 17, 0, 176, 35, 14, 0, 172, 35, 10, 0, 167, 34, 8, 0, 164, 34, 8, 0, 161,
            33, 8, 0, 158, 32, 7, 0, 154, 31, 7, 0, 151, 31, 7, 0, 148, 30, 6, 0, 144, 29, 6, 0,
            140, 29, 6, 0, 137, 27, 6, 0, 134, 26, 5, 0, 131, 26, 5, 0, 128, 25, 5, 0, 125, 25, 5,
            0, 122, 24, 5, 0, 119, 24, 5, 0, 116, 23, 4, 0, 113, 23, 4, 0, 109, 22, 4, 0, 106, 21,
            4, 0, 103, 21, 4, 0, 100, 20, 3, 0, 98, 20, 3, 0, 95, 19, 3, 0, 92, 19, 3, 0, 90, 19,
            3, 0, 88, 18, 3, 0, 85, 18, 3, 0, 82, 17, 3, 0, 80, 17, 3, 0, 78, 17, 3, 0, 76, 16, 2,
            0, 74, 16, 2, 0, 72, 15, 2, 0, 69, 15, 2, 0, 67, 14, 2, 0, 65, 14, 2, 0, 63, 13, 1, 0,
            61, 12, 1, 0, 59, 12, 1, 0, 57, 12, 1, 0, 54, 11, 1, 0, 52, 11, 1, 0, 50, 11, 1, 0, 49,
            10, 1, 0, 47, 10, 1, 0, 46, 10, 1, 0, 44, 9, 1, 0, 43, 9, 1, 0, 41, 8, 1, 0, 40, 8, 1,
            0, 39, 8, 1, 0, 37, 7, 0, 0, 36, 7, 0, 0, 35, 6, 0, 0, 34, 6, 0, 0, 32, 5, 0, 0, 32, 5,
            0, 0, 31, 5, 0, 0, 31, 4, 0, 0, 30, 4, 0, 0, 30, 4, 0, 0, 29, 3, 0, 0, 28, 3, 0, 0, 28,
            3, 0, 0, 27, 3, 0, 0, 25, 3, 0, 0, 25, 2, 0, 0, 24, 2, 0, 0, 24, 2, 0, 0, 23, 2, 0, 0,
            23, 2, 0, 0, 22, 1, 0, 0, 21, 1, 0, 0, 21, 1, 0, 0, 20, 1, 0, 0, 19, 1, 0, 0, 18, 1, 0,
            0, 18, 1, 0, 0, 17, 1, 0, 0, 17, 1, 0, 0, 16, 1, 0, 0, 15, 1, 0, 0, 14, 1, 0, 0, 14, 1,
            0, 0, 13, 1, 0, 0, 12, 1, 0, 0, 11, 1, 0, 0, 11, 1, 0, 0, 10, 1, 0, 0, 9, 1, 0, 0, 8,
            1, 0, 0, 8, 1, 0, 0, 7, 1, 0, 0, 6, 1, 0, 0,
        ];

        /*

        what we HAVE:
            - the PSF, of odd dimensions, properly centered

        what we WANT:
            - the same PSF, overlaid many times according to a given scale for each wavelength
              into a buffer of size 2048 x 1024, shifted by 2048 / 2 - 1 to be centered on (0, 0)

        */

        // upload these wavelengths to a 1D texture, and sample them during filter
        // processing prepare an RGBA texture to store the results in, and an
        // FBO to render into it the output texture will have size 1024 x 512,
        // and the shader will shift the pixels by 2048 / 2 - 1, and the central
        // row/col (for even row/col) will be zero (unused) only at that point
        // will it be ready to be convolved

        let mut filter_data = vec![0.0f32; 4 * 2048 * 1024];

        let mut total_r = 0.0;
        let mut total_g = 0.0;
        let mut total_b = 0.0;

        let z0 = 2.0;

        for y in 0..1023 {
            for x in 0..2047 {
                for wavelength in 380..750 {
                    let scale_x = z0 * (wavelength as f32) / 749.0 * 1.77777;
                    let scale_y = z0 * (wavelength as f32) / 749.0;

                    let mut px = (x as f32 + 0.5) / 2047.0;
                    let mut py = (y as f32 + 0.5) / 1023.0;

                    px = (px - 0.5) * scale_x + 0.5;
                    py = (py - 0.5) * scale_y + 0.5;

                    let value = bilinear_interpolation(&psf, psf_cols, psf_rows, px, py);

                    let spectrum_r = WAVELENGTHS[4 * (wavelength - 380) + 0] as f32 / 255.0;
                    let spectrum_g = WAVELENGTHS[4 * (wavelength - 380) + 1] as f32 / 255.0;
                    let spectrum_b = WAVELENGTHS[4 * (wavelength - 380) + 2] as f32 / 255.0;

                    // convert this to a spectral color
                    let r = spectrum_r * value;
                    let g = spectrum_g * value;
                    let b = spectrum_b * value;

                    total_r += r;
                    total_g += g;
                    total_b += b;

                    let offset = y * 2048 + x;

                    filter_data[4 * offset + 0] += r;
                    filter_data[4 * offset + 1] += g;
                    filter_data[4 * offset + 2] += b;
                }
            }
        }

        for i in 0..(2048 * 1024) {
            filter_data[4 * i + 0] /= total_r;
            filter_data[4 * i + 1] /= total_g;
            filter_data[4 * i + 2] /= total_b;
        }

        let mid_x = (2048 - 1) / 2;
        let mid_y = (1024 - 1) / 2;

        info!("R weight = {}", filter_data[4 * (mid_y * 2048 + mid_x) + 0]);
        info!("G weight = {}", filter_data[4 * (mid_y * 2048 + mid_x) + 1]);
        info!("B weight = {}", filter_data[4 * (mid_y * 2048 + mid_x) + 2]);

        filter_data[4 * (mid_y * 2048 + mid_x) + 0] = 0.0;
        filter_data[4 * (mid_y * 2048 + mid_x) + 1] = 0.0;
        filter_data[4 * (mid_y * 2048 + mid_x) + 2] = 0.0;

        // filter.upload(1024, 512, &filter_data);

        let mut r_filter_data = vec![0.0; 2 * 1024 * 2048];
        let mut g_filter_data = vec![0.0; 2 * 1024 * 2048];
        let mut b_filter_data = vec![0.0; 2 * 1024 * 2048];

        for y in 0..1024 {
            for x in 0..2048 {
                // shift the data here...
                let sx = (x + 2047 / 2) % 2048;
                let sy = (y + 1023 / 2) % 1024;

                let r = filter_data[4 * (sy * 2048 + sx) + 0];
                let g = filter_data[4 * (sy * 2048 + sx) + 1];
                let b = filter_data[4 * (sy * 2048 + sx) + 2];

                r_filter_data[2 * (y * 2048 + x)] = r;
                g_filter_data[2 * (y * 2048 + x)] = g;
                b_filter_data[2 * (y * 2048 + x)] = b;
            }
        }

        self.rspectrum_temp1.upload(2048, 1024, &r_filter_data);
        self.gspectrum_temp1.upload(2048, 1024, &g_filter_data);
        self.bspectrum_temp1.upload(2048, 1024, &b_filter_data);

        // STEP 4: compute FFT (we can actually reuse the FFT passes previously defined,
        // just stopping at the forward rows/columns and without requesting any
        // convolution) so at this point all we need is to just load the
        // convolution filter, and run the FFT passes. but... can we just load
        // it into the bottom left as before?  => yes we can
        // so we just need to have an RGB texture for the RGB convolution filter, and we
        // can just invoke the load_into_convolution_buffers and FFT shaders,
        // outputting the final result into the aperture spectrum buffers!

        let mut location = DataLocation::Temp1; // self.load_into_convolution_buffers(&filter);

        let mut passes = vec![];

        // forward passes, rows

        let mut m = 2;

        while m <= 2048 {
            for _ in 0..3 {
                passes.push(FFTPassData {
                    horizontal: 1,
                    direction: 1,                // "forward"
                    subtransform_size: 4096 / m, // inverse order
                    convolve: 0,
                });
            }

            m *= 2;
        }

        // forward passes, columns

        let mut m = 2;

        while m <= 1024 {
            for _ in 0..3 {
                passes.push(FFTPassData {
                    horizontal: 0,
                    direction: 1,                // "forward"
                    subtransform_size: 2048 / m, // inverse order
                    convolve: 0,
                });
            }

            m *= 2;
        }

        let mut vertex_array = VertexArray::new(self.gl.clone());
        vertex_array.upload(&passes);

        let command = self.fft_shader.begin_draw();

        command.set_viewport(0, 0, 2048, 1024);

        command.set_vertex_array(&vertex_array);

        for triangle_index in 0..(vertex_array.vertex_count() / 3) {
            command.bind(self.source_r_buffer(location), "r_conv_buffer");
            command.bind(self.source_g_buffer(location), "g_conv_buffer");
            command.bind(self.source_b_buffer(location), "b_conv_buffer");

            // these are not used, just to shut up WebGL
            command.bind(self.source_r_buffer(location), "r_conv_filter");
            command.bind(self.source_g_buffer(location), "g_conv_filter");
            command.bind(self.source_b_buffer(location), "b_conv_filter");

            if triangle_index == vertex_array.vertex_count() / 3 - 1 {
                // final iteration
                command.set_framebuffer(&self.aperture_fbo);
            } else {
                command.set_framebuffer(self.target_framebuffer(location));
            }

            command.draw_triangles(triangle_index, 1);

            location.swap();
        }

        // that's it, we're done, the aperture convolution filter has been
        // initialized
    }

    pub(crate) fn prepare_fft_pass_data(&mut self) {
        let mut passes = vec![];

        // forward passes, rows

        let mut m = 2;

        while m <= 2048 {
            for _ in 0..3 {
                passes.push(FFTPassData {
                    horizontal: 1,
                    direction: 1,                // "forward"
                    subtransform_size: 4096 / m, // inverse order
                    convolve: 0,
                });
            }

            m *= 2;
        }

        // forward passes, columns

        let mut m = 2;

        while m <= 1024 {
            for _ in 0..3 {
                passes.push(FFTPassData {
                    horizontal: 0,
                    direction: 1,                // "forward"
                    subtransform_size: 2048 / m, // inverse order
                    convolve: (m == 1024) as u16,
                });
            }

            m *= 2;
        }

        // inverse passes, columns

        let mut m = 2;

        while m <= 1024 {
            for _ in 0..3 {
                passes.push(FFTPassData {
                    horizontal: 0,
                    direction: 0, // "inverse"
                    subtransform_size: m,
                    convolve: 0, // m == 0 if we want to do it inline here?
                });
            }

            m *= 2;
        }

        // inverse passes, rows

        let mut m = 2;

        while m <= 2048 {
            for _ in 0..3 {
                passes.push(FFTPassData {
                    horizontal: 1,
                    direction: 0, // "inverse"
                    subtransform_size: m,
                    convolve: 0,
                });
            }

            m *= 2;
        }

        self.fft_pass_data.upload(&passes);
    }

    fn load_into_convolution_buffers(&self, texture: &Texture<RGBA32F>) -> DataLocation {
        let command = self.load_convolution_buffers_shader.begin_draw();

        command.bind(texture, "image");

        command.set_viewport(0, 0, 2048, 1024);
        command.set_framebuffer(&self.spectrum_temp1_fbo);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);

        DataLocation::Temp1
    }

    fn source_r_buffer(&self, location: DataLocation) -> &Texture<RG32F> {
        match location {
            DataLocation::Temp1 => &self.rspectrum_temp1,
            DataLocation::Temp2 => &self.rspectrum_temp2,
        }
    }

    fn source_g_buffer(&self, location: DataLocation) -> &Texture<RG32F> {
        match location {
            DataLocation::Temp1 => &self.gspectrum_temp1,
            DataLocation::Temp2 => &self.gspectrum_temp2,
        }
    }

    fn source_b_buffer(&self, location: DataLocation) -> &Texture<RG32F> {
        match location {
            DataLocation::Temp1 => &self.bspectrum_temp1,
            DataLocation::Temp2 => &self.bspectrum_temp2,
        }
    }

    fn target_framebuffer(&self, location: DataLocation) -> &Framebuffer {
        match location {
            DataLocation::Temp1 => &self.spectrum_temp2_fbo,
            DataLocation::Temp2 => &self.spectrum_temp1_fbo,
        }
    }

    fn perform_convolution(&self, location: &mut DataLocation) {
        let command = self.fft_shader.begin_draw();

        command.set_vertex_array(&self.fft_pass_data);

        command.bind(&self.r_aperture_spectrum, "r_conv_filter");
        command.bind(&self.g_aperture_spectrum, "g_conv_filter");
        command.bind(&self.b_aperture_spectrum, "b_conv_filter");

        command.set_viewport(0, 0, 2048, 1024);

        for triangle_index in 0..(self.fft_pass_data.vertex_count() / 3) {
            command.bind(self.source_r_buffer(*location), "r_conv_buffer");
            command.bind(self.source_g_buffer(*location), "g_conv_buffer");
            command.bind(self.source_b_buffer(*location), "b_conv_buffer");

            command.set_framebuffer(self.target_framebuffer(*location));

            command.draw_triangles(triangle_index, 1);

            location.swap();
        }
    }

    fn load_convolved_render_from_convolution_buffers(&self, location: &mut DataLocation) {
        let command = self.read_convolution_buffers_shader.begin_draw();

        command.bind(self.source_r_buffer(*location), "r_conv_buffer");
        command.bind(self.source_g_buffer(*location), "g_conv_buffer");
        command.bind(self.source_b_buffer(*location), "b_conv_buffer");

        command.bind(&self.integrator_radiance_estimate, "source");

        command.set_framebuffer(&self.render_fbo);

        command.set_viewport(0, 0, self.render.cols() as i32, self.render.rows() as i32);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);
    }
}

#[derive(Clone, Copy)]
enum DataLocation {
    Temp1,
    Temp2,
}

impl DataLocation {
    pub fn swap(&mut self) {
        match self {
            DataLocation::Temp1 => *self = DataLocation::Temp2,
            DataLocation::Temp2 => *self = DataLocation::Temp1,
        }
    }
}
