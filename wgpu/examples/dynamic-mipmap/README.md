# dynamic-mipmap

This example demonstrates "back-and-forth" data transfers between the CPU and GPU in a
semi-plausible manner. Each frame, a texture is dynamically generated on the CPU; the texture
is then sent to the GPU, where mipmaps are generated; the lowest mipmap level is then sent to
the CPU and used to derive the clear color; finally, the GPU renders a plane using the mipmapped
texture.

## To Run

```
cargo run --example dynamic-mipmap
```
