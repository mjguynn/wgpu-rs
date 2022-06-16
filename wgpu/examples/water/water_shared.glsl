layout( binding = 0 ) uniform Globals {
    mat4x4 view;
    mat4x4 projection;
    vec4 time_size_width;
    float viewport_height;
} u_globals;