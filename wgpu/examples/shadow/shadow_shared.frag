// for u_globals and u_entity, which are also needed by the fragment shader
#include "shadow_shared.vert"

struct Light {
    mat4x4 proj;
    vec4 pos;
    vec4 color;
};

layout(set = 0, binding = 1) readonly buffer LightBuffer {
    Light s_lights[];
};
layout(set = 0, binding = 1) uniform LightUniform {
    Light u_lights[10]; // Used when storage types are not supported
};
layout(set = 0, binding = 2) uniform texture2DArray t_shadow;
layout(set = 0, binding = 3) uniform samplerShadow sampler_shadow;

layout(location = 0) in vec3 world_normal;
layout(location = 1) in vec4 world_position;

layout(location = 0) out vec4 out_color;

vec3 c_ambient = vec3(0.05, 0.05, 0.05);
uint c_max_lights = 10;

float fetch_shadow(uint light_id, vec4 homogeneous_coords) {
    // compensate for the Y-flip difference between the NDC and texture coordinates
    vec2 flip_correction = vec2(0.5, -0.5);
    // compute texture coordinates for shadow lookup
    float proj_correction = 1.0 / homogeneous_coords.w;
    vec2 light_local = homogeneous_coords.xy * flip_correction * proj_correction + vec2(0.5, 0.5);
    // In the original WGSL code, this if statement was at the start of this function.
    // Unfortunately that doesn't work with GLSL uniform control flow requirements for 
    // implicit derivatives.
    vec2 dx = dFdx(light_local);
    vec2 dy = dFdx(light_local);
    if (homogeneous_coords.w <= 0.0) {
        return 1.0;
    }
    // do the lookup, using HW PCF and comparison
    vec4 shadow_coords = vec4(light_local, float(light_id), homogeneous_coords.z * proj_correction);
    return textureGrad(sampler2DArrayShadow(t_shadow, sampler_shadow), shadow_coords, dx, dy);
}
