#version 450

#extension GL_GOOGLE_include_directive : require
#include "water_shared.glsl"

layout( location = 0 ) in ivec2 position;
layout( location = 1 ) in ivec4 offsets;

layout( location = 0 ) out vec2 f_WaterScreenPos;
layout( location = 1 ) out float f_Fresnel;
layout( location = 2 ) out vec3 f_Light;

const vec3 light_point = vec3(150.0, 70.0, 0.0);
const vec3 light_colour = vec3(1.0, 0.98, 0.82);

const float Y_SCL = 0.86602540378443864676372317075294;
const float CURVE_BIAS = -0.1;
const float INV_1_CURVE_BIAS = 1.11111111111; //1.0 / (1.0 + CURVE_BIAS);

//
// The following code to calculate simplex 3D
// is from https://github.com/ashima/webgl-noise

//
// Description : Array and textureless GLSL 2D/3D/4D simplex 
//               noise functions.
//      Author : Ian McEwan, Ashima Arts.
//  Maintainer : stegu
//     Lastmod : 20201014 (stegu)
//     License : Copyright (C) 2011 Ashima Arts. All rights reserved.
//               Distributed under the MIT License. See LICENSE file.
//               https://github.com/ashima/webgl-noise
//               https://github.com/stegu/webgl-noise
// 
vec3 mod289(vec3 x) {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

vec4 mod289(vec4 x) {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

vec4 permute(vec4 x) {
    // The "original" code on the web uses `+ 10.0`
    // while the WGSL source used `+ one`
    return mod289(((x*34.0) + 1.0)*x);
}

vec4 taylorInvSqrt(vec4 r) {
    return 1.79284291400159 - 0.85373472095314 * r;   
}

float snoise(vec3 v) { 
    const vec2 C = vec2(1.0/6.0, 1.0/3.0) ;
    const vec4 D = vec4(0.0, 0.5, 1.0, 2.0);

    // First corner
    vec3 i  = floor(v + dot(v, C.yyy) );
    vec3 x0 =   v - i + dot(i, C.xxx) ;

    // Other corners
    vec3 g = step(x0.yzx, x0.xyz);
    vec3 l = (1.0 - g).zxy;
    vec3 i1 = min( g, l );
    vec3 i2 = max( g, l );

    //   x0 = x0 - 0.0 + 0.0 * C.xxx;
    //   x1 = x0 - i1  + 1.0 * C.xxx;
    //   x2 = x0 - i2  + 2.0 * C.xxx;
    //   x3 = x0 - 1.0 + 3.0 * C.xxx;
    vec3 x1 = x0 - i1 + C.xxx;
    vec3 x2 = x0 - i2 + C.yyy; // 2.0*C.x = 1/3 = C.y
    vec3 x3 = x0 - D.yyy;      // -1.0+3.0*C.x = -0.5 = -D.y

    // Permutations
    i = mod289(i); 
    vec4 p = permute( permute( permute( 
                i.z + vec4(0.0, i1.z, i2.z, 1.0 ))
            + i.y + vec4(0.0, i1.y, i2.y, 1.0 )) 
            + i.x + vec4(0.0, i1.x, i2.x, 1.0 ));

    // Gradients: 7x7 points over a square, mapped onto an octahedron.
    // The ring size 17*17 = 289 is close to a multiple of 49 (49*6 = 294)
    float n_ = 0.142857142857; // 1.0/7.0
    vec3  ns = n_ * D.wyz - D.xzx;

    vec4 j = p - 49.0 * floor(p * ns.z * ns.z);  //  mod(p,7*7)

    vec4 x_ = floor(j * ns.z);
    vec4 y_ = floor(j - 7.0 * x_ );    // mod(j,N)

    vec4 x = x_ *ns.x + ns.yyyy;
    vec4 y = y_ *ns.x + ns.yyyy;
    vec4 h = 1.0 - abs(x) - abs(y);

    vec4 b0 = vec4( x.xy, y.xy );
    vec4 b1 = vec4( x.zw, y.zw );

    //vec4 s0 = vec4(lessThan(b0,0.0))*2.0 - 1.0;
    //vec4 s1 = vec4(lessThan(b1,0.0))*2.0 - 1.0;
    vec4 s0 = floor(b0)*2.0 + 1.0;
    vec4 s1 = floor(b1)*2.0 + 1.0;
    vec4 sh = -step(h, vec4(0.0));

    vec4 a0 = b0.xzyw + s0.xzyw*sh.xxyy ;
    vec4 a1 = b1.xzyw + s1.xzyw*sh.zzww ;

    vec3 p0 = vec3(a0.xy,h.x);
    vec3 p1 = vec3(a0.zw,h.y);
    vec3 p2 = vec3(a1.xy,h.z);
    vec3 p3 = vec3(a1.zw,h.w);

    //Normalise gradients
    vec4 norm = taylorInvSqrt(vec4(dot(p0,p0), dot(p1,p1), dot(p2, p2), dot(p3,p3)));
    p0 *= norm.x;
    p1 *= norm.y;
    p2 *= norm.z;
    p3 *= norm.w;

    // Mix final noise value
    // The "original" code on the web uses `0.5 *`
    // while the WGSL source used `0.6 *`
    vec4 m = max(0.6 - vec4(dot(x0,x0), dot(x1,x1), dot(x2,x2), dot(x3,x3)), 0.0);
    m *= m;
    // The "original" code on the web uses `105.0 *`
    // while the WGSL source used `9.0 *`
    return 9.0 * dot( m*m, vec4( dot(p0,x0), dot(p1,x1), 
                                dot(p2,x2), dot(p3,x3) ) );
}

// End of 3D simplex code.

vec3 apply_distortion(vec3 pos) {
    vec3 perlin_pos = pos;

    //Do noise transformation to permit for smooth,
    //continuous movement.

    //TODO: we should be able to name them `sin` and `cos`.
    float sn = u_globals.time_size_width.x;
    float cs = u_globals.time_size_width.y;
    float size = u_globals.time_size_width.z;

    // Rotate 90 Z, Move Left Size / 2
    perlin_pos = vec3(perlin_pos.y - perlin_pos.x - size, perlin_pos.x, perlin_pos.z);

    float xcos = perlin_pos.x * cs;
    float xsin = perlin_pos.x * sn;
    float ycos = perlin_pos.y * cs;
    float ysin = perlin_pos.y * sn;
    float zcos = perlin_pos.z * cs;
    float zsin = perlin_pos.z * sn;

    // Rotate Time Y
    vec3 perlin_pos_y = vec3(xcos + zsin, perlin_pos.y, -xsin + xcos);

    // Rotate Time Z
    vec3 perlin_pos_z = vec3(xcos - ysin, xsin + ycos, perlin_pos.x);

    // Rotate 90 Y
    perlin_pos = vec3(perlin_pos.z - perlin_pos.x, perlin_pos.y, perlin_pos.x);

    // Rotate Time X
    vec3 perlin_pos_x = vec3(perlin_pos.x, ycos - zsin, ysin + zcos);

    // Sample at different places for x/y/z to get random-looking water.
    return vec3(
        //TODO: use splats
        pos.x + snoise(perlin_pos_x + 2.0) * 0.4,
        pos.y + snoise(perlin_pos_y - 2.0) * 1.8,
        pos.z + snoise(perlin_pos_z) * 0.4
    );
}

// Multiply the input by the scale values.
vec4 make_position(vec2 original) {
    vec3 interpreted = vec3(original.x * 0.5, 0.0, original.y * Y_SCL);
    return vec4(apply_distortion(interpreted), 1.0);
}

// Create the normal, and apply the curve. Change the Curve Bias above.
vec3 make_normal(vec3 a, vec3 b, vec3 c) {
    vec3 norm = normalize(cross(b - c, a - c));
    vec3 center = (a + b + c) * (1.0 / 3.0); //TODO: use splat
    return (normalize(a - center) * CURVE_BIAS + norm) * INV_1_CURVE_BIAS;
}

// Calculate the fresnel effect.
float calc_fresnel(vec3 view, vec3 normal) {
    float refractive = abs(dot(view, normal));
    refractive = pow(refractive, 1.33333333333);
    return refractive;
}

// Calculate the specular lighting.
float calc_specular(vec3 eye, vec3 normal, vec3 light) {
    vec3 light_reflected = reflect(light, normal);
    float specular = max(dot(eye, light_reflected), 0.0);
    specular = pow(specular, 10.0);
    return specular;
}

void main() {
    vec2 p_pos = vec2(position);
    vec4 b_pos = make_position(p_pos + vec2(offsets.xy));
    vec4 c_pos = make_position(p_pos + vec2(offsets.zw));
    vec4 a_pos = make_position(p_pos);
    vec4 original_pos = vec4(p_pos.x * 0.5, 0.0, p_pos.y * Y_SCL, 1.0);

    mat4x4 vm = u_globals.view;
    vec4 transformed_pos = vm * a_pos;
    gl_Position = u_globals.projection * transformed_pos;

    //TODO: use vector splats for division
    vec3 water_pos = transformed_pos.xyz * (1.0 / transformed_pos.w);
    vec3 normal = make_normal((vm * a_pos).xyz, (vm * b_pos).xyz, (vm * c_pos).xyz);
    vec3 eye = normalize(-water_pos);
    vec4 transformed_light = vm * vec4(light_point, 1.0);

    f_Light = light_colour * calc_specular(eye, normal, normalize(water_pos.xyz - (transformed_light.xyz * (1.0 / transformed_light.w))));
    f_Fresnel = calc_fresnel(eye, normal);

    vec4 gridpos = u_globals.projection * vm * original_pos;
    f_WaterScreenPos = (0.5 * gridpos.xy * (1.0 / gridpos.w)) + vec2(0.5, 0.5);
}
