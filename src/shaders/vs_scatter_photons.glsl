out vec4 table_major;
out vec4 table_minor;

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

    gl_Position = vec4(2.0 * (vec2(0.5) + vec2(coords)) / integrator.hash_dimensions - 1.0, 0.0, 1.0);

    table_major = vec4(fract(ray.org / integrator.cell_size), ray.dir.x);
    table_minor = vec4(ray.dir.z, throughput * ((ray.dir.y < 0.0) ? -1.0 : 1.0));
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

    // now fire the ray at the world, hoping for an intersection
    uint flags;

    for (uint bounce = 0U; bounce < 8U; ++bounce) {
        traversal_t traversal = traverse_scene(ray, 0U);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y;

            vec3 normal = geo_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint material = traversal.hit.y & 0xffffU;
            uint mat_inst = traversal.hit.y >> 16U;

            // TODO: don't hardcode this constant later
            bool is_receiver = (material & 0x8000U) != 0U;
            material &= ~0x8000U;

            is_receiver = is_receiver && (bounce != 0U);

            vec2 weights = rand_uniform_vec2(random);

            if (is_receiver && weights.x < integrator.photon_rate) {
                record_photon(ray, throughput / integrator.photon_rate);
                return; // rasterize this point into the hash table now
            }

            vec3 new_beta;

            ray = mat_interact(material, mat_inst, normal, -ray.dir, ray.org, traversal.range.y, new_beta, flags, random);

            if ((flags & RAY_FLAG_EXTINCT) != 0U) {
                break;
            }

            vec3 bnew = throughput * new_beta;

            float q = max(0.0, 1.0 - luminance(bnew) / luminance(throughput));

            if (weights.y < q) {
                break;
            }

            throughput = bnew / (1.0 - q);
        } else {
            break;
        }
    }

    // prevent point from being rasterized
    gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
}
