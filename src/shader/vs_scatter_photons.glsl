out vec3 photon_pos_data;
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

    photon_pos_data = ray.org;
    photon_sum_data = throughput / 65536.0;

    vec2 clip_space = 2.0 * (vec2(0.5) + vec2(coords)) / integrator.hash_dimensions - 1.0;
    gl_Position = vec4(clip_space, 0.0, 1.0); // put the photon into its hash table entry
}

void scatter_photon(ray_t ray, vec3 throughput, quasi_t quasi) {
    for (uint bounce = 0U; bounce < integrator.max_scatter_bounces; ++bounce) {
        traversal_t traversal = traverse_scene(ray, 0U);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y;

            vec3 normal = geo_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint mat_type = traversal.hit.y & 0xffffU;
            uint mat_inst = traversal.hit.y >> 16U;
            material_t material;

            float u1 = quasi_sample(quasi);
            float u2 = quasi_sample(quasi);
            float u3 = quasi_sample(quasi);
            float u4 = quasi_sample(quasi);

            // Note surfaces will NEVER receive first bounce photons. The "sample explicit" flag
            // is purely an optimization meant for when a surface cannot directly see any light.

            bool is_receiver = MAT_IS_RECEIVER(mat_type) && (bounce != 0U);

            float deposit_weight = is_receiver ? u1 : 0.0;

            bool inside = dot(ray.dir, normal) > 0.0;
            vec3 f;

            float n1, n2;

            throughput *= medium_absorption(traversal.hit.x >> 16U, inside,
                                            traversal.range.y, n1, n2);

            #define MAT_SWITCH_LOGIC(LOAD, EVAL, SAMPLE) {                                        \
                if (is_receiver) {                                                                \
                    deposit_photon(ray, throughput);                                              \
                    return; /* record this photon */                                              \
                }                                                                                 \
                                                                                                  \
                LOAD(mat_inst, normal, ray.org, material);                                        \
                                                                                                  \
                float unused_pdf;                                                                 \
                f = SAMPLE(material, normal, ray.dir, -ray.dir, n1, n2, unused_pdf, u2, u3);      \
            }

            MAT_DO_SWITCH(mat_type)
            #undef MAT_SWITCH_LOGIC

            float q = max(0.0, 1.0 - luminance(throughput * f) / luminance(throughput));

            if (u4 < q) {
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

    float u1 = quasi_sample(quasi);
    float u2 = quasi_sample(quasi);

    float unused_pdf;
    throughput = env_sample_light(wi, unused_pdf, u1, u2);
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

    return ray_t(mix(bbmin, bbmax, coords) - wi, wi);
}

void main() {
    quasi_t quasi = quasi_init(integrator.current_pass, decorrelate_sample(uint(gl_VertexID)));

    vec3 throughput; // measure photon path contribution
    ray_t ray = generate_photon_ray(throughput, quasi);

    gl_PointSize = 1.0;
    gl_Position = vec4(-1.0, -1.0, -1.0, 1.0);

    scatter_photon(ray, throughput, quasi);
}
