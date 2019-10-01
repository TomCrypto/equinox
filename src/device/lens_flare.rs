#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::Device;
use crate::DrawOptions;

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

impl Device {
    fn prepare_aperture(&mut self) {
        self.spectrum_temp2_fbo.clear(0, [0.0, 0.0, 0.0, 0.0]);
        self.spectrum_temp2_fbo.clear(1, [0.0, 0.0, 0.0, 0.0]);
        self.spectrum_temp2_fbo.clear(2, [0.0, 0.0, 0.0, 0.0]);

        self.spectrum_temp1_fbo.clear(0, [0.0, 0.0, 0.0, 0.0]);
        self.spectrum_temp1_fbo.clear(1, [0.0, 0.0, 0.0, 0.0]);
        self.spectrum_temp1_fbo.clear(2, [0.0, 0.0, 0.0, 0.0]);

        // 1. create the source data...

        self.draw_aperture_shader.bind_to_pipeline(|_| {});

        self.aperture_source_fbo.draw(DrawOptions {
            viewport: [0, 0, 2048, 1024],
            scissor: None,
            blend: None,
        });

        // 2. take its FFT to get the spectrum...

        self.copy_into_spectrum_shader.bind_to_pipeline(|shader| {
            shader.bind(&self.aperture_source, "source");
        });

        self.spectrum_temp1_fbo.draw(DrawOptions {
            viewport: [0, 0, 1024, 512],
            scissor: None,
            blend: None,
        });

        let mut current_input = true; // FALSE = temp2, TRUE = temp1

        let transform_size = 2048;
        let row_iterations = 11; // log2(2048)

        for i in 0..row_iterations {
            let subtransform_size = 2i32.pow(i + 1);

            self.fft_buffer.write(&FFTData {
                direction: 1.0,
                horizontal: 1.0,
                transform_size,
                subtransform_size,
            });

            if current_input {
                // render into TEMP2 from TEMP1

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp1, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp1, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp1, "b_spectrum_input");
                });

                self.spectrum_temp2_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            } else {
                // render into TEMP1 from TEMP2

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp2, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp2, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp2, "b_spectrum_input");
                });

                self.spectrum_temp1_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            }

            current_input = !current_input;
        }

        // STEP 3: perform the per-column FFT

        let transform_size = 1024;
        let row_iterations = 10; // log2(1024)

        for i in 0..row_iterations {
            let subtransform_size = 2i32.pow(i + 1);

            self.fft_buffer.write(&FFTData {
                direction: 1.0,
                horizontal: 0.0,
                transform_size,
                subtransform_size,
            });

            if current_input {
                // render into TEMP2 from TEMP1

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp1, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp1, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp1, "b_spectrum_input");
                });

                self.spectrum_temp2_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            } else {
                // render into TEMP1 from TEMP2

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp2, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp2, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp2, "b_spectrum_input");
                });

                self.spectrum_temp1_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            }

            current_input = !current_input;
        }

        let transform_size = 2048;
        let row_iterations = 11; // log2(2048)

        for i in 0..row_iterations {
            let subtransform_size = 2i32.pow(i + 1);

            self.fft_buffer.write(&FFTData {
                direction: -1.0,
                horizontal: 1.0,
                transform_size,
                subtransform_size,
            });

            if current_input {
                // render into TEMP2 from TEMP1

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp1, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp1, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp1, "b_spectrum_input");
                });

                self.spectrum_temp2_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            } else {
                // render into TEMP1 from TEMP2

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp2, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp2, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp2, "b_spectrum_input");
                });

                self.spectrum_temp1_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            }

            current_input = !current_input;
        }

        // STEP 6: perform per-column IFFT

        let transform_size = 1024;
        let row_iterations = 10; // log2(1024)

        for i in 0..row_iterations {
            let subtransform_size = 2i32.pow(i + 1);

            self.fft_buffer.write(&FFTData {
                direction: -1.0,
                horizontal: 0.0,
                transform_size,
                subtransform_size,
            });

            if current_input {
                // render into TEMP2 from TEMP1

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp1, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp1, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp1, "b_spectrum_input");
                });

                if i == row_iterations - 1 {
                    self.aperture_fbo.draw(DrawOptions {
                        viewport: [0, 0, 2048, 1024],
                        scissor: None,
                        blend: None,
                    });
                } else {
                    self.spectrum_temp2_fbo.draw(DrawOptions {
                        viewport: [0, 0, 2048, 1024],
                        scissor: None,
                        blend: None,
                    });
                }
            } else {
                // render into TEMP1 from TEMP2

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp2, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp2, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp2, "b_spectrum_input");
                });

                if i == row_iterations - 1 {
                    self.aperture_fbo.draw(DrawOptions {
                        viewport: [0, 0, 2048, 1024],
                        scissor: None,
                        blend: None,
                    });
                } else {
                    self.spectrum_temp1_fbo.draw(DrawOptions {
                        viewport: [0, 0, 2048, 1024],
                        scissor: None,
                        blend: None,
                    });
                }
            }

            current_input = !current_input;
        }
    }

    pub(crate) fn render_lens_flare(&mut self) {
        // self.prepare_aperture();

        // STEP 1: copy the data for each channel of the samples buffer into
        // temp buffers R1, G1, B1 (these are complex RG32F textures),
        // zero-padded accordingly

        /*

        Resizing logic: the spectrum buffers are defined to be 2048x1024

        we need to copy the source image into the lower 1024x512 pixels of this

        */

        self.spectrum_temp2_fbo.clear(0, [0.0, 0.0, 0.0, 0.0]);
        self.spectrum_temp2_fbo.clear(1, [0.0, 0.0, 0.0, 0.0]);
        self.spectrum_temp2_fbo.clear(2, [0.0, 0.0, 0.0, 0.0]);

        self.spectrum_temp1_fbo.clear(0, [0.0, 0.0, 0.0, 0.0]);
        self.spectrum_temp1_fbo.clear(1, [0.0, 0.0, 0.0, 0.0]);
        self.spectrum_temp1_fbo.clear(2, [0.0, 0.0, 0.0, 0.0]);

        self.direct_copy_shader.bind_to_pipeline(|shader| {
            shader.bind(&self.samples, "source");
        });

        self.conv_fbo.draw(DrawOptions {
            viewport: [0, 0, 1024, 512],
            scissor: Some([0, 0, 1024, 512]),
            blend: None,
        });

        self.copy_into_spectrum_shader.bind_to_pipeline(|shader| {
            shader.bind(&self.samples, "source");
        });

        self.spectrum_temp1_fbo.draw(DrawOptions {
            viewport: [0, 0, 1024, 512],
            scissor: Some([0, 0, 1024, 512]),
            blend: None,
        });

        let mut current_input = true; // FALSE = temp2, TRUE = temp1

        // STEP 2: perform the per-row FFT concurrently for all three buffers,
        // ping-ponging between temp1 and temp2

        let transform_size = 2048;
        let row_iterations = 11; // log2(2048)

        for i in 0..row_iterations {
            let subtransform_size = 2i32.pow(i + 1);

            self.fft_buffer.write(&FFTData {
                direction: 1.0,
                horizontal: 1.0,
                transform_size,
                subtransform_size,
            });

            if current_input {
                // render into TEMP2 from TEMP1

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp1, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp1, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp1, "b_spectrum_input");
                });

                self.spectrum_temp2_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            } else {
                // render into TEMP1 from TEMP2

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp2, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp2, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp2, "b_spectrum_input");
                });

                self.spectrum_temp1_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            }

            current_input = !current_input;
        }

        // STEP 3: perform the per-column FFT

        let transform_size = 1024;
        let row_iterations = 10; // log2(1024)

        for i in 0..row_iterations {
            let subtransform_size = 2i32.pow(i + 1);

            self.fft_buffer.write(&FFTData {
                direction: 1.0,
                horizontal: 0.0,
                transform_size,
                subtransform_size,
            });

            if current_input {
                // render into TEMP2 from TEMP1

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp1, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp1, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp1, "b_spectrum_input");
                });

                self.spectrum_temp2_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            } else {
                // render into TEMP1 from TEMP2

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp2, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp2, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp2, "b_spectrum_input");
                });

                self.spectrum_temp1_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            }

            current_input = !current_input;
        }

        // STEP 4: point-wise multiply with the aperture FFT (it too has three
        // channels)

        self.pointwise_multiply_shader.bind_to_pipeline(|shader| {
            if current_input {
                shader.bind(&self.rspectrum_temp1, "r_spectrum_input");
                shader.bind(&self.gspectrum_temp1, "g_spectrum_input");
                shader.bind(&self.bspectrum_temp1, "b_spectrum_input");
            } else {
                shader.bind(&self.rspectrum_temp2, "r_spectrum_input");
                shader.bind(&self.gspectrum_temp2, "g_spectrum_input");
                shader.bind(&self.bspectrum_temp2, "b_spectrum_input");
            }

            shader.bind(&self.r_aperture_spectrum, "r_aperture_input");
            shader.bind(&self.g_aperture_spectrum, "g_aperture_input");
            shader.bind(&self.b_aperture_spectrum, "b_aperture_input");
        });

        if current_input {
            self.spectrum_temp2_fbo.draw(DrawOptions {
                viewport: [0, 0, 2048, 1024],
                scissor: None,
                blend: None,
            });
        } else {
            self.spectrum_temp1_fbo.draw(DrawOptions {
                viewport: [0, 0, 2048, 1024],
                scissor: None,
                blend: None,
            });
        }

        current_input = !current_input;

        // STEP 5: perform per-row IFFT

        let transform_size = 2048;
        let row_iterations = 11; // log2(2048)

        for i in 0..row_iterations {
            let subtransform_size = 2i32.pow(i + 1);

            self.fft_buffer.write(&FFTData {
                direction: -1.0,
                horizontal: 1.0,
                transform_size,
                subtransform_size,
            });

            if current_input {
                // render into TEMP2 from TEMP1

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp1, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp1, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp1, "b_spectrum_input");
                });

                self.spectrum_temp2_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            } else {
                // render into TEMP1 from TEMP2

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp2, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp2, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp2, "b_spectrum_input");
                });

                self.spectrum_temp1_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            }

            current_input = !current_input;
        }

        // STEP 6: perform per-column IFFT

        let transform_size = 1024;
        let row_iterations = 10; // log2(1024)

        for i in 0..row_iterations {
            let subtransform_size = 2i32.pow(i + 1);

            self.fft_buffer.write(&FFTData {
                direction: -1.0,
                horizontal: 0.0,
                transform_size,
                subtransform_size,
            });

            if current_input {
                // render into TEMP2 from TEMP1

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp1, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp1, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp1, "b_spectrum_input");
                });

                self.spectrum_temp2_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            } else {
                // render into TEMP1 from TEMP2

                self.fft_shader.bind_to_pipeline(|shader| {
                    shader.bind(&self.fft_buffer, "FFT");
                    shader.bind(&self.rspectrum_temp2, "r_spectrum_input");
                    shader.bind(&self.gspectrum_temp2, "g_spectrum_input");
                    shader.bind(&self.bspectrum_temp2, "b_spectrum_input");
                });

                self.spectrum_temp1_fbo.draw(DrawOptions {
                    viewport: [0, 0, 2048, 1024],
                    scissor: None,
                    blend: None,
                });
            }

            current_input = !current_input;
        }

        // STEP 7: recombine the three channels into the final "render" buffer

        /*

        Resize logic: we need to copy the spectrum data out into the final render buffer, but
        only the first 1024x512 pixels. So render over the entire render buffer, but only sample
        half of the texture by multiplying the UV by 0.5

        */

        self.copy_from_spectrum_shader.bind_to_pipeline(|shader| {
            if current_input {
                // render from TEMP1

                shader.bind(&self.rspectrum_temp1, "r_spectrum");
                shader.bind(&self.gspectrum_temp1, "g_spectrum");
                shader.bind(&self.bspectrum_temp1, "b_spectrum");
            } else {
                // render from TEMP2

                shader.bind(&self.rspectrum_temp2, "r_spectrum");
                shader.bind(&self.gspectrum_temp2, "g_spectrum");
                shader.bind(&self.bspectrum_temp2, "b_spectrum");
            }

            shader.bind(&self.samples, "add");
            shader.bind(&self.conv_source, "subtract");
        });

        self.render_fbo.draw(DrawOptions {
            viewport: [0, 0, self.render.cols() as i32, self.render.rows() as i32],
            scissor: None,
            blend: None,
        });
    }

    // TODO: in the future, pack the real data into a half-size complex FFT and do
    // the convolution using that, to get hopefully a 2x speed boost?
    // TODO: see if we can just compute the FFT of the non-padded data and then
    // "stretch" it as-if it had been zero-padded? not sure if possible

    /*

    The aperture will be modified whenever the aspect ratio changes, to scale it
    accordingly to ensure the lens flares are correctly scaled

    If our convolution resolution is MxN, then the render resolution will be
    (M / 2)x(N / 2) and the aperture resolution is (M / 2 - 1)x(N / 2 - 1)

    This ensures there is no bleeding, and because the aperture has an odd
    resolution there is a single center which can be centered on zero, which
    should make edge effects disappear

    Can pick radix-16 algorithm and radix-8, so we can do:

    M = 16 * 16 = 256
    N = 8 * 8 = 64

    */

    fn perform_convolution(&mut self) {
        // Compute the (normalized) FFT of all three channels of the image to be
        // convolved. The first pass loads each channel from the source texture.

        // TODO

        // Perform a point-wise multiplication of the convolution buffers in the
        // frequency domain with the (also frequency-domain) aperture buffers.

        // TODO

        // Compute the inverse FFT of all three channels, which recovers the
        // convolved result in the spatial domain. The final pass loads each
        // channel into the output texture now ready for further processing.

        // TODO
    }
}

enum DataLocation {
    Temp1,
    Temp2,
}
