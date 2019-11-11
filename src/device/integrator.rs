#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::BlendMode;
use crate::{Aperture, Device, Integrator, RasterFilter, Scene};
use js_sys::Error;
use quasirandom::Qrng;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(AsBytes, FromBytes, Debug)]
pub struct IntegratorData {
    rng: [u32; 2],
    filter_offset: [f32; 2],

    current_pass: u32,
    photon_rate: f32,
    photon_count: f32,
    sppm_alpha: f32,

    cell_size: f32,
    hash_cell_cols: u32,
    hash_cell_rows: u32,
    hash_cell_col_bits: u32,

    hash_cols_mask: u32,
    hash_rows_mask: u32,

    hash_dimensions: [f32; 2],

    max_scatter_bounces: u32,
    max_gather_bounces: u32,

    padding: [u32; 2],
}

pub struct IntegratorPass {
    pub n: usize,
    pub m: usize,
    pub cell_size: f32,
}

#[derive(Debug)]
pub struct IntegratorState {
    pub(crate) rng: ChaCha20Rng,
    pub(crate) filter_rng: Qrng,

    pub(crate) filter: RasterFilter,
    pub(crate) integrator: Integrator,
    pub(crate) aperture: Option<Aperture>,

    pub(crate) current_pass: u32,
    pub(crate) photon_count: f32,

    pub(crate) receivers_present: bool,
}

impl Default for IntegratorState {
    fn default() -> Self {
        Self {
            rng: ChaCha20Rng::seed_from_u64(0),
            filter_rng: Qrng::new(0),
            filter: RasterFilter::default(),
            aperture: None,
            integrator: Integrator::default(),
            photon_count: 0.0,
            current_pass: 0,
            receivers_present: false,
        }
    }
}

impl Device {
    pub(crate) fn reset_integrator_state(&mut self, scene: &mut Scene) {
        self.state = IntegratorState::default();

        self.state.aperture = (*scene.aperture).clone();
        self.state.filter = scene.raster.filter;
        self.state.integrator = *scene.integrator;

        self.integrator_gather_fbo.clear(0, [0.0, 0.0, 0.0, 0.0]);
        self.integrator_update_fbo.clear(
            0,
            [
                0.0,
                0.0,
                0.0,
                self.state.integrator.initial_search_radius.powi(2),
            ],
        );

        self.state.receivers_present = false;

        for instance in scene.instance_list.iter() {
            if instance.visible && instance.photon_receiver {
                self.state.receivers_present = true;
                break;
            }
        }
    }

    pub(crate) fn prepare_integrator_pass(&self) -> IntegratorPass {
        let k = (1.0 - self.state.integrator.alpha) / 2.0;

        let search_radius = self.state.integrator.initial_search_radius
            * (1.0 + self.state.current_pass as f32).powf(-k);

        let cell_size = 2.0 * search_radius;

        let target = ((self.state.integrator.capacity_multiplier / cell_size.powi(2)).round()
            as usize)
            .min(self.state.integrator.photons_per_pass)
            .max(1);

        let (n, m) = self.calculate_photon_batch(self.state.integrator.photons_per_pass, target);

        IntegratorPass { n, m, cell_size }
    }

    pub(crate) fn update_integrator_state(&mut self, pass: &IntegratorPass) -> Result<(), Error> {
        self.state.current_pass += 1;
        self.state.photon_count += (pass.n * pass.m) as f32;

        let (hash_cell_cols, hash_cell_rows) = Self::get_hash_cell_dimensions(pass.m);

        // we need to ignore the first (0, 0) sample from the sequence
        let (mut x, mut y) = self.state.filter_rng.next::<(f32, f32)>();

        if x == 0.0 && y == 0.0 {
            x = 0.5;
            y = 0.5;
        }

        let data: &mut IntegratorData = self.allocator.allocate_one();

        data.filter_offset[0] = 4.0 * self.state.filter.importance_sample(x) - 2.0;
        data.filter_offset[1] = 4.0 * self.state.filter.importance_sample(y) - 2.0;
        data.rng[0] = self.state.rng.next_u32();
        data.rng[1] = self.state.rng.next_u32();
        data.current_pass = self.state.current_pass;
        data.photon_rate = self.state.integrator.photon_rate.max(0.05).min(0.95);
        data.photon_count = self.state.photon_count;
        data.sppm_alpha = self.state.integrator.alpha;
        data.cell_size = pass.cell_size;
        data.hash_cell_cols = hash_cell_cols as u32;
        data.hash_cell_rows = hash_cell_rows as u32;
        data.hash_cell_col_bits = (hash_cell_cols - 1).count_ones();
        data.hash_dimensions[0] = self.photon_hash_table_major.cols() as f32;
        data.hash_dimensions[1] = self.photon_hash_table_major.rows() as f32;
        data.max_scatter_bounces = self.state.integrator.max_scatter_bounces;
        data.max_gather_bounces = self.state.integrator.max_gather_bounces;

        data.hash_cols_mask =
            ((self.photon_hash_table_major.cols() - 1) & !(hash_cell_cols - 1)) as u32;
        data.hash_rows_mask =
            ((self.photon_hash_table_major.rows() - 1) & !(hash_cell_rows - 1)) as u32;

        self.integrator_buffer.write(&data)
    }

    pub(crate) fn scatter_photons(&mut self, pass: &IntegratorPass) {
        if !self.state.receivers_present {
            return;
        }

        let command = self.integrator_scatter_photons_shader.begin_draw();

        command.bind(&self.geometry_buffer, "Geometry");
        command.bind(&self.material_buffer, "Material");
        command.bind(&self.instance_buffer, "Instance");
        command.bind(&self.integrator_buffer, "Integrator");
        command.bind(&self.raster_buffer, "Raster");
        command.bind(&self.envmap_texture, "envmap_texture");
        command.bind(&self.envmap_marg_cdf, "envmap_marg_cdf");
        command.bind(&self.envmap_cond_cdf, "envmap_cond_cdf");

        command.set_viewport(
            0,
            0,
            self.photon_hash_table_major.cols() as i32,
            self.photon_hash_table_minor.rows() as i32,
        );

        command.set_framebuffer(&self.photon_fbo);
        self.photon_fbo.clear(0, [1e20; 4]);
        self.photon_fbo.clear(1, [1e20; 4]);

        command.unset_vertex_array();
        command.draw_points_instanced(pass.n, pass.m);
    }

    pub(crate) fn gather_photons(&mut self) {
        let command = self.integrator_gather_photons_shader.begin_draw();

        command.bind(&self.camera_buffer, "Camera");
        command.bind(&self.geometry_buffer, "Geometry");
        command.bind(&self.material_buffer, "Material");
        command.bind(&self.instance_buffer, "Instance");
        command.bind(&self.integrator_buffer, "Integrator");
        command.bind(&self.raster_buffer, "Raster");
        command.bind(&self.envmap_texture, "envmap_texture");
        command.bind(&self.envmap_marg_cdf, "envmap_marg_cdf");
        command.bind(&self.envmap_cond_cdf, "envmap_cond_cdf");
        command.bind(&self.integrator_li_range, "li_range_tex");
        command.bind(&self.photon_hash_table_major, "photon_table_major");
        command.bind(&self.photon_hash_table_minor, "photon_table_minor");

        command.set_framebuffer(&self.integrator_gather_fbo);

        self.integrator_gather_fbo.clear(1, [0.0; 4]);

        command.set_viewport(0, 0, self.samples.cols() as i32, self.samples.rows() as i32);

        command.set_blend_mode(BlendMode::Add);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);
    }

    pub(crate) fn update_estimates(&mut self) {
        if !self.state.receivers_present {
            return;
        }

        let command = self.integrator_update_estimates_shader.begin_draw();

        command.bind(&self.integrator_buffer, "Integrator");
        command.bind(&self.integrator_ld_count, "ld_count_tex");
        command.bind(&self.integrator_li_count, "li_count_tex");

        command.set_viewport(0, 0, self.samples.cols() as i32, self.samples.rows() as i32);

        command.set_framebuffer(&self.integrator_update_fbo);

        command.set_blend_mode(BlendMode::UpdateEstimate);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);
    }

    pub(crate) fn estimate_radiance(&mut self) {
        let command = self.integrator_estimate_radiance_shader.begin_draw();

        command.bind(&self.integrator_buffer, "Integrator");
        command.bind(&self.integrator_ld_count, "ld_count_tex");
        command.bind(&self.integrator_li_range, "li_range_tex");

        command.set_viewport(0, 0, self.samples.cols() as i32, self.samples.rows() as i32);

        command.set_framebuffer(&self.samples_fbo);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);
    }

    fn calculate_photon_batch(&self, max_load: usize, target: usize) -> (usize, usize) {
        let mut best_n = 0;
        let mut best_m = 0;

        for s in 0..=self.state.integrator.max_hash_cell_bits {
            let m = 1 << s;
            let n = (max_load / m).min(target);

            if n * m > best_n * best_m {
                best_n = n;
                best_m = m;
            }
        }

        (best_n, best_m)
    }

    fn get_hash_cell_dimensions(mut m: usize) -> (usize, usize) {
        let mut cols = 1;
        let mut rows = 1;

        while m != 1 {
            if m >= 4 {
                cols *= 2;
                rows *= 2;
                m /= 4;
            } else {
                cols *= 2;
                m /= 2;
            }
        }

        (cols, rows)
    }
}
