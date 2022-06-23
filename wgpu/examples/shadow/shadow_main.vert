#version 450
#extension GL_GOOGLE_include_directive : require
#include "shadow_shared.vert"

layout(location = 0) in ivec4 position;
layout(location = 1) in ivec4 normal;

layout(location = 0) out vec3 world_normal;
layout(location = 1) out vec4 world_position;

void main() {
    mat4x4 w = u_entity.world;
    vec4 world_pos = w * position;
    gl_Position = u_globals.view_proj * world_pos;
    world_normal = mat3x3(w[0].xyz, w[1].xyz, w[2].xyz) * vec3(normal.xyz);
    world_position = world_pos;
}