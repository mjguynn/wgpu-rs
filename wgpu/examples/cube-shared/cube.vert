#version 450

layout(binding = 0) uniform Locals {
    mat4x4 transform;
} r_locals;

layout(location = 0) in vec4 position;
layout(location = 1) in vec2 tex_coord;

layout(location = 0) out vec2 frag_tex_coord;

void main() {
    frag_tex_coord = tex_coord;
    gl_Position = r_locals.transform * position;
}