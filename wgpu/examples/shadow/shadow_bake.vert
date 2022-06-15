#version 450
#extension GL_GOOGLE_include_directive : require
#include "shadow_shared.vert"

layout(location = 0) in ivec4 position;

void main() {
    gl_Position = u_globals.view_proj * u_entity.world * vec4(position);
}