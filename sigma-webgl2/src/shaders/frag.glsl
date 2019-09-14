#version 300 es

precision highp float;

in vec2 clip;

out vec4 color;

layout (std140) uniform Camera {
    vec3 fp0;
    vec3 fp1;
    vec3 fp2;
    vec3 fp3;
    vec3 pos;
} camera;

struct Instance {
    mat4x3 transform;
    uvec4 indices;
};

layout (std140, row_major) uniform Instances {
    Instance data[128];
} instances;


uniform uint instance_count;

uniform sampler2D bvh_data;
uniform sampler2D tri_data;

uniform uint seed;

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

float rand(inout uint state) {
    state ^= state << 13;
    state ^= state >> 17;
    state ^= state << 5;

    return float((state / uint(65536))) / 65535.0;
}

void main() {
    uint random_state = (uint(gl_FragCoord.x + 0.5) * uint(7193) + uint(gl_FragCoord.y + 0.5) * uint(3719)) * seed;

    float dx = (rand(random_state) - 0.5) / 1920.0;
    float dy = (rand(random_state) - 0.5) / 1020.0;

    // generate a ray to trace the world against

    vec3 dir = normalize(mix(mix(camera.fp0, camera.fp1, clip.x + dx), mix(camera.fp2, camera.fp3, clip.x + dx), 1.0 - clip.y - dy));
    vec3 pos = camera.pos;

    Result result;

    // intersect the BVH
    if (intersect_world(pos, dir, result)) {
        pos = pos + dir * result.distance + result.normal * 1e-3;

        float u = rand(random_state);
        float v = rand(random_state);

        float theta = 2.0 * 3.14159165 * u;
        float phi = acos(2.0 * v - 1.0);

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
