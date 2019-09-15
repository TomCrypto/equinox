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


uniform uint instance_count;

uniform sampler2D bvh_data;
uniform sampler2D tri_data;

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

float ray_triangle(vec3 o, vec3 d, vec3 p1, vec3 e1, vec3 e2) {
    o -= p1;
    vec3 s = cross(d, e2);
    float de = 1.0f / dot(s, e1);

    float u = dot(o, s) * de;

    if ((u < 0.0) || (u > 1.0)) return -1.0;

    s = cross(o, e1);
    float v = dot(d, s) * de;

    if ((v < 0.0) || (u + v > 1.0)) return -1.0;

    return dot(e2, s) * de;
}

struct Triangle {
    vec3 p1;
    vec3 e1;
    vec3 e2;
    vec3 n;
};

void read_triangle(uint index, out Triangle triangle) {
    int pixel_offset = int(index) * 4; // 4 pixels per triangle!

    int w = pixel_offset % 4096;
    int h = pixel_offset / 4096;

    triangle.p1 = texelFetch(tri_data, ivec2(w + 0, h), 0).xyz;
    triangle.e1 = texelFetch(tri_data, ivec2(w + 1, h), 0).xyz;
    triangle.e2 = texelFetch(tri_data, ivec2(w + 2, h), 0).xyz;
    triangle.n  = texelFetch(tri_data, ivec2(w + 3, h), 0).xyz;
}

void read_bvh_node(uint offset, out vec4 value0, out vec4 value1) {
    int pixel_offset = int(offset) * 2;

    int w = pixel_offset % 4096;
    int h = pixel_offset / 4096;

    value0 = texelFetch(bvh_data, ivec2(w + 0, h), 0);
    value1 = texelFetch(bvh_data, ivec2(w + 1, h), 0);
}

bool ray_bvh(vec3 origin, vec3 direction, uint offset, uint limit, uint triangle_start, out Result result) {
    vec3 inv_dir = vec3(1.0) / direction;

    result.distance = 1e10;

    while (offset != limit) {
        vec4 elem1;
        vec4 elem2;

        read_bvh_node(offset, elem1, elem2);

        if (ray_bbox(origin, inv_dir, elem1.xyz, elem2.xyz)) {
            uint data = floatBitsToUint(elem2.w);

            if (data != uint(0)) {
                Triangle triangle;

                read_triangle(triangle_start + data - uint(1), triangle);

                float distance = ray_triangle(origin, direction, triangle.p1, triangle.e1, triangle.e2);

                if (distance > 0.0 && distance < result.distance) {
                    result.distance = distance;
                    result.normal = triangle.n; // TODO
                }
            }

            offset += uint(1);
        } else {
            offset += floatBitsToUint(elem1.w);
        }
    }

    return result.distance < 1e10;
}

bool ray_bvh_occlusion(vec3 origin, vec3 direction, uint offset, uint limit, uint triangle_start) {
    vec3 inv_dir = vec3(1.0) / direction;

    while (offset != limit) {
        vec4 elem1;
        vec4 elem2;

        read_bvh_node(offset, elem1, elem2);

        if (ray_bbox(origin, inv_dir, elem1.xyz, elem2.xyz)) {
            uint data = floatBitsToUint(elem2.w);

            if (data != uint(0)) {
                Triangle triangle;

                read_triangle(triangle_start + data - uint(1), triangle);

                float distance = ray_triangle(origin, direction, triangle.p1, triangle.e1, triangle.e2);

                if (distance > 0.0) {
                    return true;
                }
            }

            offset += uint(1);
        } else {
            offset += floatBitsToUint(elem1.w);
        }
    }

    return false;
}

bool intersect_world(vec3 origin, vec3 direction, out Result result) {
    result.distance = 1e10;

    for (uint i = uint(0); i < instance_count; ++i) {
        mat4x3 xfm = instances.data[i].transform;

        vec3 new_origin = xfm * vec4(origin, 1.0);
        vec3 new_direction = xfm * vec4(direction, 0.0);

        Result tmp;

        if (ray_bvh(new_origin, new_direction, instances.data[i].indices.x, instances.data[i].indices.y, instances.data[i].indices.z, tmp)) {
            if (tmp.distance < result.distance) {
                result = tmp;
            }
        }
    }

    return result.distance < 1e10;
}

bool intersect_world_occlusion(vec3 origin, vec3 direction) {
    for (uint i = uint(0); i < instance_count; ++i) {
        mat4x3 xfm = instances.data[i].transform;

        vec3 new_origin = xfm * vec4(origin, 1.0);
        vec3 new_direction = xfm * vec4(direction, 0.0);

        if (ray_bvh_occlusion(new_origin, new_direction, instances.data[i].indices.x, instances.data[i].indices.y, instances.data[i].indices.z)) {
            return true;
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

// end of random stuff

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
    if (intersect_world(pos, dir, result)) {
        pos = pos + dir * result.distance + result.normal * 1e-3;

        vec2 rng = gen_vec2_uniform(frame_state);
        bitshuffle_mini(frame_state);

        float theta = 2.0 * 3.14159165 * rng.x;
        float phi = acos(2.0 * rng.y - 1.0);

        dir = vec3(cos(theta) * sin(phi), cos(phi), sin(theta) * sin(phi));

        dir *= sign(dot(dir, result.normal));

        if (intersect_world_occlusion(pos, dir)) {
            color = vec4(0.0, 0.0, 0.0, 1.0);
        } else {
            color = vec4(0.75, 0.95, 0.65, 1.0);
        }
    } else {
        color = vec4(0.0, 0.0, 0.0, 1.0);
    }
}
