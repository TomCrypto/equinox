struct ray_t {
    vec3 org;
    vec3 dir;
};

#define M_PI   3.14159265359
#define M_2PI  6.28318530718

// Maintains closest-hit information during a traversal.
struct traversal_t {
    uvec2 hit; // packed data for the closest SDF hit (geometry/material ID + parameter offsets)
    vec2 range; // min/max of the ray distance
};

traversal_t traversal_prepare(float near) {
    return traversal_t(uvec2(0xffffffffU), vec2(near, 1.0 / 0.0));
}

void traversal_record_hit(inout traversal_t traversal, float distance, uvec2 hit) {
    traversal = traversal_t(hit, vec2(traversal.range.x, distance));
}

bool traversal_has_hit(traversal_t traversal) {
    return traversal.hit.x != 0xffffffffU;
}

// Takes a ray segment and a bounding box and cuts the ray to be fully contained
// inside the bounding box. Returns true if the ray intersects the bounding box.
bool ray_bbox(vec3 org, vec3 idir, inout vec2 range, vec3 bbmin, vec3 bbmax) {
    vec3 bot = (bbmin - org) * idir;
    vec3 top = (bbmax - org) * idir;

    vec3 tmin = min(bot, top);
    vec3 tmax = max(bot, top);

    range.x = max(max(max(tmin.x, tmin.y), tmin.z), range.x);
    range.y = min(min(min(tmax.x, tmax.y), tmax.z), range.y);

    return range.x <= range.y;
}

vec3 _spherical_coordinates(float phi, float theta) {
    float sin_theta = sin(theta);

    return vec3(sin_theta * cos(phi), cos(theta), sin_theta * sin(phi));
}

// Transforms equirectangular coordinates into a unit direction vector with
// an optional custom rotation. The V = 0.5 line corresponds to a direction
// on the XZ plane, and (0.0, 0.5) will correspond to (1, 0, 0) by default.
vec3 equirectangular_to_direction(vec2 uv, float rotation) {
    return _spherical_coordinates(uv.x * M_2PI + rotation, uv.y * M_PI);
}

// Transforms a unit vector into equirectangular coordinates with a custom
// rotation. If a non-zero rotation is provided, the u-coordinate returned
// may be outside of the [0, 1] range and can be taken modulo 1 as needed.
vec2 direction_to_equirectangular(vec3 dir, float rotation) {
    return vec2((atan(dir.z, dir.x) - rotation) / M_2PI + 0.5, acos(dir.y) / M_PI);
}

// Picks an arbitrary rotation transforming the vector (0, 1, 0) into the provided
// normal and transforms the a-vector by that rotation. This is equivalent to constructing an arbitrary orthonormal basis where n is the Y-axis and
// transforming the given vector by it.

vec3 rotate(vec3 a, vec3 n) {
    float dir = (n.y > 0.0) ? 1.0 : -1.0; // not using sign() as it may return zero

    n.y += dir;

    return n * (dot(a, n) / n.y) - a * dir;
}
