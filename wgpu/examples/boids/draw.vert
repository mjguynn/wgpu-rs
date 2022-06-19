#version 450

layout( location = 0 ) in vec2 particle_pos;
layout( location = 1 ) in vec2 particle_vel;
layout( location = 2 ) in vec2 position;

void main() {
    float angle = -atan(particle_vel.x, particle_vel.y);
    vec2 pos = vec2(
        position.x * cos(angle) - position.y * sin(angle),
        position.x * sin(angle) + position.y * cos(angle)
    );
    gl_Position = vec4(pos + particle_pos, 0.0, 1.0);
}