#include <geometry.glsl>

struct BvhNode {
    vec4 data1;
    vec4 data2;
};

layout (std140) uniform Instance {
    BvhNode data[256];
} instance_buffer;

// NOTE: this algorithm now actually works whichever starting offset you use, as long as the
// termination condition is adjusted to stop as soon as you encounter the starting offset
// again.
// this only holds true if the ray origin is actually inside the starting offset's AABB...
// (to within the specified precision, i.e. it must be othervise visited during traversal)
// (if not this MAY LOOP FOREVER! must be absolutely surely, provably inside the AABB)

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
            if (word2 != 0xffffffffU && ray_sdf(ray, word1 & 0xffffU, word1 >> 16U, range)) {
                traversal_record_hit(traversal, range.x, uvec2(word1, word2));
            }
        } else if (word2 == 0xffffffffU) {
            index = word1;
        }
    } while (index != 0U);

    return traversal;
}

// Tests whether a ray intersects any geometry in the scene. This is much faster
// than finding the closest intersection and is intended for visibility testing.

bool is_ray_occluded(ray_t ray, float distance) {
    vec3 idir = vec3(1.0) / ray.dir;

    uint index = 0U;

    do {
        BvhNode node = instance_buffer.data[index++];

        uint word1 = floatBitsToUint(node.data1.w);
        uint word2 = floatBitsToUint(node.data2.w);

        index *= uint((word1 & 0x00008000U) == 0U);
        word1 &= 0xffff7fffU; // remove cyclic bit

        vec2 range = vec2(PREC * 100.0, distance);

        if (ray_bbox(ray.org, idir, range, node.data1.xyz, node.data2.xyz)) {
            if (word2 != 0xffffffffU && ray_sdf(ray, word1 & 0xffffU, word1 >> 16U, range)) {
                return true;
            }
        } else if (word2 == 0xffffffffU) {
            index = word1;
        }
    } while (index != 0U);

    return false;
}

