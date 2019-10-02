#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Device;
use crate::DrawOptions;
use crate::Framebuffer;
use crate::{Texture, RG32F};

use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(AsBytes, FromBytes, Debug)]
pub struct FFTData {
    pub transform_size: i32,
    pub subtransform_size: i32,
    pub horizontal: f32,
    pub direction: f32,
}

// TODO: possible speed-ups:
//  - larger radix to reduce number of iterations
//  - complex-to-real/real-to-complex FFT speedups
//  - reducing number of state switches...

impl Device {
    pub(crate) fn render_lens_flare(&mut self) {
        let mut location = self.load_path_traced_render_into_convolution_buffers();
        self.perform_forward_fft(&mut location);
        self.perform_pointwise_multiplication(&mut location);
        self.perform_inverse_fft(&mut location);
        self.load_convolved_render_from_convolution_buffers(&mut location);
    }

    // TODO: in the future, pack the real data into a half-size complex FFT and do
    // the convolution using that, to get hopefully a 2x speed boost?
    // TODO: see if we can just compute the FFT of the non-padded data and then
    // "stretch" it as-if it had been zero-padded? not sure if possible

    fn load_path_traced_render_into_convolution_buffers(&mut self) -> DataLocation {
        self.load_convolution_buffers.bind_to_pipeline(|shader| {
            shader.bind(&self.samples, "image");
        });

        self.spectrum_temp1_fbo.draw(DrawOptions {
            viewport: [0, 0, 2048, 1024], // TODO: get this data from a central place
            scissor: None,
            blend: None,
        });

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

    // TODO: might be a way to speed things up here by avoiding unnecessary
    // useProgram (which I suspect might not be the fastest operation out there)

    fn perform_forward_fft(&mut self, location: &mut DataLocation) {
        // per-row pass, transform size will be row size

        let (mut s, mut m) = (1, 2);

        while m <= 2048 {
            self.fft_buffer.write(&FFTData {
                direction: 1.0,
                horizontal: 1.0,
                transform_size: 2048,
                subtransform_size: m,
            });

            self.fft_shader.bind_to_pipeline(|shader| {
                shader.bind(&self.fft_buffer, "FFT");

                shader.bind(self.source_r_buffer(*location), "r_spectrum_input");
                shader.bind(self.source_g_buffer(*location), "g_spectrum_input");
                shader.bind(self.source_b_buffer(*location), "b_spectrum_input");
            });

            self.target_framebuffer(*location).draw(DrawOptions {
                viewport: [0, 0, 2048, 1024],
                scissor: None,
                blend: None,
            });

            location.swap();

            s += 1;
            m *= 2;
        }

        // per-column pass, transform size will be column size

        let (mut s, mut m) = (1, 2);

        while m <= 1024 {
            self.fft_buffer.write(&FFTData {
                direction: 1.0,
                horizontal: 0.0,
                transform_size: 1024,
                subtransform_size: m,
            });

            self.fft_shader.bind_to_pipeline(|shader| {
                shader.bind(&self.fft_buffer, "FFT");

                shader.bind(self.source_r_buffer(*location), "r_spectrum_input");
                shader.bind(self.source_g_buffer(*location), "g_spectrum_input");
                shader.bind(self.source_b_buffer(*location), "b_spectrum_input");
            });

            self.target_framebuffer(*location).draw(DrawOptions {
                viewport: [0, 0, 2048, 1024],
                scissor: None,
                blend: None,
            });

            location.swap();

            s += 1;
            m *= 2;
        }
    }

    fn perform_inverse_fft(&mut self, location: &mut DataLocation) {
        // per-row pass, transform size will be row size

        let (mut s, mut m) = (1, 2);

        while m <= 2048 {
            self.fft_buffer.write(&FFTData {
                direction: -1.0,
                horizontal: 1.0,
                transform_size: 2048,
                subtransform_size: m,
            });

            self.fft_shader.bind_to_pipeline(|shader| {
                shader.bind(&self.fft_buffer, "FFT");

                shader.bind(self.source_r_buffer(*location), "r_spectrum_input");
                shader.bind(self.source_g_buffer(*location), "g_spectrum_input");
                shader.bind(self.source_b_buffer(*location), "b_spectrum_input");
            });

            self.target_framebuffer(*location).draw(DrawOptions {
                viewport: [0, 0, 2048, 1024],
                scissor: None,
                blend: None,
            });

            location.swap();

            s += 1;
            m *= 2;
        }

        // per-column pass, transform size will be column size

        let (mut s, mut m) = (1, 2);

        while m <= 1024 {
            self.fft_buffer.write(&FFTData {
                direction: -1.0,
                horizontal: 0.0,
                transform_size: 1024,
                subtransform_size: m,
            });

            self.fft_shader.bind_to_pipeline(|shader| {
                shader.bind(&self.fft_buffer, "FFT");

                shader.bind(self.source_r_buffer(*location), "r_spectrum_input");
                shader.bind(self.source_g_buffer(*location), "g_spectrum_input");
                shader.bind(self.source_b_buffer(*location), "b_spectrum_input");
            });

            self.target_framebuffer(*location).draw(DrawOptions {
                viewport: [0, 0, 2048, 1024],
                scissor: None,
                blend: None,
            });

            location.swap();

            s += 1;
            m *= 2;
        }
    }

    fn perform_pointwise_multiplication(&mut self, location: &mut DataLocation) {
        self.pointwise_multiply_shader.bind_to_pipeline(|shader| {
            shader.bind(self.source_r_buffer(*location), "r_spectrum_input");
            shader.bind(self.source_g_buffer(*location), "g_spectrum_input");
            shader.bind(self.source_b_buffer(*location), "b_spectrum_input");

            shader.bind(&self.r_aperture_spectrum, "r_aperture_input");
            shader.bind(&self.g_aperture_spectrum, "g_aperture_input");
            shader.bind(&self.b_aperture_spectrum, "b_aperture_input");
        });

        self.target_framebuffer(*location).draw(DrawOptions {
            viewport: [0, 0, 2048, 1024],
            scissor: None,
            blend: None,
        });

        location.swap();
    }

    fn load_convolved_render_from_convolution_buffers(&mut self, location: &mut DataLocation) {
        self.copy_from_spectrum_shader.bind_to_pipeline(|shader| {
            shader.bind(self.source_r_buffer(*location), "r_spectrum");
            shader.bind(self.source_g_buffer(*location), "g_spectrum");
            shader.bind(self.source_b_buffer(*location), "b_spectrum");

            // shader.bind(&self.samples, "add");
            // shader.bind(&self.conv_source, "subtract");
        });

        self.render_fbo.draw(DrawOptions {
            viewport: [0, 0, self.render.cols() as i32, self.render.rows() as i32],
            scissor: None,
            blend: None,
        });
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
