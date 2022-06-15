[[vk::binding(1)]] Texture2D<uint> r_color;

float4 fs_main([[vk::location(0)]] float2 tex_coord) {
    int2 pixel_coords = int2(256.0 * tex_coord);
    float4 tex = r_color.Load(int3(pixel_coords, 0)); // 0 LOD
    float v = tex.x / 255.0;
    return float4(1.0 - (v * 5.0), 1.0 - (v * 15.0), 1.0 - (v * 50.0), 1.0);
}