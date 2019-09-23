#include <common.glsl>

uniform highp  sampler2D tex_hierarchy;
uniform highp  sampler2D tex_triangles;
uniform highp  sampler2D tex_vertex_positions;
uniform highp usampler2D tex_vertex_attributes;

void read_triangle_vertices(uint index, out vec3 p1, out vec3 e1, out vec3 e2) {
    int pixel_offset = int(index) * 4; // 4 pixels per triangle

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    p1 = texelFetch(tex_triangles, ivec2(w, h), 0).xyz;
    e1 = texelFetch(tex_triangles, ivec2(w + 1, h), 0).xyz;
    e2 = texelFetch(tex_triangles, ivec2(w + 2, h), 0).xyz;
}

uvec4 read_triangle_metadata(uint index) {
    int pixel_offset = int(index) * 4; // 4 pixels per triangle

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    return floatBitsToUint(texelFetch(tex_triangles, ivec2(w + 3, h), 0));
}

/*vec3 read_vertex_position(uint index) {
    int pixel_offset = int(index); // 1 pixel per vertex

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    return texelFetch(tex_vertex_positions, ivec2(w, h), 0).xyz;
}*/

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
void read_triangle_attributes(inout traversal_t traversal, out vec3 normal, out vec2 uv, out vec3 tangent, out uint mat) {
    uvec4 metadata = read_triangle_metadata(traversal.triangle);

    // construct a barycentric basis
    vec3 barycentric = vec3(1.0 - dot(traversal.hit.zw, vec2(1.0)), traversal.hit.zw);

    // put the vertex data in a matrix, 4 rows, 3 columns
    mat3x4 normU, tangV;

    read_vertex_attributes(metadata.x, normU[0], tangV[0]);
    read_vertex_attributes(metadata.y, normU[1], tangV[1]);
    read_vertex_attributes(metadata.z, normU[2], tangV[2]);

    // then just multiply with the barycentric coordinates!

    vec4 normUX = normU * barycentric;
    vec4 tangVX = tangV * barycentric;

    normal = normUX.xyz;
    tangent = tangVX.xyz;
    uv = vec2(normUX.w, tangVX.w);
    mat = metadata.w;

    // note: may need to orthogonalize normal and tangent here (may not be quite right)
}

// The "full" variants update the ray's max-t automatically if a hit occurs and return true
// Later on there may be faster functions that don't do any extra work to update the ray
// (e.g. for occlusion queries)

// if there is no hit, returns false and does nothing to the ray
// if there is a hit, returns true and updates the ray's max-t to that triangle
void ray_triangle_full(inout ray_t ray, uint triangle, inout traversal_t traversal) {
    // fetch p1, e1, e2
    vec3 p1, e1, e2;

    read_triangle_vertices(triangle, p1, e1, e2);

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

/*struct BvhNode {
    vec4 data1;
    vec4 data2;
};

void read_bvh_node(uint offset, out BvhNode node) {
    int pixel_offset = int(offset) * 2; // 2 pixels per BVH node

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    node.data1 = texelFetch(tex_hierarchy, ivec2(w + 0, h), 0);
    node.data2 = texelFetch(tex_hierarchy, ivec2(w + 1, h), 0);
}*/

struct BvhNode {
    vec4 lhs1;
    vec4 lhs2;
    vec4 rhs1;
    vec4 rhs2;
};

void read_bvh_node(uint offset, out BvhNode node) {
    int pixel_offset = int(offset) * 4; // 2 pixels per BVH node

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    node.lhs1 = texelFetch(tex_hierarchy, ivec2(w + 0, h), 0);
    node.lhs2 = texelFetch(tex_hierarchy, ivec2(w + 1, h), 0);
    node.rhs1 = texelFetch(tex_hierarchy, ivec2(w + 2, h), 0);
    node.rhs2 = texelFetch(tex_hierarchy, ivec2(w + 3, h), 0);
}

/*uvec4 read_triangle(uint index) {
    int pixel_offset = int(index); // 1 pixel per triangle!

    int w = pixel_offset % TBUF_WIDTH;
    int h = pixel_offset / TBUF_WIDTH;

    return texelFetch(tex_triangles, ivec2(w, h), 0);
}*/

void ray_bvh(ray_t ray, uvec4 indices, inout traversal_t traversal) {
    vec3 idir = vec3(1.0) / ray.dir;
    uint stack[20];

/*
    uint stack[20];
    uint stack_ptr = 1U;
    stack[0] = indices.x;

    while (stack_ptr != 0U) {
        BvhNode data;

        read_bvh_node(stack[--stack_ptr], data);

        uint ptr1 = floatBitsToUint(data.data1.w);
        uint ptr2 = floatBitsToUint(data.data2.w);

        if ((ptr1 & 0x80000000U) == 0U) {
            // MSB is not set, this is a NODE!
            // check for ray-AABB intersection, and then push the children if it is
            //  -> this is where we could select the push order?

            if (ray_bbox(ray.org, idir, data.data1.xyz, data.data2.xyz, traversal)) {
                stack[stack_ptr++] = ptr2;
                stack[stack_ptr++] = ptr1;
            }
        } else {
            ptr1 &= ~0x80000000U;
            // MSB is set, this is a LEAF!
            // iterate over the triangles...
            for (uint i = 0U; i < ptr1; ++i) {
                // uvec4 triangle = read_triangle(indices.y + ptr2 + i);
                ray_triangle_full(ray, indices.y + ptr2 + i, traversal);
            }
        }
    }
*/

/*

    uint node = indices.x;

    do {
        

        

        bool hitL = ray_bbox(ray.org, idir, data.lhs1.xyz, data.lhs2.xyz, traversal);
        bool hitR = ray_bbox(ray.org, idir, data.rhs1.xyz, data.rhs2.xyz, traversal);

        uint lhs_ptr = floatBitsToUint(data.lhs1.w);
        uint lhs_len = floatBitsToUint(data.lhs2.w);
        uint rhs_ptr = floatBitsToUint(data.rhs1.w);
        uint rhs_len = floatBitsToUint(data.rhs2.w);

        if (hitL && lhs_len != 0U) {
            for (uint i = 0U; i < lhs_len; ++i) {
                uvec4 triangle = read_triangle(indices.y + lhs_ptr + i);
                ray_triangle_full(ray, triangle + indices.zzzw, traversal);
            }
        }

        if (hitR && rhs_len != 0U) {
            for (uint i = 0U; i < rhs_len; ++i) {
                uvec4 triangle = read_triangle(indices.y + rhs_ptr + i);
                ray_triangle_full(ray, triangle + indices.zzzw, traversal);
            }
        }

        bool traverseL = (hitL && lhs_len == 0U);
        bool traverseR = (hitR && rhs_len == 0U);

        if (!traverseL && !traverseR) {
            node = stack[--stack_ptr];
        } else {
            node = (traverseL) ? lhs_ptr : rhs_ptr;
            if (traverseL && traverseR) {
                stack[stack_ptr++] = rhs_ptr;
            }
        }
    } while (node != 0xffffffffU);

*/

    stack[0] = indices.x; // root node
    uint idx = 1U;

    while (idx != 0U) {
        BvhNode node;
        int pushed = 0;

        float lhs_distance = -1e10;
        float rhs_distance = -1e10;

        read_bvh_node(stack[--idx], node);

        // do we intersect the LEFT node?
        if (ray_bbox_with_distance(ray.org, idir, node.lhs1.xyz, node.lhs2.xyz, traversal, lhs_distance)) {
            uint ptr = floatBitsToUint(node.lhs1.w);
            uint len = floatBitsToUint(node.lhs2.w);

            if (len == 0U) {
                // this is another node, push it on the stack for later
                stack[idx++] = ptr + indices.x;
                pushed += 1;
            } else {
                for (uint i = 0U; i < len; ++i) {
                    ray_triangle_full(ray, indices.y + ptr + i, traversal);
                }
            }
        }

        // do we intersect the RIGHT node?
        if (ray_bbox_with_distance(ray.org, idir, node.rhs1.xyz, node.rhs2.xyz, traversal, rhs_distance)) {
            uint ptr = floatBitsToUint(node.rhs1.w);
            uint len = floatBitsToUint(node.rhs2.w);

            if (len == 0U) {
                // this is another node, push it on the stack for later
                stack[idx++] = ptr + indices.x;
                pushed += 1;
            } else {
                for (uint i = 0U; i < len; ++i) {
                    ray_triangle_full(ray, indices.y + ptr + i, traversal);
                }
            }
        }

        if ((pushed == 2) && (lhs_distance < rhs_distance)) {
            uint tmp = stack[idx - 1U];
            stack[idx - 1U] = stack[idx - 2U];
            stack[idx - 2U] = tmp;
        }
    }
}
