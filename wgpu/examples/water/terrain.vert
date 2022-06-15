#version 450

layout( binding = 0 ) uniform Globals {
    mat4x4 projection_view;
    vec4 clipping_plane;
} u_globals;

vec3 light = vec3(150.0, 70.0, 0.0);
vec3 light_colour = vec3(1.0, 0.98, 0.82);
float ambient = 0.2;

layout( location = 0 ) in vec3 position;
layout( location = 1 ) in vec3 normal;
layout( location = 2 ) in vec4 colour;

layout( location = 0 ) out vec4 frag_colour;
layout( location = 1 ) out float clip_dist;

void main() {
    gl_Position = u_globals.projection_view * vec4(position, 1.0);
    vec3 normalized_light_direction = normalize(position - light);
    float brightness_diffuse = clamp(dot(normalized_light_direction, normal), 0.2, 1.0);

    frag_colour = vec4(max((brightness_diffuse + ambient) * light_colour * colour.rgb, vec3(0.0, 0.0, 0.0)), colour.a);
    clip_dist = dot(vec4(position, 1.0), u_globals.clipping_plane);
}
