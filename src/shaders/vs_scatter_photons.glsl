out vec4 photon_major_data;
out vec4 photon_minor_data;

#include <common.glsl>
#include <random.glsl>

#include <geometry.glsl>
#include <instance.glsl>
#include <material.glsl>
#include <environment.glsl>
#include <integrator.glsl>

layout (std140) uniform Raster {
    vec4 dimensions;
} raster;

void deposit_photon(ray_t ray, vec3 throughput) {
    ivec2 coords = hash_entry_for_cell(cell_for_point(ray.org), uint(gl_InstanceID));

    photon_major_data = vec4(   fract(ray.org / integrator_cell_size()), -ray.dir.x);
    photon_minor_data = vec4(ray.dir.y > 0.0 ? -throughput : throughput, -ray.dir.z);

    vec2 clip_space = 2.0 * (vec2(0.5) + vec2(coords)) / integrator.hash_dimensions - 1.0;
    gl_Position = vec4(clip_space, 0.0, 1.0); // put the photon into its hash table entry
}

void scatter_photon(inout ray_t ray, inout vec3 throughput, random_t random) {
    for (uint bounce = 0U; bounce < integrator.max_scatter_bounces; ++bounce) {
        traversal_t traversal = traverse_scene(ray, 0U);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y;

            vec3 normal = geo_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint material = traversal.hit.y & 0xffffU;
            uint mat_inst = traversal.hit.y >> 16U;
            
            vec2 weights = rand_uniform_vec2(random);

            // Note surfaces will NEVER receive first bounce photons. The "sample explicit" flag
            // is purely an optimization meant for when a surface cannot directly see any light.

            bool is_receiver = MAT_IS_RECEIVER(material) && (bounce != 0U);

            bool inside = dot(ray.dir, normal) > 0.0;
            vec3 f;

            #define MAT_SWITCH_LOGIC(absorption, eval, sample) {                                  \
                throughput *= absorption(mat_inst, inside, traversal.range.y);                    \
                                                                                                  \
                if (is_receiver && weights.x < integrator.photon_rate) {                          \
                    deposit_photon(ray, throughput / integrator.photon_rate);                     \
                    return; /* rasterize this photon into the photon table */                     \
                }                                                                                 \
                                                                                                  \
                throughput /= is_receiver ? 1.0 - integrator.photon_rate : 1.0;                   \
                                                                                                  \
                float unused_pdf; /* we don't need the PDF of the sampling method */              \
                f = sample(mat_inst, normal, ray.dir, -ray.dir, unused_pdf, random);              \
            }

            MAT_DO_SWITCH(material)
            #undef MAT_SWITCH_LOGIC

            float q = max(0.0, 1.0 - luminance(throughput * f) / luminance(throughput));

            if (weights.y < q) {
                return;
            }

            throughput *= f / (1.0 - q);

            ray = make_ray(ray.org, ray.dir, normal);
        } else {
            return;
        }
    }
}

ray_t generate_photon_ray(out vec3 throughput, inout random_t random) {
    vec3 bbmin, bbmax, wi;

    get_scene_bbox(bbmin, bbmax);

    float unused_pdf;
    throughput = env_sample_light(wi, unused_pdf, random);
    wi = -wi;

    vec3 coords = ceil(-wi);

    float x_area = (bbmax.y - bbmin.y) * (bbmax.z - bbmin.z) * abs(wi.x);
    float y_area = (bbmax.x - bbmin.x) * (bbmax.z - bbmin.z) * abs(wi.y);
    float z_area = (bbmax.x - bbmin.x) * (bbmax.y - bbmin.y) * abs(wi.z);

    float area = x_area + y_area + z_area;
    throughput *= area; // division by PDF

    float w = rand_uniform_float(random) * area;
    vec2 surface_uv = rand_uniform_vec2(random);

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
    random_t random = rand_initialize_from_seed(uvec2(gl_VertexID, gl_InstanceID) + integrator.rng);

    vec3 throughput; // measure photon path contribution
    ray_t ray = generate_photon_ray(throughput, random);

    gl_PointSize = 1.0;
    gl_Position = vec4(-1.0, -1.0, -1.0, 1.0);

    scatter_photon(ray, throughput, random);
}
