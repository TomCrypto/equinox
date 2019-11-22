out vec3 photon_pos_data;
out vec3 photon_dir_data;
out vec3 photon_sum_data;

#include <common.glsl>

#include <geometry.glsl>
#include <instance.glsl>
#include <material.glsl>
#include <environment.glsl>
#include <integrator.glsl>
#include <quasi.glsl>

layout (std140) uniform Raster {
    vec4 dimensions;
} raster;

void deposit_photon(ray_t ray, vec3 throughput) {
    ivec2 coords = hash_entry_for_cell(cell_for_point(ray.org));

    photon_pos_data = fract(ray.org / integrator.cell_size);
    photon_dir_data = 0.5 - 0.5 * ray.dir;
    photon_sum_data = throughput * 1e-5;

    vec2 clip_space = 2.0 * (vec2(0.5) + vec2(coords)) / integrator.hash_dimensions - 1.0;
    gl_Position = vec4(clip_space, 0.0, 1.0); // put the photon into its hash table entry
}

void scatter_photon(ray_t ray, vec3 throughput, quasi_t quasi) {
    for (uint bounce = 0U; bounce < integrator.max_scatter_bounces; ++bounce) {
        traversal_t traversal = traverse_scene(ray, 0U);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y;

            vec3 normal = geo_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint material = traversal.hit.y & 0xffffU;
            uint mat_inst = traversal.hit.y >> 16U;

            // Note surfaces will NEVER receive first bounce photons. The "sample explicit" flag
            // is purely an optimization meant for when a surface cannot directly see any light.

            bool is_receiver = MAT_IS_RECEIVER(material) && (bounce != 0U);

            float deposit_weight = is_receiver ? quasi_sample(quasi) : 0.0;

            bool inside = dot(ray.dir, normal) > 0.0;
            vec3 f;

            #define MAT_SWITCH_LOGIC(absorption, eval, sample) {                                  \
                throughput *= absorption(mat_inst, inside, traversal.range.y);                    \
                                                                                                  \
                if (is_receiver && deposit_weight < integrator.photon_rate) {                     \
                    deposit_photon(ray, throughput / integrator.photon_rate);                     \
                    return; /* rasterize this photon into the photon table */                     \
                }                                                                                 \
                                                                                                  \
                throughput /= is_receiver ? 1.0 - integrator.photon_rate : 1.0;                   \
                                                                                                  \
                float unused_pdf; /* we don't use the PDF of the sampling method */               \
                f = sample(mat_inst, normal, ray.dir, -ray.dir, unused_pdf, quasi);               \
            }

            MAT_DO_SWITCH(material)
            #undef MAT_SWITCH_LOGIC

            float q = max(0.0, 1.0 - luminance(throughput * f) / luminance(throughput));

            if (quasi_sample(quasi) < q) {
                return;
            }

            throughput *= f / (1.0 - q);

            ray = make_ray(ray.org, ray.dir, normal);
        } else {
            return;
        }
    }
}

ray_t generate_photon_ray(out vec3 throughput, inout quasi_t quasi) {
    vec3 bbmin, bbmax, wi;

    get_scene_bbox(bbmin, bbmax);

    float unused_pdf;
    throughput = env_sample_light(wi, unused_pdf, quasi);
    wi = -wi;

    vec3 coords = ceil(-wi);

    float x_area = (bbmax.y - bbmin.y) * (bbmax.z - bbmin.z) * abs(wi.x);
    float y_area = (bbmax.x - bbmin.x) * (bbmax.z - bbmin.z) * abs(wi.y);
    float z_area = (bbmax.x - bbmin.x) * (bbmax.y - bbmin.y) * abs(wi.z);

    float area = x_area + y_area + z_area;
    throughput *= area; // division by PDF

    float w = quasi_sample(quasi) * area;
    vec2 surface_uv; // get surface point
    
    surface_uv.s = quasi_sample(quasi);
    surface_uv.t = quasi_sample(quasi);

    if (w < x_area) {
        coords.yz = surface_uv;
    } else if (w < x_area + y_area) {
        coords.xz = surface_uv;
    } else {
        coords.xy = surface_uv;
    }

    return ray_t(mix(bbmin, bbmax, coords), wi);
}

void main() {
    quasi_t quasi = quasi_init(decorrelate_sample(uint(gl_VertexID)), integrator.current_pass);

    vec3 throughput; // measure photon path contribution
    ray_t ray = generate_photon_ray(throughput, quasi);

    gl_PointSize = 1.0;
    gl_Position = vec4(-1.0, -1.0, -1.0, 1.0);

    scatter_photon(ray, throughput, quasi);
}
