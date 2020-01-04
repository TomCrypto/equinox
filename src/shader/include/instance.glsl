// requires-define INSTANCE_DATA_LEN
// requires-define INSTANCE_DATA_PRESENT
// requires-define PREC
// requires-define PUSHBACK

#include <geometry.glsl>

// Maintains closest-hit information during a traversal.
struct traversal_t {
    uvec2 hit; // packed data for the closest SDF hit (geometry/material ID + parameter offsets)
    vec2 range; // min/max of the ray distance
};

ray_t make_ray(vec3 org, vec3 dir, vec3 normal) {
    return ray_t(org + normal * PUSHBACK * PREC * sign(dot(dir, normal)), dir);
}

traversal_t traversal_prepare() {
    return traversal_t(uvec2(0xffffffffU), vec2(0.0, 1.0 / 0.0));
}

void traversal_record_hit(inout traversal_t traversal, float distance, uvec2 hit) {
    traversal = traversal_t(uvec2(hit), vec2(traversal.range.x, distance));
}

bool traversal_has_hit(traversal_t traversal) {
    return traversal.hit.x != 0xffffffffU;
}

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

vec3 medium_absorption(uint inst, bool inside, float distance, out float n1, out float n2) {
    const vec3 WAVENUMBERS = M_2PI / vec3(685e-9, 530e-9, 470e-9);

    vec4 ext_medium = geometry_buffer.data[inst + 0U];
    vec4 int_medium = geometry_buffer.data[inst + 1U];

    if (inside) {
        n1 = int_medium.w;
        n2 = ext_medium.w;

        return exp(-int_medium.xyz * int_medium.w * distance * WAVENUMBERS);
    } else {
        n1 = ext_medium.w;
        n2 = int_medium.w;

        return exp(-ext_medium.xyz * ext_medium.w * distance * WAVENUMBERS);
    }
}

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

        if (ray_bbox(ray.org, idir, range, bbmin - PREC, bbmax + PREC)) {
            if (word2 != 0xffffffffU && geo_intersect(word1 & 0xffffU, word1 >> 16U, ray, range)) {
                traversal_record_hit(traversal, range.x, uvec2(word1, word2));
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

        if (ray_bbox(ray.org, idir, range, bbmin - PREC, bbmax + PREC)) {
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
