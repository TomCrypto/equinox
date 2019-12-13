#![allow(clippy::all)] // this feature is on hold for the moment

#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{
    Aperture, BlendMode, ConvolutionTileSize, Device, Framebuffer, Texture, VertexAttribute,
    VertexAttributeKind, VertexLayout, RGBA16F,
};
use img2raw::{ColorSpace, DataFormat, Header};
use itertools::{iproduct, Itertools};
use js_sys::Error;
use std::collections::HashMap;
use std::iter::repeat;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

#[repr(align(8), C)]
#[derive(AsBytes, FromBytes, Clone, Copy, Debug)]
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

impl Device {
    pub(crate) fn update_aperture_filter(
        &mut self,
        aperture: &Aperture,
        assets: &HashMap<String, Vec<u8>>,
    ) -> Result<(), Error> {
        let (header, data) =
            LayoutVerified::<_, Header>::new_from_prefix(assets[&aperture.filter].as_slice())
                .unwrap();

        if header.data_format.try_parse() != Some(DataFormat::RGBA16F) {
            return Err(Error::new("expected RGBA16F aperture filter"));
        }

        if header.color_space.try_parse() != Some(ColorSpace::LinearSRGB) {
            return Err(Error::new("expected linear sRGB aperture filter"));
        }

        if header.dimensions[0] == 0 || header.dimensions[1] == 0 {
            return Err(Error::new("invalid aperture filter dimensions"));
        }

        if header.dimensions[0] != header.dimensions[1] {
            return Err(Error::new("invalid aperture filter dimensions"));
        }

        let mut tile_size = match aperture.tile_size {
            ConvolutionTileSize::Lowest => 128,
            ConvolutionTileSize::Low => 256,
            ConvolutionTileSize::Medium => 512,
            ConvolutionTileSize::High => 1024,
        };

        // no sense using a tile which is larger than the filter
        tile_size = tile_size.min(header.dimensions[0] as usize);

        self.fft_signal_tile_r.create(2 * tile_size, 2 * tile_size);
        self.fft_signal_tile_g.create(2 * tile_size, 2 * tile_size);
        self.fft_signal_tile_b.create(2 * tile_size, 2 * tile_size);
        self.fft_signal_fbo.rebuild(
            &[
                &self.fft_signal_tile_r,
                &self.fft_signal_tile_g,
                &self.fft_signal_tile_b,
            ],
            None,
        )?;

        self.fft_buffer_tile_r.create(2 * tile_size, 2 * tile_size);
        self.fft_buffer_tile_g.create(2 * tile_size, 2 * tile_size);
        self.fft_buffer_tile_b.create(2 * tile_size, 2 * tile_size);
        self.fft_buffer_fbo.rebuild(
            &[
                &self.fft_buffer_tile_r,
                &self.fft_buffer_tile_g,
                &self.fft_buffer_tile_b,
            ],
            None,
        )?;

        self.generate_fft_passes(2 * tile_size);

        let tile_generator = TileIterator::new(
            header.dimensions[0] as usize,
            header.dimensions[1] as usize,
            tile_size,
        );

        let mut filter: Texture<RGBA16F> = Texture::new(self.gl.clone());
        filter.upload(
            header.dimensions[0] as usize,
            header.dimensions[1] as usize,
            &LayoutVerified::new_slice(data).unwrap(),
        );

        for (index, tile) in tile_generator.enumerate() {
            let mut r_tex = Texture::new(self.gl.clone());
            let mut g_tex = Texture::new(self.gl.clone());
            let mut b_tex = Texture::new(self.gl.clone());
            r_tex.create(2 * tile_size, 2 * tile_size);
            g_tex.create(2 * tile_size, 2 * tile_size);
            b_tex.create(2 * tile_size, 2 * tile_size);

            let mut fbo = Framebuffer::new(self.gl.clone());
            fbo.rebuild(&[&r_tex, &g_tex, &b_tex], None)?;

            self.load_filter_tile(index, tile, &filter);
            self.precompute_filter_tile_fft(index);

            self.fft_filter_tile_r.push(r_tex);
            self.fft_filter_tile_g.push(g_tex);
            self.fft_filter_tile_b.push(b_tex);
            self.fft_filter_fbo.push(fbo);
        }

        Ok(())
    }

    pub(crate) fn reset_convolution_state(&mut self) {
        if !self.fft_filter_fbo.is_empty() {
            let tile_size = self.current_tile_size();

            let signal_iter = TileIterator::new(
                self.convolution_signal_fbo.cols(),
                self.convolution_signal_fbo.rows(),
                tile_size,
            );

            let filter_size = self.current_filter_size();

            let filter_iter = TileIterator::new(filter_size, filter_size, tile_size).enumerate();

            self.convolution_tiles =
                Box::new(iproduct!(signal_iter, filter_iter).with_position().cycle());
        }
    }

    /// Returns the currently initialized tile size.
    pub(crate) fn current_tile_size(&self) -> usize {
        self.fft_signal_fbo.cols() / 2
    }

    /// Returns the currently initialized filter size.
    pub(crate) fn current_filter_size(&self) -> usize {
        // This isn't the cleanest code but will work because the filters are always
        // square and the tiles themselves should always divide the filters exactly.

        (self.fft_filter_fbo.len() as f64).sqrt() as usize * self.current_tile_size()
    }

    /// Loads a tile of the filter into the filter tile.
    ///
    /// After this method returns, the filter tile buffer will contain the
    /// specified tile of the filter, zero-padded & ready for convolution.
    pub(crate) fn load_filter_tile(&mut self, index: usize, tile: Tile, filter: &Texture<RGBA16F>) {
        let command = self.load_filter_tile_shader.begin_draw();

        command.bind(filter, "filter_tex");

        command.set_framebuffer(&self.fft_filter_fbo[index]);

        command.set_uniform_ivec2("tile_offset", tile.x as i32, tile.y as i32);
        command.set_uniform_ivec2(
            "tile_size",
            self.fft_filter_fbo[index].cols() as i32,
            self.fft_filter_fbo[index].rows() as i32,
        );

        command.set_viewport(
            0,
            0,
            self.fft_filter_fbo[index].cols() as i32,
            self.fft_filter_fbo[index].rows() as i32,
        );

        command.unset_vertex_array();
        command.draw_triangles(0, 1);
    }

    /// Loads a tile of the signal into the signal tile.
    ///
    /// After this method returns, the signal tile buffer will contain the
    /// specified tile of the signal, zero-padded & ready for convolution.
    pub(crate) fn load_signal_tile(&self, tile: Tile) {
        self.fft_signal_fbo.clear(0, [0.0; 4]);
        self.fft_signal_fbo.clear(1, [0.0; 4]);
        self.fft_signal_fbo.clear(2, [0.0; 4]);

        let command = self.load_signal_tile_shader.begin_draw();

        command.bind(&self.convolution_signal, "signal");

        // we render into the central half of the buffer; the rest is just zero-padded
        let offset = self.fft_signal_fbo.cols() / 4;

        command.set_uniform_ivec2(
            "tile_offset",
            tile.x as i32 - offset as i32,
            tile.y as i32 - offset as i32,
        );

        command.set_viewport(
            offset as i32,
            offset as i32,
            2 * offset as i32,
            2 * offset as i32,
        );

        command.set_framebuffer(&self.fft_signal_fbo);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);
    }

    /// Copies the radiance estimate to the convolution signal buffer.
    ///
    /// This is intended to allow the integrator to keep updating the estimate
    /// while the convolution operates on a fixed (albeit out of date) signal.
    pub(crate) fn copy_radiance_estimate_to_convolution_signal(&self) {
        let command = self.decompose_signal_shader.begin_draw();

        command.set_viewport(
            0,
            0,
            self.convolution_signal_fbo.cols() as i32,
            self.convolution_signal_fbo.rows() as i32,
        );

        command.bind(&self.integrator_radiance_estimate, "radiance_estimate");

        command.set_framebuffer(&self.convolution_signal_fbo);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);
    }

    /// Performs a forward FFT on the provided filter tile.
    ///
    /// After this method returns, the filter tile buffer will contain the FFT
    /// of the filter tile, the contents of which must have been pregenerated.
    pub(crate) fn precompute_filter_tile_fft(&self, tile_index: usize) {
        let command = self.execute_fft_pass_shader.begin_draw();

        command.set_vertex_array(&self.filter_fft_passes);

        // Placeholder textures (we're not convolving in here)
        command.bind(&self.fft_signal_tile_r, "r_conv_filter");
        command.bind(&self.fft_signal_tile_b, "g_conv_filter");
        command.bind(&self.fft_signal_tile_g, "b_conv_filter");

        command.set_viewport(
            0,
            0,
            self.fft_filter_fbo[tile_index].cols() as i32,
            self.fft_filter_fbo[tile_index].rows() as i32,
        );

        for pass in 0..(self.filter_fft_passes.vertex_count() / 3) {
            if pass % 2 == 0 {
                command.bind(&self.fft_filter_tile_r[tile_index], "r_conv_buffer");
                command.bind(&self.fft_filter_tile_g[tile_index], "g_conv_buffer");
                command.bind(&self.fft_filter_tile_b[tile_index], "b_conv_buffer");
                command.set_framebuffer(&self.fft_buffer_fbo);
            } else {
                command.bind(&self.fft_buffer_tile_r, "r_conv_buffer");
                command.bind(&self.fft_buffer_tile_g, "g_conv_buffer");
                command.bind(&self.fft_buffer_tile_b, "b_conv_buffer");
                command.set_framebuffer(&self.fft_filter_fbo[tile_index]);
            }

            command.draw_triangles(pass, 1);
        }
    }

    /// Convolves the current signal tile with a filter tile.
    ///
    /// After this method returns, the signal tile buffers will contain the
    /// convolved signal, ready to be composited in the convolution buffer.
    pub(crate) fn convolve_tile(&self, tile_index: usize) {
        let command = self.execute_fft_pass_shader.begin_draw();

        command.set_vertex_array(&self.signal_fft_passes);

        command.bind(&self.fft_filter_tile_r[tile_index], "r_conv_filter");
        command.bind(&self.fft_filter_tile_g[tile_index], "g_conv_filter");
        command.bind(&self.fft_filter_tile_b[tile_index], "b_conv_filter");

        command.set_viewport(
            0,
            0,
            self.fft_signal_fbo.cols() as i32,
            self.fft_signal_fbo.rows() as i32,
        );

        for pass in 0..(self.signal_fft_passes.vertex_count() / 3) {
            if pass % 2 == 0 {
                command.bind(&self.fft_signal_tile_r, "r_conv_buffer");
                command.bind(&self.fft_signal_tile_g, "g_conv_buffer");
                command.bind(&self.fft_signal_tile_b, "b_conv_buffer");
                command.set_framebuffer(&self.fft_buffer_fbo);
            } else {
                command.bind(&self.fft_buffer_tile_r, "r_conv_buffer");
                command.bind(&self.fft_buffer_tile_g, "g_conv_buffer");
                command.bind(&self.fft_buffer_tile_b, "b_conv_buffer");
                command.set_framebuffer(&self.fft_signal_fbo);
            }

            command.draw_triangles(pass, 1);
        }
    }

    /// Composites the current signal tile into the convolution buffer.
    ///
    /// After this method returns, the contents of the signal tile will have
    /// been accumulated into the convolution buffer. Once the final tile is
    /// processed, the convolution buffer will contain the convolved signal.
    pub(crate) fn composite_tile(&self, x: i32, y: i32, w: i32, h: i32) {
        let command = self.read_signal_tile_shader.begin_draw();

        command.bind(&self.fft_signal_tile_r, "signal_tile_r");
        command.bind(&self.fft_signal_tile_g, "signal_tile_g");
        command.bind(&self.fft_signal_tile_b, "signal_tile_b");

        command.set_uniform_ivec2("tile_offset", x, y);

        command.set_viewport(x, y, w, h);

        command.set_framebuffer(&self.convolution_output_fbo);
        command.set_blend_mode(BlendMode::Add);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);
    }

    /// Generates the necessary FFT passes for convolution.
    ///
    /// This method must have been called prior to attempting any FFT or
    /// convolution operation, or the operation will not work correctly.
    pub(crate) fn generate_fft_passes(&mut self, resolution: usize) {
        self.generate_filter_fft_passes(resolution);
        self.generate_signal_fft_passes(resolution);
    }

    fn generate_filter_fft_passes(&mut self, resolution: usize) {
        let (depth, mut passes) = (resolution.trailing_zeros() as u16, vec![]);

        for m in (1..=depth).rev() {
            passes.push(FFTPassData {
                horizontal: 1,
                direction: 1,
                subtransform_size: 1 << m,
                convolve: 0,
            });
        }

        for m in (1..=depth).rev() {
            passes.push(FFTPassData {
                horizontal: 0,
                direction: 1,
                subtransform_size: 1 << m,
                convolve: 0,
            });
        }

        self.filter_fft_passes.upload(&Self::gen_pass_tris(passes));
    }

    fn generate_signal_fft_passes(&mut self, resolution: usize) {
        let (depth, mut passes) = (resolution.trailing_zeros() as u16, vec![]);

        for m in (1..=depth).rev() {
            passes.push(FFTPassData {
                horizontal: 1,
                direction: 1,
                subtransform_size: 1 << m,
                convolve: 0,
            });
        }

        for m in (1..=depth).rev() {
            passes.push(FFTPassData {
                horizontal: 0,
                direction: 1,
                subtransform_size: 1 << m,
                convolve: (m == 1) as u16,
            });
        }

        for m in 1..=depth {
            passes.push(FFTPassData {
                horizontal: 0,
                direction: 0,
                subtransform_size: 1 << m,
                convolve: 0,
            });
        }

        for m in 1..=depth {
            passes.push(FFTPassData {
                horizontal: 1,
                direction: 0,
                subtransform_size: 1 << m,
                convolve: 0,
            });
        }

        self.signal_fft_passes.upload(&Self::gen_pass_tris(passes));
    }

    fn gen_pass_tris(passes: Vec<FFTPassData>) -> Vec<FFTPassData> {
        let tris = passes.into_iter().map(|x| repeat(x).take(3));
        tris.flatten().collect() // create one triangle per pass
    }
}

/// An iterator over tiles of a 2D grid.
///
/// Given a 2D grid of N columns by M rows, this iterator will generate a list
/// of tiles (limited to a maximum square size) which entirely cover the grid.
#[derive(Clone, Copy, Debug)]
pub struct TileIterator {
    cols: usize,
    rows: usize,
    tile_cols: usize,
    tile_rows: usize,
    tile_size: usize,
    tile_counter: usize,
}

impl TileIterator {
    pub fn new(cols: usize, rows: usize, tile_size: usize) -> Self {
        Self {
            cols,
            rows,
            tile_cols: (cols + tile_size - 1) / tile_size,
            tile_rows: (rows + tile_size - 1) / tile_size,
            tile_size,
            tile_counter: 0,
        }
    }

    pub fn tile_count(&self) -> usize {
        self.tile_cols * self.tile_rows
    }
}

impl Iterator for TileIterator {
    type Item = Tile;

    fn next(&mut self) -> Option<Tile> {
        if self.tile_counter != self.tile_count() {
            let x = (self.tile_counter % self.tile_cols) * self.tile_size;
            let y = (self.tile_counter / self.tile_cols) * self.tile_size;

            let w = (x + self.tile_size).min(self.cols) - x;
            let h = (y + self.tile_size).min(self.rows) - y;
            self.tile_counter += 1; // advance to next tile

            Some(Tile { x, y, w, h })
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}
