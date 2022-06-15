#version 450
#extension GL_GOOGLE_include_directive : require
#include "shadow_shared.frag"

void main() {
    vec3 normal = normalize(world_normal);
    // accumulate color
    vec3 color = c_ambient;
    for(uint i = 0; i < min(u_globals.num_lights.x, c_max_lights); i += 1) {
        Light light = s_lights[i];
        // project into the light space
        float shadow = fetch_shadow(i, light.proj * world_position);
        // compute Lambertian diffuse term
        vec3 light_dir = normalize(light.pos.xyz - world_position.xyz);
        float diffuse = max(0.0, dot(normal, light_dir));
        // add light contribution
        color += shadow * diffuse * light.color.xyz;
    }
    // multiply the light by material color
    out_color = vec4(color, 1.0) * u_entity.color;
}