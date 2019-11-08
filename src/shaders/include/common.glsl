struct ray_t {
    vec3 org;
    vec3 dir;
};

#define PREC (1e-3) // general precision for interacting with the distance fields

#define M_PI   3.14159265359
#define M_2PI  6.28318530718
#define M_4PI  12.5663706144

// Maintains closest-hit information during a traversal.
struct traversal_t {
    uvec3 hit; // packed data for the closest SDF hit (geometry/material ID + parameter offsets)
    vec2 range; // min/max of the ray distance
};

ray_t make_ray(vec3 org, vec3 dir, vec3 normal) {
    return ray_t(org + normal * PREC * sign(dot(dir, normal)), dir);
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

// Takes a ray segment and a bounding box and clips the ray to be fully contained
// inside the bounding box. Returns true if the ray intersected the bounding box.
bool ray_bbox(vec3 org, vec3 idir, inout vec2 range, vec3 bbmin, vec3 bbmax) {
    vec3 bot = (bbmin - org) * idir;
    vec3 top = (bbmax - org) * idir;

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

bool unpack_visible_point(vec4 data1, vec4 data2, vec4 data3, out vec3 position, out vec3 direction,
                          out vec3 normal, out vec3 throughput, out uint material, out uint inst) {
    uint mat_info = floatBitsToUint(data1.w);

    throughput = data2.xyz;
    
    if (mat_info == 0xffffffffU) {
        return false;
    }

    position = data1.xyz;
    material = mat_info & 0xffffU;
    inst = mat_info >> 16U;

    direction.xz = data3.xy;
    normal.xz = data3.zw;

    direction.y = sqrt(max(0.0, 1.0 - direction.x * direction.x - direction.z * direction.z));
    normal.y = sqrt(max(0.0, 1.0 - normal.x * normal.x - normal.z * normal.z));

    int signs = int(data2.w);

    if ((signs & 0x1) != 0) {
        direction.y = -direction.y;
    }

    if ((signs & 0x2) != 0) {
        normal.y = -normal.y;
    }

    return true;
}

void pack_visible_point(vec3 position, vec3 direction, vec3 normal, vec3 throughput, vec3 radiance,
                        uint material, uint inst, out vec4 data1, out vec4 data2, out vec4 data3, out vec4 data4) {
    data1.xyz = position;
    data2.xyz = throughput;
    data1.w = uintBitsToFloat(material | (inst << 16));

    int signs = 0;

    if (direction.y < 0.0) {
        signs |= 0x1;
    }

    if (normal.y < 0.0) {
        signs |= 0x2;
    }

    data2.w = float(signs);

    data3.xy = direction.xz;
    data3.zw = normal.xz;
    
    data4.rgb = radiance;
}

void pack_invalid_visible_point(vec3 radiance, out vec4 data1, out vec4 data2, out vec4 data3, out vec4 data4) {
    data1.xyz = vec3(0.0);
    data1.w = uintBitsToFloat(0xffffffffU);
    data2.xyz = vec3(0.0);
    data2.w = 0.0;
    data3 = vec4(0.0);
    data4.rgb = radiance;
}
