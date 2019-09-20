precision highp float;

#include <random.glsl>

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

layout (std140, row_major) uniform Instances {
    Instance data[128];
} instances;

struct InstanceNode {
    vec4 lhs_min;
    vec4 lhs_max;
    vec4 rhs_min;
    vec4 rhs_max;
};

layout (std140) uniform InstanceHierarchy {
    InstanceNode data[127];
} instance_hierarchy;

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


uniform sampler2D bvh_data;
uniform highp usampler2D tri_data;
uniform sampler2D position_data;
uniform highp usampler2D normal_data;

struct Result {
    float distance;
    vec3 normal;
};

bool ray_bbox(vec3 origin, vec3 inv_dir, vec3 bmin, vec3 bmax) {
    vec3 bot = (bmin - origin) * inv_dir;
    vec3 top = (bmax - origin) * inv_dir;

    vec3 tmin = min(bot, top);
    vec3 tmax = max(bot, top);

    float near = max(max(tmin.x, tmin.y), tmin.z);
    float far = min(min(tmax.x, tmax.y), tmax.z);

    return (near <= far) && (far > 0.0);
}

vec3 ray_triangle(vec3 o, vec3 d, vec3 p1, vec3 e1, vec3 e2) {
    o -= p1;
    vec3 s = cross(d, e2);
    float de = 1.0f / dot(s, e1);

    float u = dot(o, s) * de;

    if ((u < 0.0) || (u > 1.0)) return vec3(0.0, 0.0, -1.0);

    s = cross(o, e1);
    float v = dot(d, s) * de;

    if ((v < 0.0) || (u + v > 1.0)) return vec3(0.0, 0.0, -1.0);

    return vec3(u, v, dot(e2, s) * de);
}

uvec4 read_triangle(uint index) {
    int pixel_offset = int(index); // 1 pixel per triangle!

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    return texelFetch(tri_data, ivec2(w, h), 0);
}

vec3 read_vertex_position(uint index) {
    int pixel_offset = int(index); // 1 pixel per vertex

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    return texelFetch(position_data, ivec2(w, h), 0).xyz;
}

vec3 read_vertex_normal(uint index) {
    int pixel_offset = int(index); // 1 pixel per vertex

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    uvec4 data = texelFetch(normal_data, ivec2(w, h), 0);

    vec2 nxny = unpackHalf2x16(data.x);
    float nz = unpackHalf2x16(data.y).x;

    return vec3(nxny, nz);
}

void read_bvh_node(uint offset, out vec4 value0, out vec4 value1) {
    int pixel_offset = int(offset) * 2;

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    value0 = texelFetch(bvh_data, ivec2(w + 0, h), 0);
    value1 = texelFetch(bvh_data, ivec2(w + 1, h), 0);
}

// TODO: pass in the instance data directly...
bool ray_bvh(vec3 origin, vec3 direction, uint offset, uint triangle_start, uint vertex_start, out Result result) {
    vec3 inv_dir = vec3(1.0) / direction;

    result.distance = 1e10;

    while (true) {
        vec4 elem1;
        vec4 elem2;

        read_bvh_node(offset, elem1, elem2);
        
        uint skip = floatBitsToUint(elem1.w);

        if (ray_bbox(origin, inv_dir, elem1.xyz, elem2.xyz)) {
            uint data = floatBitsToUint(elem2.w);

            if (data != uint(0)) {
                uvec4 tri = read_triangle(triangle_start + data - uint(1));

                vec3 p1 = read_vertex_position(vertex_start + tri.x);
                vec3 e1 = read_vertex_position(vertex_start + tri.y) - p1;
                vec3 e2 = read_vertex_position(vertex_start + tri.z) - p1;

                vec3 hit = ray_triangle(origin, direction, p1, e1, e2);

                if (hit.z > 0.0 && hit.z < result.distance) {
                    vec3 n0 = read_vertex_normal(vertex_start + tri.x);
                    vec3 n1 = read_vertex_normal(vertex_start + tri.y);
                    vec3 n2 = read_vertex_normal(vertex_start + tri.z);

                    vec3 normal = normalize(n1 * hit.x + n2 * hit.y + n0 * (1.0 - hit.x - hit.y));

                    result.distance = hit.z;
                    result.normal = normal;
                }
            }

            if ((skip & 0x80000000U) != 0U) {
                break;
            }

            offset += uint(1);
        } else {
            if ((skip & 0x40000000U) != 0U) {
                break;
            }

            offset += skip & ~0xC0000000U;
        }
    }

    return result.distance < 1e10;
}

bool ray_bvh_occlusion(vec3 origin, vec3 direction, uint offset, uint triangle_start, uint vertex_start) {
    vec3 inv_dir = vec3(1.0) / direction;

    while (true) {
        vec4 elem1;
        vec4 elem2;

        read_bvh_node(offset, elem1, elem2);

        uint skip = floatBitsToUint(elem1.w);

        if (ray_bbox(origin, inv_dir, elem1.xyz, elem2.xyz)) {
            uint data = floatBitsToUint(elem2.w);

            if (data != uint(0)) {
                uvec4 tri = read_triangle(triangle_start + data - uint(1));

                vec3 p1 = read_vertex_position(vertex_start + tri.x);
                vec3 e1 = read_vertex_position(vertex_start + tri.y) - p1;
                vec3 e2 = read_vertex_position(vertex_start + tri.z) - p1;

                vec3 hit = ray_triangle(origin, direction, p1, e1, e2);

                if (hit.z > 0.0) {
                    return true;
                }
            }

            if ((skip & 0x80000000U) != 0U) {
                break;
            }

            offset += uint(1);
        } else {
            if ((skip & 0x40000000U) != 0U) {
                break;
            }

            offset += skip & ~0xC0000000U;
        }
    }

    return false;
}

bool traverse_scene_bvh(vec3 origin, vec3 direction, out Result result) {
    uint stack[7]; // equal to max BVH depth + 1, so log2(128) = 7? TODO: find the exact number
    
    vec3 inv_dir = vec3(1.0) / direction;

    result.distance = 1e10;

    stack[0] = 0U;
    uint idx = 1U;

    while (idx != 0U) {
        InstanceNode node = instance_hierarchy.data[stack[--idx]];

        // TODO: optimize this traversal later

        // do we intersect the LEFT node?
        if (ray_bbox(origin, inv_dir, node.lhs_min.xyz, node.lhs_max.xyz)) {
            uint next = floatBitsToUint(node.lhs_min.w);
            uint inst = floatBitsToUint(node.lhs_max.w);

            if (next == 0xffffffffU) {
                // this is a leaf node, traverse it
                mat4x3 xfm = instances.data[inst].transform;

                vec3 new_origin = xfm * vec4(origin, 1.0);
                vec3 new_direction = xfm * vec4(direction, 0.0);

                Result tmp;

                if (ray_bvh(new_origin, new_direction, instances.data[inst].indices.x, instances.data[inst].indices.y, instances.data[inst].indices.z, tmp)) {
                    if (tmp.distance < result.distance) {
                        result = tmp;
                    }
                }
            } else {
                // this is another node, push it on the stack
                stack[idx++] = next;
            }
        }

        // do we intersect the LEFT node?
        if (ray_bbox(origin, inv_dir, node.rhs_min.xyz, node.rhs_max.xyz)) {
            uint next = floatBitsToUint(node.rhs_min.w);
            uint inst = floatBitsToUint(node.rhs_max.w);

            if (next == 0xffffffffU) {
                // this is a leaf node, traverse it
                mat4x3 xfm = instances.data[inst].transform;

                vec3 new_origin = xfm * vec4(origin, 1.0);
                vec3 new_direction = xfm * vec4(direction, 0.0);

                Result tmp;

                if (ray_bvh(new_origin, new_direction, instances.data[inst].indices.x, instances.data[inst].indices.y, instances.data[inst].indices.z, tmp)) {
                    if (tmp.distance < result.distance) {
                        result = tmp;
                    }
                }
            } else {
                // this is another node, push it on the stack
                stack[idx++] = next;
            }
        }
    }

    return result.distance < 1e10;
}

bool traverse_scene_bvh_occlusion(vec3 origin, vec3 direction) {
    uint stack[7]; // equal to max BVH depth + 1, so log2(128) = 7? TODO: find the exact number
    
    vec3 inv_dir = vec3(1.0) / direction;

    stack[0] = 0U;
    uint idx = 1U;

    while (idx != 0U) {
        InstanceNode node = instance_hierarchy.data[stack[--idx]];

        // TODO: optimize this traversal later

        // do we intersect the LEFT node?
        if (ray_bbox(origin, inv_dir, node.lhs_min.xyz, node.lhs_max.xyz)) {
            uint next = floatBitsToUint(node.lhs_min.w);
            uint inst = floatBitsToUint(node.lhs_max.w);

            if (next == 0xffffffffU) {
                // this is a leaf node, traverse it
                mat4x3 xfm = instances.data[inst].transform;

                vec3 new_origin = xfm * vec4(origin, 1.0);
                vec3 new_direction = xfm * vec4(direction, 0.0);

                if (ray_bvh_occlusion(new_origin, new_direction, instances.data[inst].indices.x, instances.data[inst].indices.y, instances.data[inst].indices.z)) {
                    return true;
                }
            } else {
                // this is another node, push it on the stack
                stack[idx++] = next;
            }
        }

        // do we intersect the LEFT node?
        if (ray_bbox(origin, inv_dir, node.rhs_min.xyz, node.rhs_max.xyz)) {
            uint next = floatBitsToUint(node.rhs_min.w);
            uint inst = floatBitsToUint(node.rhs_max.w);

            if (next == 0xffffffffU) {
                // this is a leaf node, traverse it
                mat4x3 xfm = instances.data[inst].transform;

                vec3 new_origin = xfm * vec4(origin, 1.0);
                vec3 new_direction = xfm * vec4(direction, 0.0);

                if (ray_bvh_occlusion(new_origin, new_direction, instances.data[inst].indices.x, instances.data[inst].indices.y, instances.data[inst].indices.z)) {
                    return true;
                }
            } else {
                // this is another node, push it on the stack
                stack[idx++] = next;
            }
        }
    }

    return false;
}

// Low-discrepancy sequence generator.
//
// Given a fixed, unchanging key, this will produce a low-discrepancy sequence of 2D points
// as a function of frame number, e.g. on the next frame for the same key the next point in
// the sequence will be produced. The key should be <= 2^16 to prevent precision problems.

vec2 low_discrepancy_2d(uvec2 key) {
    return fract(vec2(key + FRAME_NUMBER) * vec2(0.7548776662, 0.5698402909));
}

// Begin camera stuff

vec2 evaluate_circular_aperture_uv(uvec2 pixel_state) {
    vec2 uv = low_discrepancy_2d(pixel_state >> 16U);

    float a = uv.s * M_2PI;

    return sqrt(uv.t) * vec2(cos(a), sin(a));
}

vec2 evaluate_polygon_aperture_uv(uvec2 pixel_state) {
    vec2 uv = low_discrepancy_2d(pixel_state >> 16U);

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

    vec3 pos;
    vec3 dir;
    evaluate_primary_ray(pixel_state, pos, dir);

    Result result;

    // intersect the BVH
    if (traverse_scene_bvh(pos, dir, result)) {
        pos = pos + dir * result.distance + result.normal * 1e-3;

        vec2 rng = gen_vec2_uniform(frame_state);
        bitshuffle_mini(frame_state);

        float theta = 2.0 * 3.14159165 * rng.x;
        float phi = acos(2.0 * rng.y - 1.0);

        dir = vec3(cos(theta) * sin(phi), cos(phi), sin(theta) * sin(phi));

        dir *= sign(dot(dir, result.normal));

        if (traverse_scene_bvh_occlusion(pos, dir)) {
            color = vec4(0.0, 0.0, 0.0, 1.0);
        } else {
            color = vec4(0.75, 0.95, 0.65, 1.0);
        }

        // color = vec4(0.5 * result.normal + 0.5, 1.0);
    } else {
        color = vec4(0.0, 0.0, 0.0, 1.0);
    }
}
