#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::{BlendMode, Device, Integrator, RasterFilter, Scene};
use js_sys::Error;
use quasi_rd::Sequence;
use rand::{rngs::StdRng, RngCore, SeedableRng};
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
    photon_count: f32,
    sppm_alpha: f32,
    padding: u32,

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
    pub(crate) rng: StdRng,
    pub(crate) filter_rng: Sequence,

    pub(crate) filter: RasterFilter,
    pub(crate) integrator: Integrator,

    pub(crate) current_pass: u32,
    pub(crate) photon_count: f32,
    pub(crate) kernel_radii: KernelRadiusSequence,

    pub(crate) receivers_present: bool,
}

impl Default for IntegratorState {
    fn default() -> Self {
        Self {
            rng: StdRng::seed_from_u64(0),
            filter_rng: Sequence::new(2),
            filter: RasterFilter::default(),
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
        self.integrator_gather_photons_shader
            .set_define("PREC", format!("{:.32}", integrator.geometry_precision));
        self.integrator_scatter_photons_shader
            .set_define("PREC", format!("{:.32}", integrator.geometry_precision));
        self.integrator_gather_photons_shader
            .set_define("PUSHBACK", format!("{:.32}", integrator.geometry_pushback));
        self.integrator_scatter_photons_shader
            .set_define("PUSHBACK", format!("{:.32}", integrator.geometry_pushback));

        Ok(())
    }

    pub(crate) fn reset_integrator_state(&mut self, scene: &mut Scene) {
        self.state.rng = StdRng::seed_from_u64(0);
        self.state.filter_rng.seek(0);
        self.state.photon_count = 0.0;
        self.state.current_pass = 0;

        self.state.filter = scene.raster.filter;
        self.state.integrator = *scene.integrator;

        Self::clamp_integrator_settings(&mut self.state.integrator);

        // The photon positions are limited by the geometry precision, clamp the minimum
        // search radius to approximately that precision to avoid bad render artifacts.

        self.state.kernel_radii = KernelRadiusSequence::new(
            self.state.integrator.max_search_radius,
            self.state
                .integrator
                .min_search_radius
                .max(10.0 * self.state.integrator.geometry_precision),
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

        let mut data = IntegratorData::default();
        let x = self.state.filter_rng.next_f32();
        let y = self.state.filter_rng.next_f32();

        data.filter_offset[0] = 4.0 * self.state.filter.importance_sample(x) - 2.0;
        data.filter_offset[1] = 4.0 * self.state.filter.importance_sample(y) - 2.0;
        data.hash_key[0] = self.state.rng.next_u32();
        data.hash_key[1] = self.state.rng.next_u32();
        data.hash_key[2] = self.state.rng.next_u32();
        data.hash_key[3] = 0;
        data.current_pass = self.state.current_pass;
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

        self.integrator_scatter_fbo.clear(0, [0.0; 4]);
        self.integrator_scatter_fbo.clear(1, [0.0; 4]);
        self.integrator_scatter_fbo.clear(2, [0.0; 4]);

        let command = self.integrator_scatter_photons_shader.begin_draw();

        command.bind(&self.geometry_buffer, "Geometry");
        command.bind(&self.material_buffer, "Material");
        command.bind(&self.instance_buffer, "Instance");
        command.bind(&self.integrator_buffer, "Integrator");
        command.bind(&self.raster_buffer, "Raster");
        command.bind(&self.environment_buffer, "Environment");
        command.bind(&self.scatter_quasi_buffer, "QuasiSampler");

        if self.envmap_color.is_invalid() {
            command.bind(&self.placeholder_texture, "envmap_color");
        } else {
            command.bind(&self.envmap_color, "envmap_color");
        }

        if self.envmap_marg_cdf.is_invalid() {
            command.bind(&self.placeholder_texture, "envmap_marg_cdf");
        } else {
            command.bind(&self.envmap_marg_cdf, "envmap_marg_cdf");
        }

        if self.envmap_cond_cdf.is_invalid() {
            command.bind(&self.placeholder_texture, "envmap_cond_cdf");
        } else {
            command.bind(&self.envmap_cond_cdf, "envmap_cond_cdf");
        }

        if self.material_textures.is_invalid() {
            command.bind(&self.placeholder_texture_array, "material_textures");
        } else {
            command.bind(&self.material_textures, "material_textures");
        }

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
        command.bind(&self.integrator_photon_table_pos, "photon_table_pos");
        command.bind(&self.integrator_photon_table_sum, "photon_table_sum");

        if self.envmap_color.is_invalid() {
            command.bind(&self.placeholder_texture, "envmap_color");
        } else {
            command.bind(&self.envmap_color, "envmap_color");
        }

        if self.envmap_marg_cdf.is_invalid() {
            command.bind(&self.placeholder_texture, "envmap_marg_cdf");
        } else {
            command.bind(&self.envmap_marg_cdf, "envmap_marg_cdf");
        }

        if self.envmap_cond_cdf.is_invalid() {
            command.bind(&self.placeholder_texture, "envmap_cond_cdf");
        } else {
            command.bind(&self.envmap_cond_cdf, "envmap_cond_cdf");
        }

        if self.material_textures.is_invalid() {
            command.bind(&self.placeholder_texture_array, "material_textures");
        } else {
            command.bind(&self.material_textures, "material_textures");
        }

        command.set_framebuffer(&self.integrator_gather_fbo);

        if let Some([x, y, w, h]) = self.render_region {
            command.set_viewport(x as i32, y as i32, w as i32, h as i32);
        } else {
            command.set_viewport(
                0,
                0,
                self.integrator_gather_fbo.cols() as i32,
                self.integrator_gather_fbo.rows() as i32,
            );
        }

        command.set_blend_mode(BlendMode::Add);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);
    }

    fn clamp_integrator_settings(integrator: &mut Integrator) {
        integrator.alpha = integrator.alpha.max(0.0).min(1.0);
        integrator.max_scatter_bounces = integrator.max_scatter_bounces.max(2);
        integrator.max_gather_bounces = integrator.max_gather_bounces.max(2);
    }

    fn populate_quasi_buffer(buffer: &mut [SamplerDimensionAlpha]) {
        let mut parameters = vec![0; buffer.len()];

        quasi_rd::generate_parameters(buffer.len(), &mut parameters);

        let mut k = buffer.len() / 2;

        for (index, output) in buffer.iter_mut().enumerate() {
            output.alpha = [
                (parameters[k] >> 96) as u32,
                (parameters[k] >> 64) as u32,
                (parameters[k] >> 32) as u32,
                0,
            ];

            if index % 2 == 0 {
                k -= index + 1;
            } else {
                k += index + 1;
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
