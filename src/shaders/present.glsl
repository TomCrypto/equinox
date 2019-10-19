uniform sampler2D samples;

out vec4 color;

layout (std140) uniform Display {
    float exposure;
    float saturation;
    uint has_camera_response;
    vec4 camera_response[11];
} display;

vec3 LinearTosRGB(vec3 value) {
  return mix(value * 12.92,
             1.055 * pow(value, vec3(1.0 / 2.4)) - 0.055,
             vec3(greaterThan(value, vec3(0.00313066844250063))));
}

// sRGB => XYZ => D65_2_D60 => AP1 => RRT_SAT
const mat3 ACESInputMat =
mat3(
    vec3(0.59719, 0.35458, 0.04823),
    vec3(0.07600, 0.90834, 0.01566),
    vec3(0.02840, 0.13383, 0.83777)
);

// ODT_SAT => XYZ => D60_2_D65 => sRGB
const mat3 ACESOutputMat =
mat3(
    vec3( 1.60475, -0.53108, -0.07367),
    vec3(-0.10208,  1.10813, -0.00605),
    vec3(-0.00327, -0.07276,  1.07602)
);

vec3 RRTAndODTFit(vec3 v)
{
    vec3 a = v * (v + 0.0245786) - 0.000090537;
    vec3 b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
}

vec3 ACESFitted(vec3 color)
{
    color = color * ACESInputMat;

    // Apply RRT and ODT
    color = RRTAndODTFit(color);

    color = color * ACESOutputMat;

    // Clamp to [0, 1]
    color = clamp(color, 0.0, 1.0);

    return color;
}

vec3 camera_response(uint curve, vec3 x) {
    if (curve == 0xffffffffU) {
        return x; // no curve
    }

    vec3 xi = vec3(1.0);
    vec3 yi = vec3(0.0);

    for (uint i = 0U; i < 11U; ++i) {
        yi += xi * display.camera_response[i].rgb;
        xi *= x;
    }

    return yi;
}

void main() {
    vec4 value = texelFetch(samples, ivec2(gl_FragCoord.xy - 0.5), 0);

    // For debugging purposes only

    if (any(isnan(value.xyz))) {
        color = vec4(1.0, 0.0, 1.0, 1.0);
        return;
    }

    if (any(isinf(value.xyz))) {
        color = vec4(0.0, 0.0, 1.0, 1.0);
        return;
    }

    vec3 tone_mapped = ACESFitted(value.rgb * display.exposure);

    if (display.has_camera_response != 0U) {
        tone_mapped = camera_response(0U, tone_mapped);
    }
    
    if (display.saturation != 1.0) {
        float luminance = sqrt(dot(tone_mapped, tone_mapped * vec3(0.299, 0.587, 0.114)));
        tone_mapped = luminance + (tone_mapped - luminance) * display.saturation;
    }

    color = vec4(LinearTosRGB(tone_mapped), 1.0);
}
