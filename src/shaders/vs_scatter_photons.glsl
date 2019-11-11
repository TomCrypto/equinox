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

void record_photon(ray_t ray, vec3 throughput) {
    ivec2 coords = hash_entry_for_cell(cell_for_point(ray.org), uint(gl_InstanceID));

    photon_major_data = vec4(     fract(ray.org / integrator.cell_size), ray.dir.x);
    photon_minor_data = vec4(ray.dir.y < 0.0 ? -throughput : throughput, ray.dir.z);

    vec2 clip_space = 2.0 * (vec2(0.5) + vec2(coords)) / integrator.hash_dimensions - 1.0;
    gl_Position = vec4(clip_space, 0.0, 1.0); // put the photon into its hash table entry
}

bool scatter_photon(ray_t ray, inout random_t random, vec3 throughput) {
    uint traversal_start = 0U;

    for (uint bounce = 0U; bounce < integrator.max_scatter_bounces; ++bounce) {
        traversal_t traversal = traverse_scene(ray, traversal_start);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y;

            vec3 normal = geo_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint material = traversal.hit.y & 0xffffU;
            uint mat_inst = traversal.hit.y >> 16U;
            
            vec2 weights = rand_uniform_vec2(random);

            // TODO: add direct lighting flag to extend the first bounce check....
            bool is_receiver = MAT_IS_RECEIVER(material) && (bounce != 0U);

            bool inside = dot(ray.dir, normal) > 0.0;
            vec3 wi;

            #define MAT_SWITCH_LOGIC(absorption, eval, sample) {                                  \
                throughput *= absorption(mat_inst, inside, traversal.range.y);                    \
                                                                                                  \
                if (is_receiver && weights.x < integrator.photon_rate) {                          \
                    record_photon(ray, throughput / integrator.photon_rate);                      \
                    return true; /* rasterize photon into the hash table */                       \
                }                                                                                 \
                                                                                                  \
                throughput /= is_receiver ? 1.0 - integrator.photon_rate : 1.0;                   \
                                                                                                  \
                float material_pdf; /* we don't need the PDF of the sampling method here */       \
                vec3 f = sample(mat_inst, normal, ray.dir, -ray.dir, material_pdf, random);       \
                                                                                                  \
                float q = max(0.0, 1.0 - luminance(throughput * f) / luminance(throughput));      \
                                                                                                  \
                if (weights.y < q) {                                                              \
                    return false;                                                                 \
                }                                                                                 \
                                                                                                  \
                throughput *= f / (1.0 - q);                                                      \
                                                                                                  \
                ray = make_ray(ray.org, ray.dir, normal);                                         \
            }

            MAT_DO_SWITCH(material)
            #undef MAT_SWITCH_LOGIC

            traversal_start = (!inside && dot(ray.dir, normal) < 0.0) ? traversal.hit.z : 0U;
        } else {
            return false;
        }
    }

    return false;
}

void main() {
    random_t random = rand_initialize_from_seed(uvec2(gl_VertexID, gl_InstanceID) + integrator.rng);

    ray_t ray;

    // pick a random ray target in the scene's bounding box
    vec3 bbmin, bbmax;

    get_scene_bbox(bbmin, bbmax);

    // pick a random incident ray direction, importance-sampled
    float unused_pdf;
    vec3 throughput = env_sample_light(ray.dir, unused_pdf, random);
    ray.dir = -ray.dir;

    // TODO: better sampling for this, maybe using an ellipse or something to better fit the AABB

    // find the bounding sphere for the scene
    float radius = max(bbmax.x - bbmin.x, max(bbmax.y - bbmin.y, bbmax.z - bbmin.z)) / 2.0 * sqrt(3.0);

    // adjust PDF
    throughput *= M_PI * radius * radius;

    // generate a random "upwards" vector in the unit disk
    vec2 rng1 = rand_uniform_vec2(random);

    float r = sqrt(rng1.x) * radius;
    float a = rng1.y * M_2PI;

    float px = r * cos(a);
    float py = r * sin(a);

    vec3 base_pos = vec3(px, 0.0, py);
    
    // rotate it to be aligned with the ray direction
    vec3 real_pos = rotate(base_pos, ray.dir);

    // compute a good ray origin
    ray.org = real_pos - radius * ray.dir;

    if (!scatter_photon(ray, random, throughput)) {
        // prevent point from being rasterized
        gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
    }
}
