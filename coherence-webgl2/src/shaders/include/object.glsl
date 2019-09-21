#include <common.glsl>

uniform highp  sampler2D tex_hierarchy;
uniform highp usampler2D tex_triangles;
uniform highp  sampler2D tex_vertex_positions;
uniform highp usampler2D tex_vertex_attributes;

vec3 read_vertex_position(uint index) {
    int pixel_offset = int(index); // 1 pixel per vertex

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    return texelFetch(tex_vertex_positions, ivec2(w, h), 0).xyz;
}

void read_vertex_attributes(uint vertex, out vec4 normU, out vec4 tangV) {
    int pixel_offset = int(vertex); // 1 pixel per vertex

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    uvec4 data = texelFetch(tex_vertex_attributes, ivec2(w, h), 0);

    normU = vec4(unpackHalf2x16(data.x), unpackHalf2x16(data.y));
    tangV = vec4(unpackHalf2x16(data.z), unpackHalf2x16(data.w));
}

// note: could make UV/tangent optional here (e.g. if the material does not need them)
// as it would save quite a bit of work
void read_triangle_attributes(inout traversal_t traversal, out vec3 normal, out vec2 uv, out vec3 tangent) {
    // construct a barycentric basis
    vec3 barycentric = vec3(1.0 - dot(traversal.hit.zw, vec2(1.0)), traversal.hit.zw);

    // put the vertex data in a matrix, 4 rows, 3 columns
    mat3x4 normU, tangV;

    read_vertex_attributes(traversal.triangle.x, normU[0], tangV[0]);
    read_vertex_attributes(traversal.triangle.y, normU[1], tangV[1]);
    read_vertex_attributes(traversal.triangle.z, normU[2], tangV[2]);

    // then just multiply with the barycentric coordinates!

    vec4 normUX = normU * barycentric;
    vec4 tangVX = tangV * barycentric;

    normal = normUX.xyz;
    tangent = tangVX.xyz;
    uv = vec2(normUX.w, tangVX.w);

    // note: may need to orthogonalize normal and tangent here (may not be quite right)
}

// The "full" variants update the ray's max-t automatically if a hit occurs and return true
// Later on there may be faster functions that don't do any extra work to update the ray
// (e.g. for occlusion queries)

// if there is no hit, returns false and does nothing to the ray
// if there is a hit, returns true and updates the ray's max-t to that triangle
void ray_triangle_full(inout ray_t ray, uvec4 triangle, inout traversal_t traversal) {
    // fetch the triangle's vertex positions
    // (this is a bit sad...)

    vec3 p1 = read_vertex_position(triangle.x);
    vec3 e1 = read_vertex_position(triangle.y) - p1;
    vec3 e2 = read_vertex_position(triangle.z) - p1;

    vec3 o = ray.org - p1;
    vec3 s = cross(ray.dir, e2);
    float de = 1.0 / dot(s, e1);

    float u = dot(o, s) * de;

    if (u < 0.0 || u > 1.0) {
        return;
    }

    s = cross(o, e1);
    float v = dot(ray.dir, s) * de;

    if (v < 0.0 || u + v > 1.0) {
        return;
    }

    // IF the distance is > minT and < maxT, then UPDATE the ray and return TRUE
    // else, return FALSE without UPDATING the ray

    float t = dot(e2, s) * de;

    // if (clamp(t, traversal.hit.x, traversal.hit.y) == t) {
    if (t > traversal.hit.x && t < traversal.hit.y) {
        // we have a hit! don't update min-t of course
        traversal.hit.yzw = vec3(t, u, v);
        traversal.triangle = triangle;
    }
}

void read_bvh_node(uint offset, out vec4 value0, out vec4 value1) {
    int pixel_offset = int(offset) * 2;

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    value0 = texelFetch(tex_hierarchy, ivec2(w + 0, h), 0);
    value1 = texelFetch(tex_hierarchy, ivec2(w + 1, h), 0);
}

uvec4 read_triangle(uint index) {
    int pixel_offset = int(index); // 1 pixel per triangle!

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    return texelFetch(tex_triangles, ivec2(w, h), 0);
}

void ray_bvh(ray_t ray, uvec4 indices, inout traversal_t traversal) {
    vec3 idir = vec3(1.0) / ray.dir;

    uint offset = indices.x; // + 1 if we assume we've already checked the root

    while (true) {
        vec4 elem1;
        vec4 elem2;

        read_bvh_node(offset, elem1, elem2);
        
        uint skip = floatBitsToUint(elem1.w);

        if (ray_bbox(ray.org, idir, elem1.xyz, elem2.xyz, traversal)) {
            uint data = floatBitsToUint(elem2.w);

            if (data != uint(0)) {
                // this is a leaf, grab the triangle data
                uvec4 triangle = read_triangle(indices.y + data - uint(1));

                ray_triangle_full(ray, triangle + indices.zzzw, traversal);
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
}
