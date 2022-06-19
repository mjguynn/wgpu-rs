layout(set = 0, binding = 0) uniform Globals {
    mat4x4 view_proj;
    uvec4 num_lights;
} u_globals;
layout(set = 1, binding = 0) uniform Entity {
    mat4x4 world;
    vec4 color;
} u_entity;
