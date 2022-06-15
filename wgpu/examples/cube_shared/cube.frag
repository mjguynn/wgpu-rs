#version 450
#extension GL_EXT_samplerless_texture_functions : require

layout(binding = 1) uniform utexture2D r_color;

layout(location = 0) in vec2 tex_coord;

layout(location = 0) out vec4 outColor;

void main(){
    vec4 tex = texelFetch(r_color, ivec2(256.0 * tex_coord), 0);
    float v = tex.x / 255.0;
    outColor = vec4(1.0 - (v * 5.0), 1.0 - (v * 15.0), 1.0 - (v * 50.0), 1.0);
}