#version 450

// I think this is equivalent to @early_depth_test in WGSL
layout( early_fragment_tests ) in; 

layout( location = 0 ) in vec4 frag_colour;
layout( location = 1 ) in float clip_dist;

layout( location = 0 ) out vec4 out_colour;

void main() {
    if(clip_dist < 0.0) {
        discard;
    }
    out_colour = vec4(frag_colour.xyz, 1.0);
}