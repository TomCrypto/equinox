#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Device;
use crate::Framebuffer;
use crate::{Texture, RG32F};
use crate::{VertexAttribute, VertexAttributeKind, VertexLayout};

use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
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
//  - larger radix to reduce number of iterations
//  - complex-to-real/real-to-complex FFT speedups
//  - reducing number of state switches...

impl Device {
    pub(crate) fn render_lens_flare(&mut self) {
        let mut location = self.load_path_traced_render_into_convolution_buffers();
        self.perform_convolution(&mut location);
        self.load_convolved_render_from_convolution_buffers(&mut location);
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

        info!("FFT passes: {:?}", passes);

        self.fft_pass_data.upload(&passes);
    }

    // TODO: in the future, pack the real data into a half-size complex FFT and do
    // the convolution using that, to get hopefully a 2x speed boost?
    // TODO: see if we can just compute the FFT of the non-padded data and then
    // "stretch" it as-if it had been zero-padded? not sure if possible

    fn load_path_traced_render_into_convolution_buffers(&mut self) -> DataLocation {
        let command = self.load_convolution_buffers.begin_draw();

        command.bind(&self.samples, "image");

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

    fn perform_convolution(&mut self, location: &mut DataLocation) {
        let command = self.fft_shader.begin_draw();

        command.set_vertex_array(&self.fft_pass_data);

        command.bind(&self.r_aperture_spectrum, "r_aperture_input");
        command.bind(&self.g_aperture_spectrum, "g_aperture_input");
        command.bind(&self.b_aperture_spectrum, "b_aperture_input");

        command.set_viewport(0, 0, 2048, 1024);

        for triangle_index in 0..(self.fft_pass_data.vertex_count() / 3) {
            command.bind(self.source_r_buffer(*location), "r_spectrum_input");
            command.bind(self.source_g_buffer(*location), "g_spectrum_input");
            command.bind(self.source_b_buffer(*location), "b_spectrum_input");

            command.set_framebuffer(self.target_framebuffer(*location));

            command.draw_triangles(triangle_index, 1);

            location.swap();
        }

        self.fft_pass_data.unbind();
    }

    fn load_convolved_render_from_convolution_buffers(&mut self, location: &mut DataLocation) {
        let command = self.copy_from_spectrum_shader.begin_draw();

        command.bind(self.source_r_buffer(*location), "r_spectrum");
        command.bind(self.source_g_buffer(*location), "g_spectrum");
        command.bind(self.source_b_buffer(*location), "b_spectrum");

        // shader.bind(&self.samples, "add");
        // shader.bind(&self.conv_source, "subtract");

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
