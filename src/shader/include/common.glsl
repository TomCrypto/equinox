struct ray_t {
    vec3 org;
    vec3 dir;
};

#define PREC (1e-4) // general precision for interacting with the distance fields

#define M_PI   3.14159265359
#define M_2PI  6.28318530718
#define M_4PI  12.5663706144

// Maintains closest-hit information during a traversal.
struct traversal_t {
    uvec3 hit; // packed data for the closest SDF hit (geometry/material ID + parameter offsets)
    vec2 range; // min/max of the ray distance
};

ray_t make_ray(vec3 org, vec3 dir, vec3 normal) {
    return ray_t(org + normal * 10.0 * PREC * sign(dot(dir, normal)), dir);
}

traversal_t traversal_prepare() {
    return traversal_t(uvec3(0xffffffffU), vec2(0.0, 1.0 / 0.0));
}

void traversal_record_hit(inout traversal_t traversal, float distance, uvec2 hit, uint index) {
    traversal = traversal_t(uvec3(hit, index), vec2(traversal.range.x, distance));
}

bool traversal_has_hit(traversal_t traversal) {
    return traversal.hit.x != 0xffffffffU;
}

float luminance(vec3 color) {
    return dot(color, vec3(0.2126, 0.7152, 0.0722));
}

// Takes a ray segment and a bounding box and clips the ray to be fully contained
// inside the bounding box. Returns true if the ray intersected the bounding box.
bool ray_bbox(vec3 org, vec3 idir, inout vec2 range, vec3 bbmin, vec3 bbmax) {
    vec3 bot = (bbmin - PREC - org) * idir;
    vec3 top = (bbmax + PREC - org) * idir;

    vec3 tmin = min(bot, top);
    vec3 tmax = max(bot, top);

    range.x = max(max(max(tmin.x, tmin.y), tmin.z), range.x);
    range.y = min(min(min(tmax.x, tmax.y), tmax.z), range.y);

    return range.x <= range.y;
}

// Transforms the given (phi, theta) azimuth/elevation angles into a direction
// vector with the north pole being (0, 1, 0). The vector will be unit length.
vec3 to_spherical(float phi, float theta) {
    float sin_theta = sin(theta);

    return vec3(sin_theta * cos(phi), cos(theta), sin_theta * sin(phi));
}

// Transforms equirectangular coordinates into a unit direction vector with
// an optional custom rotation. The V = 0.5 line corresponds to a direction
// on the XZ plane, and (0.0, 0.5) will correspond to (1, 0, 0) by default.
vec3 equirectangular_to_direction(vec2 uv, float rotation) {
    return to_spherical(rotation - uv.x * M_2PI, uv.y * M_PI);
}

// Transforms a unit vector into equirectangular coordinates with a custom
// rotation. The u-coordinate returned may be outside of the [0, 1] range.
vec2 direction_to_equirectangular(vec3 dir, float rotation) {
    return vec2((rotation - atan(dir.z, dir.x)) / M_2PI, acos(dir.y) / M_PI);
}

// Rotates an arbitrary vector "a" by an arbitrarily chosen rotation which
// takes the (0, 1, 0) vector to the "n" vector ("n" must be unit length).
vec3 rotate(vec3 a, vec3 n) {
    float dir = (n.y > 0.0) ? 1.0 : -1.0;
    n.y += dir; // avoids extra register

    return n * (dot(a, n) / n.y) - a * dir;
}

float power_heuristic(float f, float g) {
    f *= f;
    g *= g;

    return f / (f + g);
}

// Feeds an input value through a keyed pseudorandom permutation, to decorrelate
// a correlated sequence; this can suppress visual artifacts when used properly.
uint decorrelate_sample(uint x, uint key) {
    x ^= key;
    x ^= x >> 17U;
    x ^= x >> 10U;
    x *= 0xb36534e5U;
    x ^= x >> 12U;
    x ^= x >> 21U;
    x *= 0x93fc4795U;
    x ^= 0xdf6e307fU;
    x ^= x >> 17U;
    x *= 1U | key >> 18U;

    return x;
}

uint decorrelate_sample(uint x) {
    // pass in a default key when not given one
    return decorrelate_sample(x, 0xa8f4c2c1U);
}
