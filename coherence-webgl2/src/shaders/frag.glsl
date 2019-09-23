#include <common.glsl>
#include <random.glsl>
// #include <object.glsl>

#define M_PI   3.14159265359
#define M_2PI  6.28318530718

out vec4 color;

layout (std140) uniform Camera {
    vec4 origin_plane[4];
    vec4 target_plane[4];
    vec4 aperture_settings;
} camera;

struct Instance {
    mat4x3 transform;
    uvec4 indices;
};

/*layout (std140, row_major) uniform Instances {
    Instance data[128];
} instances;*/

struct InstanceNode {
    vec4 lhs_min;
    vec4 lhs_max;
    vec4 rhs_min;
    vec4 rhs_max;
};

/*layout (std140) uniform InstanceHierarchy {
    InstanceNode data[127];
} instance_hierarchy;*/

layout (std140) uniform Globals {
    vec2 filter_delta;
    uvec4 frame_state;
} globals;

struct Material {
    vec4 info;
};

/*layout (std140) uniform MaterialLookup {
    uint index[128];
} material_lookup;

layout (std140) uniform Materials {
    Material data[128];
} materials;*/

#define FILTER_DELTA (globals.filter_delta)
#define FRAME_RANDOM (globals.frame_state.xy)
#define FRAME_NUMBER (globals.frame_state.z)

layout (std140) uniform Raster {
    vec4 dimensions;
} raster;

layout (std140) uniform GeometryValues {
    vec4 data[64];
} geometry_values;

layout (std140) uniform MaterialValues {
    vec4 data[64];
} material_values;

struct BvhNode {
    vec4 data1;
    vec4 data2;
};

layout (std140) uniform InstanceHierarchy {
    BvhNode data[256];
} instance_hierarchy;

// TODO: put this somewhere else eventually... we need to know the BVH size to traverse it
// unfortunately or we have no idea when to stop
#define BVH_LIMIT 3U

#define PREC 1e-5

SDF_CODE

bool traverse_sdf(ray_t ray, uint geometry, uint instance, inout vec2 range) {
    while (range.x < range.y) {
        float dist = sdf(geometry, instance, ray.org + range.x * ray.dir);

        if (dist < PREC) {
            return true;
        }

        range.x += dist;
    }

    return false;
}

bool traverse_scene_bvh(ray_t ray, inout traversal_t traversal) {
    vec3 idir = vec3(1.0) / ray.dir;

    traversal.hit = uvec2(0U, 0U);
    traversal.range = vec2(PREC * 100.0, 1e10);

    uint offset = 0U;

    do {
        BvhNode node = instance_hierarchy.data[offset];

        uint packed1 = floatBitsToUint(node.data1.w);
        uint packed2 = floatBitsToUint(node.data2.w);

        vec2 range = traversal.range;

        if (ray_bbox(ray.org, idir, node.data1.xyz, node.data2.xyz, range)) {
            if (packed2 != 0xffffffffU) {
                if (traverse_sdf(ray, packed1 & 0xffffU, packed1 >> 16U, range)) {
                    traversal.range.y = range.x; // new closest
                    traversal.hit = uvec2(packed1, packed2);
                }
            }

            offset += 1U;
        } else {
            offset = (packed2 == 0xffffffffU) ? packed1 : (offset + 1U);
        }
    } while (offset != BVH_LIMIT);

    return traversal.range.y != 1e10;
}

// Low-discrepancy sequence generator.
//
// Given a fixed, unchanging key, this will produce a low-discrepancy sequence of 2D points
// as a function of frame number, e.g. on the next frame for the same key the next point in
// the sequence will be produced.

vec2 low_discrepancy_2d(uvec2 key) {
    return fract(vec2((key + FRAME_NUMBER) % 8192U) * vec2(0.7548776662, 0.5698402909));
}

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
        traversal_t traversal;

        if (traverse_scene_bvh(ray, traversal)) {
            ray.org += ray.dir * traversal.range.y; // closest distance to triangle

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


                    factor *= material_values.data[offset + 0U].xyz;
                    break;
                }
                case 1U: {
                    // specular
                    // reflect the ray off the normal and continue; assume perfect reflection so no change
                    // in factor

                    ray.dir = reflect(ray.dir, normal);
                    break;
                }
                case 2U: {
                    // emissive
                    // terminate the ray

                    accumulated += factor * material_values.data[offset + 0U].xyz;
                    factor = vec3(0.0);
                    break;
                }
                default:
                    return; // bug (TODO: do something coherent on these kinds of bugs)
            }

            // color = vec4(normal * 0.5 + 0.5, 1.0);
            // return;
        } else {
            // we've escaped, break out
            break;
        }

        // russian roulette

        vec2 rng = gen_vec2_uniform(frame_state);
        bitshuffle_mini(frame_state);
        float p = max(factor.x, max(factor.y, factor.z)); // dot(factor, vec3(1.0 / 3.0));

        if (rng.x > p) {
            factor = vec3(0.0);
            break;
        } else {
            factor /= p;
        }
    }

    // brightness...
    color = vec4(accumulated + vec3(0.0) * factor, 1.0);
}
