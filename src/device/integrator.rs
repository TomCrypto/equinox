#[allow(unused_imports)]
use log::{debug, info, warn};

use crate::BlendMode;
use crate::Device;

/*

Textures:

RGBA32F: pixel_state_li_radius_main   - stores total Li and radius^2 for the pixel
RGBA16F: pixel_state_li_count_temp   - stores new Li and the photon count for that pixel for this pass, intermediate data
RGBA16F: pixel_state_ld_count         - stores total Ld and photon count for the pixel, both divided by the pass count
                                        (this means Ld is usable as a radiance estimate; the photon count must be multiplied before use)

make the first one RGBA16F eventually when we figure out how to do it; it needs to be normalized against something

Framebuffers:

SHADER 1: for accumulating ld + count:

    location 0 => pixel_state_ld_count
    location 1 => pixel_state_li_count_temp

BINDS: pixel_state_li_radius_main (to get the pixel's search radius)

SHADER 2: for updating Li + radius:

    location 0 => pixel_state_li_radius_main

BINDS: pixel_state_ld_count (to get the previous photon count), pixel_state_li_count_temp (to get the pass photon count, and the pass Li)
        -> this is used to calculate the ratio which is used to update both Li and radius^2 for the current pixel

SHADER 3: for combining the radiance estimates:

    location 0 => "samples" (we'll fix this up later)

BINDS: pixel_state_li_radius_main, pixel_state_ld_count


*/

impl Device {
    pub(crate) fn scatter_photons(&mut self, n: usize, m: usize) {
        let command = self.test_shader.begin_draw();

        command.bind(&self.geometry_buffer, "Geometry");
        command.bind(&self.material_buffer, "Material");
        command.bind(&self.instance_buffer, "Instance");
        command.bind(&self.globals_buffer, "Globals");
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
        self.photon_fbo.clear(0, [-1.0; 4]);
        self.photon_fbo.clear(1, [-1.0; 4]);

        command.unset_vertex_array();
        command.draw_points_instanced(n, m);
    }

    pub(crate) fn gather_photons(&mut self) {
        // TODO: rename to "integrator_gather_photons_shader" when it works
        let command = self.visible_point_gen_shader.begin_draw();

        command.bind(&self.camera_buffer, "Camera");
        command.bind(&self.geometry_buffer, "Geometry");
        command.bind(&self.material_buffer, "Material");
        command.bind(&self.instance_buffer, "Instance");
        command.bind(&self.globals_buffer, "Globals");
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
        let command = self.integrator_update_estimates_shader.begin_draw();

        command.bind(&self.globals_buffer, "Globals");
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

        command.bind(&self.globals_buffer, "Globals");
        command.bind(&self.integrator_ld_count, "ld_count_tex");
        command.bind(&self.integrator_li_range, "li_range_tex");

        command.set_viewport(0, 0, self.samples.cols() as i32, self.samples.rows() as i32);

        command.set_framebuffer(&self.samples_fbo);

        command.unset_vertex_array();
        command.draw_triangles(0, 1);
    }
}
