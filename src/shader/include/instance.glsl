// requires-define INSTANCE_DATA_LEN
// requires-define INSTANCE_DATA_PRESENT

#include <geometry.glsl>

struct BvhNode {
    float minx;
    float miny;
    float minz;
    uint word1;
    float maxx;
    float maxy;
    float maxz;
    uint word2;
};

layout (std140) uniform Instance {
    BvhNode data[INSTANCE_DATA_LEN];
} instance_buffer;

void get_scene_bbox(out vec3 bbmin, out vec3 bbmax) {
    bbmin = vec3(instance_buffer.data[0].minx,
                 instance_buffer.data[0].miny,
                 instance_buffer.data[0].minz);
    bbmax = vec3(instance_buffer.data[0].maxx,
                 instance_buffer.data[0].maxy,
                 instance_buffer.data[0].maxz);
}

// NOTE: this algorithm now actually works whichever starting offset you use, as long as the
// termination condition is adjusted to stop as soon as you encounter the starting offset
// again.
// this only holds true if the ray origin is actually inside the starting offset's AABB...
// (to within the specified precision, i.e. it must be othervise visited during traversal)
// (if not this MAY LOOP FOREVER! must be absolutely surely, provably inside the AABB)

traversal_t traverse_scene(ray_t ray, uint start) {
    traversal_t traversal = traversal_prepare();

#if INSTANCE_DATA_PRESENT
    vec3 idir = vec3(1.0) / ray.dir;
    uint index = start;

    do {
        BvhNode node = instance_buffer.data[index++];

        vec3 bbmin = vec3(node.minx, node.miny, node.minz);
        vec3 bbmax = vec3(node.maxx, node.maxy, node.maxz);
        uint word1 = node.word1, word2 = node.word2;

        index *= uint((word1 & 0x00008000U) == 0U);
        word1 &= 0xffff7fffU; // remove cyclic bit

        vec2 range = traversal.range;

        if (ray_bbox(ray.org, idir, range, bbmin, bbmax)) {
            if (word2 != 0xffffffffU && geo_intersect(word1 & 0xffffU, word1 >> 16U, ray, range)) {
                traversal_record_hit(traversal, range.x, uvec2(word1, word2), index);
            }
        } else if (word2 == 0xffffffffU) {
            index = word1;
        }
    } while (index != start);
#endif

    return traversal;
}

// Tests whether a ray intersects any geometry in the scene. This is much faster
// than finding the closest intersection and is intended for visibility testing.

bool is_ray_occluded(ray_t ray, float limit) {
#if INSTANCE_DATA_PRESENT
    vec3 idir = vec3(1.0) / ray.dir;
    uint index = 0U;

    do {
        BvhNode node = instance_buffer.data[index++];

        vec3 bbmin = vec3(node.minx, node.miny, node.minz);
        vec3 bbmax = vec3(node.maxx, node.maxy, node.maxz);
        uint word1 = node.word1, word2 = node.word2;

        index *= uint((word1 & 0x00008000U) == 0U);
        word1 &= 0xffff7fffU; // remove cyclic bit

        vec2 range = vec2(0.0, limit);

        if (ray_bbox(ray.org, idir, range, bbmin, bbmax)) {
            if (word2 != 0xffffffffU && geo_intersect(word1 & 0xffffU, word1 >> 16U, ray, range)) {
                return true;
            }
        } else if (word2 == 0xffffffffU) {
            index = word1;
        }
    } while (index != 0U);
#endif

    return false;
}