#include <common.glsl>
#include <random.glsl>
// #include <object.glsl>

out vec4 color;

layout (std140) uniform Camera {
    vec4 origin_plane[4];
    vec4 target_plane[4];
    vec4 aperture_settings;
} camera;

layout (std140) uniform Globals {
    vec2 filter_delta;
    uvec4 frame_state;
} globals;

#define FILTER_DELTA (globals.filter_delta)
#define FRAME_RANDOM (globals.frame_state.xy)
#define FRAME_NUMBER (globals.frame_state.z)

layout (std140) uniform Raster {
    vec4 dimensions;
} raster;

layout (std140) uniform Geometry {
    vec4 data[64];
} geometry_buffer;

layout (std140) uniform Material {
    vec4 data[64];
} material_buffer;

struct BvhNode {
    vec4 data1;
    vec4 data2;
};

layout (std140) uniform Instance {
    BvhNode data[256];
} instance_buffer;

#define PREC 1e-5

#include <geometry.glsl>

bool eval_sdf(ray_t ray, uint geometry, uint instance, inout vec2 range) {
    // TODO: possibly dynamically adjust precision based on initial distance?

    while (range.x < range.y) {
        float dist = sdf(geometry, instance, ray.org + range.x * ray.dir);

        if (dist < PREC) {
            return true;
        }

        range.x += dist;
    }

    return false;
}

// NOTE: this algorithm now actually works whichever starting offset you use, as long as the
// termination condition is adjusted to stop as soon as you encounter the starting offset
// again.
// this only holds true if the ray origin is actually inside the starting offset's AABB...
// (to within the specified precision, i.e. it must be othervise visited during traversal)
// (if not this MAY LOOP FOREVER! must be absolutely surely, provably inside the AABB)
// if this property is not needed we can do some more micro-optimizations

traversal_t traverse_scene(ray_t ray) {
    traversal_t traversal = traversal_prepare(PREC * 100.0);
    vec3 idir = vec3(1.0) / ray.dir; // precomputed inverse

    uint index = 0U;

    do {
        BvhNode node = instance_buffer.data[index++];

        uint word1 = floatBitsToUint(node.data1.w);
        uint word2 = floatBitsToUint(node.data2.w);

        index *= uint((word1 & 0x00008000U) == 0U);
        word1 &= 0xffff7fffU; // remove cyclic bit

        vec2 range = traversal.range;

        if (ray_bbox(ray.org, idir, range, node.data1.xyz, node.data2.xyz)) {
            if (word2 != 0xffffffffU && eval_sdf(ray, word1 & 0xffffU, word1 >> 16U, range)) {
                traversal_record_hit(traversal, range.x, uvec2(word1, word2));
            }
        } else if (word2 == 0xffffffffU) {
            index = word1; // skip branch
        }
    } while (index != 0U);

    return traversal;
}

// Low-discrepancy sequence generator.
//
// Given a fixed, unchanging key, this will produce a low-discrepancy sequence of 2D points
// as a function of frame number, e.g. on the next frame for the same key the next point in
// the sequence will be produced.

vec2 low_discrepancy_2d(uvec2 key) {
    return fract(vec2((key + FRAME_NUMBER) % 8192U) * vec2(0.7548776662, 0.5698402909));
}

// Begin envmap stuff

uniform sampler2D envmap_cdf_tex;
uniform sampler2D envmap_pix_tex;

uniform sampler2D envmap_marginal_cdf;
uniform sampler2D envmap_conditional_cdfs;

#define ENVMAP_W 4096
#define ENVMAP_H 2048

// TODO: add PDFs later on (see the PBR book for correct values...)

vec3 sample_envmap(vec3 direction) {
    vec2 uv = direction_to_equirectangular(direction, 0.0);

    return texture(envmap_pix_tex, uv).xyz;
}

uint find_interval(sampler2D texture, int y, float u) {
    uint first = 0U;
    uint size = uint(textureSize(texture, 0).x);
    uint len = size;

    int DEBUG = 0;

    while (len > 0U) {
        DEBUG += 1;

        if (DEBUG > 100) {
            discard;
        }

        uint _half = len >> 1U;
        uint middle = first + _half;

        float value = texelFetch(texture, ivec2(int(middle), y), 0).x;

        if (value <= u) {
            first = middle + 1U;
            len -= _half + 1U;
        } else {
            len = _half;
        }
    }

    return clamp(first - 1U, 0U, size - 2U);
}

/*

marginal CDF = [
    0.0,
    0.028975997,
    0.63967484,
    0.74944526,
    0.83684194,
    0.906162,
    0.95527494,
    0.9882174,
    1.0,
]

*/

// returns (U, V) of the sampled environment map
vec3 importance_sample_envmap(float u, float v, out float pdf) {
    // V DIRECTION (marginal CDF)

    uint v_offset = find_interval(envmap_marginal_cdf, 0, u);

    float v_cdf_at_offset = texelFetch(envmap_marginal_cdf, ivec2(int(v_offset), 0), 0).x;
    float v_cdf_at_offset_next = texelFetch(envmap_marginal_cdf, ivec2(int(v_offset) + 1, 0), 0).x;

    // linearly interpolate between u_offset and u_offset + 1 based on position of u between cdf_at_offset and u_cdf_at_offset_next
    float dv = (u - v_cdf_at_offset) / (v_cdf_at_offset_next - v_cdf_at_offset);

    pdf = (v_cdf_at_offset_next - v_cdf_at_offset);

    /*float dv = v - v_cdf_at_offset;

    if (v_cdf_at_offset_next != v_cdf_at_offset) {
        dv /= (v_cdf_at_offset_next - v_cdf_at_offset);
    }*/

    // PDF is func[offset] / funcInt which (IIUC) is just (cdf_at_offset_next - cdf_at_offset)

    float sampled_v = (float(v_offset) + dv) / float(textureSize(envmap_marginal_cdf, 0).x - 1);

    // U DIRECTION (conditional CDF)

    uint u_offset = find_interval(envmap_conditional_cdfs, int(v_offset), v);

    float u_cdf_at_offset = texelFetch(envmap_conditional_cdfs, ivec2(int(u_offset), v_offset), 0).x;
    float u_cdf_at_offset_next = texelFetch(envmap_conditional_cdfs, ivec2(int(u_offset) + 1, v_offset), 0).x;

    /*float du = v - u_cdf_at_offset;

    if (u_cdf_at_offset_next != u_cdf_at_offset) {
        du /= (u_cdf_at_offset_next - u_cdf_at_offset);
    }*/

    float du = (v - u_cdf_at_offset) / (u_cdf_at_offset_next - u_cdf_at_offset);

    pdf *= (u_cdf_at_offset_next - u_cdf_at_offset);

    // See V direction for PDF

    float sampled_u = (float(u_offset) + du) / float(textureSize(envmap_conditional_cdfs, 0).x - 1);

    // float sampled_u = v;

    return equirectangular_to_direction(vec2(fract(sampled_u + 0.5), sampled_v), 0.0);
}


// End envmap stuff

// Begin camera stuff

vec2 evaluate_circular_aperture_uv(uvec2 pixel_state) {
    vec2 uv = low_discrepancy_2d(pixel_state);

    float a = uv.s * M_2PI;

    return sqrt(uv.t) * vec2(cos(a), sin(a));
}

vec2 evaluate_polygon_aperture_uv(uvec2 pixel_state) {
    pixel_state += FRAME_RANDOM;
    bitshuffle_mini(pixel_state);

    vec2 uv = gen_vec2_uniform(pixel_state); // low_discrepancy_2d(pixel_state);

    float corner = floor(uv.s * camera.aperture_settings.y);

    float u = 1.0 - sqrt(uv.s * camera.aperture_settings.y - corner);
    float v = uv.t * (1.0 - u);

    float a = M_PI * camera.aperture_settings.w;

    float rotation = camera.aperture_settings.z + corner * 2.0 * a;

    float c = cos(rotation);
    float s = sin(rotation);

    vec2 p = vec2((u + v) * cos(a), (u - v) * sin(a));
    return vec2(c * p.x - s * p.y, s * p.x + c * p.y);
}

vec2 evaluate_aperture_uv(uvec2 pixel_state) {
    switch (int(camera.aperture_settings.x)) {
        case 0: return evaluate_circular_aperture_uv(pixel_state);
        case 1: return evaluate_polygon_aperture_uv(pixel_state);       
    }

    return vec2(0.0);
}

vec3 bilinear(vec4 p[4], vec2 uv) {
    return mix(mix(p[0].xyz, p[1].xyz, uv.x), mix(p[2].xyz, p[3].xyz, uv.x), uv.y);
}

void evaluate_primary_ray(uvec2 pixel_state, out vec3 pos, out vec3 dir) {
    vec2 raster_uv = (gl_FragCoord.xy + FILTER_DELTA) * raster.dimensions.w;
    raster_uv.x -= (raster.dimensions.x * raster.dimensions.w - 1.0) * 0.5;

    vec3 origin = bilinear(camera.origin_plane, evaluate_aperture_uv(pixel_state) * 0.5 + 0.5);

    // TODO: this isn't quite right; this generates a flat focal plane but it should be curved
    // (to be equidistant to the lens)
    // maybe just generate this directly in the shader, pass in the camera kind/parameters
    // but it will do for now, we can extend it later when it's needed

    vec3 target = bilinear(camera.target_plane, raster_uv);

    pos = origin;
    dir = normalize(target - origin);
}

// End camera stuff

void main() {
    uvec2 pixel_state = uvec2(gl_FragCoord.xy);
    bitshuffle_full(pixel_state); // randomized

    uvec2 frame_state = pixel_state + FRAME_RANDOM;

    ray_t ray;
    evaluate_primary_ray(pixel_state, ray.org, ray.dir);

    vec3 accumulated = vec3(0.0);
    vec3 factor = vec3(1.0);

    // many bounces (with russian roulette)
    for (int i = 0; i < 10; ++i) {
        traversal_t traversal = traverse_scene(ray);

        if (traversal_has_hit(traversal)) {
            ray.org += ray.dir * traversal.range.y; // closest distance to hit

            vec3 normal = sdf_normal(traversal.hit.x & 0xffffU, traversal.hit.x >> 16U, ray.org);

            uint material = traversal.hit.y & 0xffffU;
            uint offset = traversal.hit.y >> 16U;

            switch (material) {
                case 0U: {
                    // diffuse
                    // pick a random direction in the hemisphere and adjust factor

                    vec2 rng = gen_vec2_uniform(frame_state);
                    bitshuffle_mini(frame_state);

                    // importance sampling through cosine weighting

                    float r = sqrt(rng.x);
                    float phi = M_2PI * rng.y;

                    vec3 a = vec3(r * cos(phi), sqrt(1.0 - rng.x), r * sin(phi));

                    // basis transform

                    vec3 v = normal - vec3(0.0, 1.0, 0.0);
                    ray.dir = a - 2.0 * v * (dot(a, v) / max(1e-5, dot(v, v)));


                    factor *= material_buffer.data[offset + 0U].xyz;

                    vec2 rng2 = gen_vec2_uniform(frame_state);
                    bitshuffle_mini(frame_state);
                    float pdf;
                    vec3 dir_to_envmap = importance_sample_envmap(rng2.x, rng2.y, pdf);

                    traversal_t traversal2 = traverse_scene(ray_t(ray.org, dir_to_envmap));

                    if (!traversal_has_hit(traversal2)) {
                        pdf /= (M_2PI * M_PI);

                        accumulated += factor * sample_envmap(dir_to_envmap) * max(0.0, dot(dir_to_envmap, normal)) / pdf;
                    }

                    break;
                }
                case 1U: {
                    // specular
                    // reflect the ray off the normal and continue; assume perfect reflection so no change
                    // in factor

                    ray.dir = reflect(ray.dir, normal);
                    factor *= 0.5;
                    break;
                }
                case 2U: {
                    // emissive
                    // terminate the ray

                    accumulated += factor * material_buffer.data[offset + 0U].xyz;
                    factor = vec3(0.0);
                    break;
                }
                default:
                    return; // bug (TODO: do something coherent on these kinds of bugs)
            }

            // color = vec4(normal * 0.5 + 0.5, 1.0);
            // return;
        } else {
            // we've escaped; accumulate environment map and break out

            // accumulated += factor * sample_envmap(ray.dir) / (M_2PI * M_PI);

            // we've escaped, break out
            break;
        }

        // russian roulette

        vec2 rng = gen_vec2_uniform(frame_state);
        bitshuffle_mini(frame_state);
        float p = max(factor.x, max(factor.y, factor.z)); // dot(factor, vec3(1.0 / 3.0));

        if (rng.x > p) {
            break;
        } else {
            factor /= p;
        }
    }

    color = vec4(accumulated * 0.000001, 1.0);
}
