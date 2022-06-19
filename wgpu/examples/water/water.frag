#version 450

#extension GL_GOOGLE_include_directive : require
#include "water_shared.glsl"

const vec3 water_colour = vec3(0.0, 0.46, 0.95);
const float zNear = 10.0;
const float zFar = 400.0;

layout( binding = 1 ) uniform texture2D reflection;
layout( binding = 2 ) uniform texture2D terrain_depth_tex;
layout( binding = 3 ) uniform sampler colour_sampler;
layout( binding = 4 ) uniform sampler depth_sampler;

layout( origin_upper_left ) in vec4 gl_FragCoord;
layout( location = 0 ) in vec2 f_WaterScreenPos;
layout( location = 1 ) in float f_Fresnel;
layout( location = 2 ) in vec3 f_Light;

layout( location = 0 ) out vec4 out_color;

float to_linear_depth(float depth) {
    float z_n = 2.0 * depth - 1.0;
    float z_e = 2.0 * zNear * zFar / (zFar + zNear - z_n * (zFar - zNear));
    return z_e;
}

void main() {
    vec3 reflection_colour = texture(sampler2D(reflection, colour_sampler), f_WaterScreenPos.xy).xyz;

    float pixel_depth = to_linear_depth(gl_FragCoord.z);
    vec2 normalized_coords = gl_FragCoord.xy / vec2(u_globals.time_size_width.w, u_globals.viewport_height);
    float terrain_depth = to_linear_depth(texture(sampler2D(terrain_depth_tex, depth_sampler), normalized_coords).r);

    float dist = terrain_depth - pixel_depth;
    float clamped = pow(smoothstep(0.0, 1.5, dist), 4.8);

    vec3 final_colour = f_Light + reflection_colour;
    float t = smoothstep(1.0, 5.0, dist) * 0.2; //TODO: splat for mix()?
    vec3 depth_colour = mix(final_colour, water_colour, vec3(t, t, t));

    out_color = vec4(depth_colour, clamped * (1.0 - f_Fresnel));
}