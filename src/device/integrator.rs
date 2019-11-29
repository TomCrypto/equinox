#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{Aperture, BlendMode, Device, Integrator, RasterFilter, Scene};
use js_sys::Error;
use quasirandom::Qrng;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use zerocopy::{AsBytes, FromBytes};

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Clone, Copy, Debug, Default)]
pub struct SamplerDimensionAlpha {
    alpha: [u32; 4],
}

#[repr(align(16), C)]
#[derive(AsBytes, FromBytes, Debug, Default)]
pub struct IntegratorData {
    hash_key: [u32; 4],
    filter_offset: [f32; 2],

    current_pass: u32,
    photon_rate: f32,
    photon_count: f32,
    sppm_alpha: f32,

    search_radius: f32,
    search_radius_squared: f32,
    photons_for_pass: f32,
    cell_size: f32,

    hash_cols_mask: u32,
    hash_rows_mask: u32,

    hash_dimensions: [f32; 2],

    max_scatter_bounces: u32,
    max_gather_bounces: u32,
}

pub struct IntegratorPass {
    pub n: usize,
    pub search_radius: f32,
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
    pub(crate) kernel_radii: KernelRadiusSequence,

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
            kernel_radii: KernelRadiusSequence::default(),
            photon_count: 0.0,
            current_pass: 0,
            receivers_present: true,
        }
    }
}

impl Device {
    pub(crate) fn update_integrator(&mut self, integrator: &Integrator) -> Result<(), Error> {
        if integrator.hash_table_bits < 16 {
            return Err(Error::new("hash_table_bits must be 16 or more"));
        }

        if integrator.max_search_radius <= 0.0 {
            return Err(Error::new("max_search_radius must be positive"));
        }

        if integrator.min_search_radius <= 0.0 {
            return Err(Error::new("min_search_radius must be positive"));
        }

        if integrator.max_gather_bounces > 100 {
            return Err(Error::new("max_gather_bounces must be 100 or less"));
        }

        if integrator.max_scatter_bounces > 100 {
            return Err(Error::new("max_scatter_bounces must be 100 or less"));
        }

        let gather_dimensions = 2 + 5 * integrator.max_gather_bounces as usize;
        let scatter_dimensions = 5 + 4 * integrator.max_scatter_bounces as usize;

        let mut quasi_buffer =
            vec![SamplerDimensionAlpha::default(); gather_dimensions.max(scatter_dimensions)];

        Self::populate_quasi_buffer(&mut quasi_buffer[..gather_dimensions]);
        Self::populate_quasi_buffer(&mut quasi_buffer[..scatter_dimensions]);

        self.gather_quasi_buffer.write_array(
            self.gather_quasi_buffer.max_len(),
            &quasi_buffer[..gather_dimensions],
        )?;

        self.scatter_quasi_buffer.write_array(
            self.scatter_quasi_buffer.max_len(),
            &quasi_buffer[..scatter_dimensions],
        )?;

        self.integrator_gather_photons_shader
            .set_define("SAMPLER_MAX_DIMENSIONS", self.gather_quasi_buffer.len());
        self.integrator_scatter_photons_shader
            .set_define("SAMPLER_MAX_DIMENSIONS", self.scatter_quasi_buffer.len());

        Ok(())
    }

    pub(crate) fn reset_integrator_state(&mut self, scene: &mut Scene) {
        self.state.rng = ChaCha20Rng::seed_from_u64(0);
        self.state.filter_rng = Qrng::new(0);
        self.state.photon_count = 0.0;
        self.state.current_pass = 0;

        // ignore the first (0, 0) sample from the sequence
        let _ = self.state.filter_rng.next::<(f32, f32)>();

        self.state.aperture = (*scene.aperture).clone();
        self.state.filter = scene.raster.filter;
        self.state.integrator = *scene.integrator;

        Self::clamp_integrator_settings(&mut self.state.integrator);

        self.state.kernel_radii = KernelRadiusSequence::new(
            self.state.integrator.max_search_radius,
            self.state.integrator.min_search_radius,
            self.state.integrator.alpha,
        );

        self.integrator_gather_fbo.clear(0, [0.0, 0.0, 0.0, 0.0]);

        let receivers_present = scene.has_photon_receivers();

        if !self.state.receivers_present && receivers_present {
            info!("photon receivers present, enabling photon scatter pass");
        } else if self.state.receivers_present && !receivers_present {
            info!("no photons receivers present, disabling photon scatter pass");
        }

        self.state.receivers_present = receivers_present;
    }

    pub(crate) fn prepare_integrator_pass(&mut self) -> IntegratorPass {
        IntegratorPass {
            n: self.state.integrator.photons_per_pass,
            search_radius: self.state.kernel_radii.next_radius(),
        }
    }

    pub(crate) fn update_integrator_state(&mut self, pass: &IntegratorPass) -> Result<(), Error> {
        self.state.current_pass += 1;

        if self.state.receivers_present {
            self.state.photon_count += (pass.n) as f32;
        }

        let (x, y) = self.state.filter_rng.next::<(f32, f32)>();

        let mut data = IntegratorData::default();

        data.filter_offset[0] = 4.0 * self.state.filter.importance_sample(x) - 2.0;
        data.filter_offset[1] = 4.0 * self.state.filter.importance_sample(y) - 2.0;
        data.hash_key[0] = self.state.rng.next_u32();
        data.hash_key[1] = self.state.rng.next_u32();
        data.hash_key[2] = self.state.rng.next_u32();
        data.hash_key[3] = 0;
        data.current_pass = self.state.current_pass;
        data.photon_rate = self.state.integrator.photon_rate;
        data.photon_count = self.state.photon_count.max(1.0);
        data.sppm_alpha = self.state.integrator.alpha;
        data.search_radius = pass.search_radius;
        data.search_radius_squared = pass.search_radius * pass.search_radius;
        data.photons_for_pass = (pass.n) as f32;
        data.cell_size = 2.0 * pass.search_radius;
        data.hash_dimensions[0] = self.integrator_scatter_fbo.cols() as f32;
        data.hash_dimensions[1] = self.integrator_scatter_fbo.rows() as f32;
        data.max_scatter_bounces = self.state.integrator.max_scatter_bounces;
        data.max_gather_bounces = self.state.integrator.max_gather_bounces;
        data.hash_cols_mask = (self.integrator_scatter_fbo.cols() - 1) as u32;
        data.hash_rows_mask = (self.integrator_scatter_fbo.rows() - 1) as u32;

        self.integrator_buffer.write(&data)
    }

    pub(crate) fn scatter_photons(&mut self, pass: &IntegratorPass) {
        if !self.state.receivers_present {
            return;
        }

        self.integrator_scatter_fbo.clear(0, [std::f32::NAN; 4]);
        self.integrator_scatter_fbo.clear(2, [0.0; 4]);

        let command = self.integrator_scatter_photons_shader.begin_draw();

        command.bind(&self.geometry_buffer, "Geometry");
        command.bind(&self.material_buffer, "Material");
        command.bind(&self.instance_buffer, "Instance");
        command.bind(&self.integrator_buffer, "Integrator");
        command.bind(&self.raster_buffer, "Raster");
        command.bind(&self.environment_buffer, "Environment");
        command.bind(&self.scatter_quasi_buffer, "QuasiSampler");
        command.bind(&self.envmap_color, "envmap_color");
        command.bind(&self.envmap_marg_cdf, "envmap_marg_cdf");
        command.bind(&self.envmap_cond_cdf, "envmap_cond_cdf");

        command.set_viewport(
            0,
            0,
            self.integrator_scatter_fbo.cols() as i32,
            self.integrator_scatter_fbo.rows() as i32,
        );

        command.set_framebuffer(&self.integrator_scatter_fbo);
        command.set_blend_mode(BlendMode::AlphaPredicatedAdd);

        command.unset_vertex_array();
        command.draw_points(0, pass.n);
    }

    pub(crate) fn gather_photons(&mut self) {
        let command = self.integrator_gather_photons_shader.begin_draw();

        command.bind(&self.camera_buffer, "Camera");
        command.bind(&self.geometry_buffer, "Geometry");
        command.bind(&self.material_buffer, "Material");
        command.bind(&self.instance_buffer, "Instance");
        command.bind(&self.integrator_buffer, "Integrator");
        command.bind(&self.raster_buffer, "Raster");
        command.bind(&self.environment_buffer, "Environment");
        command.bind(&self.gather_quasi_buffer, "QuasiSampler");
        command.bind(&self.envmap_color, "envmap_color");
        command.bind(&self.envmap_marg_cdf, "envmap_marg_cdf");
        command.bind(&self.envmap_cond_cdf, "envmap_cond_cdf");
        command.bind(&self.integrator_photon_table_pos, "photon_table_pos");
        command.bind(&self.integrator_photon_table_dir, "photon_table_dir");
        command.bind(&self.integrator_photon_table_sum, "photon_table_sum");

        command.set_framebuffer(&self.integrator_gather_fbo);

        command.set_viewport(
            0,
            0,
            self.integrator_gather_fbo.cols() as i32,
            self.integrator_gather_fbo.rows() as i32,
        );

        command.set_blend_mode(BlendMode::Add);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);
    }

    fn clamp_integrator_settings(integrator: &mut Integrator) {
        integrator.alpha = integrator.alpha.max(0.0).min(1.0);
        integrator.photon_rate = integrator.photon_rate.max(0.05).min(0.95);
        integrator.max_scatter_bounces = integrator.max_scatter_bounces.max(2);
        integrator.max_gather_bounces = integrator.max_gather_bounces.max(2);
    }

    fn populate_quasi_buffer(buffer: &mut [SamplerDimensionAlpha]) {
        let phi = Self::weyl_sequence_phi(buffer.len() as f64);

        for (i, value) in buffer.iter_mut().enumerate() {
            let alpha = (1.0 / phi).powi((1 + i) as i32);

            // scale the alpha value by 2^64 to go into the fixed-point domain
            let alpha_u64: u64 = (alpha * 18_446_744_073_709_551_616.0) as u64;

            let lo = alpha_u64 as u32;
            let hi = (alpha_u64 >> 32) as u32;

            value.alpha = [lo, hi, lo & 0xffff, lo >> 16];
        }
    }

    #[allow(clippy::float_cmp)]
    fn weyl_sequence_phi(d: f64) -> f64 {
        let q = 1.0 / (d + 1.0);
        let mut phi: f64 = 2.0;

        loop {
            let succ = (1.0 + phi).powf(q);

            if succ != phi {
                phi = succ;
            } else {
                return phi;
            }
        }
    }
}

/// Sequence of radii for the photon gather kernel.
///
/// This struct returns an appropriate search radius to use during the radiance
/// estimation pass of the photon mapping algorithm. It will decrease over time
/// at the correct rate to ensure the estimator's variance converges to zero.
#[derive(Debug, Default)]
pub struct KernelRadiusSequence {
    max: f32,
    min: f32,
    alpha: f32,
    product: f32,
    iters: f32,
}

impl KernelRadiusSequence {
    pub fn new(max: f32, min: f32, alpha: f32) -> Self {
        Self {
            max,
            min,
            alpha,
            product: 1.0,
            iters: 1.0,
        }
    }

    pub fn next_radius(&mut self) -> f32 {
        let radius = (self.max * (self.product / self.iters).sqrt()).max(self.min);

        self.product *= (self.iters + self.alpha) / self.iters;
        self.iters += 1.0;

        radius
    }
}
