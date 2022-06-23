#[path = "../framework.rs"]
mod framework;

use rand::distributions::Uniform;
use rand::prelude::*;
use std::{
    borrow::Cow,
    cmp::{max, min},
    convert::TryInto,
    f32::consts,
    num::NonZeroU32,
};
use wgpu::util::DeviceExt;

const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const MIP_LEVEL_COUNT: u32 = 10;
const MIP_PASS_COUNT: u32 = MIP_LEVEL_COUNT - 1;

type RgbaU8 = [u8; 4];

trait ColorExt {
    fn from_u8(rgba: &RgbaU8) -> Self;
    fn to_srgb(&self) -> Self;
    fn to_linear(&self) -> Self;
}
impl ColorExt for wgpu::Color {
    fn from_u8(rgba: &RgbaU8) -> Self {
        fn cvt(i: u8) -> f64 {
            f64::from(i) / f64::from(u8::MAX)
        }
        Self {
            r: cvt(rgba[0]),
            g: cvt(rgba[1]),
            b: cvt(rgba[2]),
            a: cvt(rgba[3]),
        }
    }
    fn to_srgb(&self) -> Self {
        fn cvt(i: f64) -> f64 {
            if i <= 0.0031308 {
                12.92 * i
            } else {
                let ie = i.powf(1.0 / 2.4);
                1.055 * ie - 0.055
            }
        }
        Self {
            r: cvt(self.r),
            g: cvt(self.g),
            b: cvt(self.b),
            a: self.a,
        }
    }
    fn to_linear(&self) -> Self {
        fn cvt(i: f64) -> f64 {
            if i <= 0.04045 {
                i / 12.92
            } else {
                ((i + 0.055) / 1.055).powf(2.4)
            }
        }
        Self {
            r: cvt(self.r),
            g: cvt(self.g),
            b: cvt(self.b),
            a: self.a,
        }
    }
}

fn sample_color(rng: &mut ThreadRng, color_dist: &Uniform<u8>) -> RgbaU8 {
    color_dist
        .sample_iter(rng)
        .take(4)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}
struct RandomTexture {
    data: Box<[RgbaU8]>,
    extents: wgpu::Extent3d,
    rng: ThreadRng,
    color_dist: Uniform<u8>,
    coord_dist: Uniform<u32>,
}
impl RandomTexture {
    fn new(dim: u32) -> Self {
        let mut rng = thread_rng();
        let color_dist = Uniform::from(0..=u8::MAX);
        let initial_color = sample_color(&mut rng, &color_dist);
        let data = vec![initial_color; (dim * dim) as usize].into();
        let coord_dist = Uniform::from(0..dim);
        let extents = wgpu::Extent3d {
            width: dim,
            height: dim,
            depth_or_array_layers: 1,
        };
        Self {
            data,
            extents,
            rng,
            color_dist,
            coord_dist,
        }
    }
    // fills a random subrect with a random color
    fn randomize(&mut self) {
        let color = sample_color(&mut self.rng, &self.color_dist);
        let coords: Vec<_> = (&self.coord_dist)
            .sample_iter(&mut self.rng)
            .take(4)
            .collect();
        let (x0, y0) = (min(coords[0], coords[1]), min(coords[2], coords[3]));
        let (x1, y1) = (max(coords[0], coords[1]), max(coords[2], coords[3]));
        for x in x0..=x1 {
            for y in y0..=y1 {
                let i = y * self.extents.width + x;
                self.data[i as usize] = color;
            }
        }
    }
    fn extents(&self) -> wgpu::Extent3d {
        self.extents
    }
    // writes the toplevel mip to `texture`. doesn't touch lower level mips
    fn write(&self, queue: &wgpu::Queue, texture_handle: &wgpu::Texture) {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: texture_handle,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&self.data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::new(self.extents.width * 4).unwrap()),
                rows_per_image: None,
            },
            self.extents,
        )
    }
}

struct Example {
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
    draw_pipeline: wgpu::RenderPipeline,
    clear_color: wgpu::Color,
    random_texture: RandomTexture,
    random_texture_handle: wgpu::Texture,
}

impl Example {
    fn generate_matrix(aspect_ratio: f32) -> glam::Mat4 {
        let projection = glam::Mat4::perspective_rh(consts::FRAC_PI_4, aspect_ratio, 1.0, 1000.0);
        let view = glam::Mat4::look_at_rh(
            glam::Vec3::new(0f32, 0.0, 10.0),
            glam::Vec3::new(0f32, 50.0, 0.0),
            glam::Vec3::Z,
        );
        projection * view
    }

    fn generate_mipmaps(
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        texture: &wgpu::Texture,
        mip_count: u32,
    ) {
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("blit.wgsl"))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[TEXTURE_FORMAT.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let bind_group_layout = pipeline.get_bind_group_layout(0);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mip"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let views = (0..mip_count)
            .map(|mip| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("mip"),
                    format: None,
                    dimension: None,
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip,
                    mip_level_count: NonZeroU32::new(1),
                    base_array_layer: 0,
                    array_layer_count: None,
                })
            })
            .collect::<Vec<_>>();

        for target_mip in 1..mip_count as usize {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&views[target_mip - 1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: None,
            });

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &views[target_mip],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&pipeline);
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }
    }
}

impl framework::Example for Example {
    fn optional_features() -> wgpu::Features {
        wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::PIPELINE_STATISTICS_QUERY
    }

    fn init(
        config: &wgpu::SurfaceConfiguration,
        _adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        // Create the texture
        let size = 1 << MIP_PASS_COUNT;
        let mut random_texture = RandomTexture::new(size);
        let random_texture_handle = device.create_texture(&wgpu::TextureDescriptor {
            size: random_texture.extents(),
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            label: Some("random texture"),
        });
        let random_texture_view =
            random_texture_handle.create_view(&wgpu::TextureViewDescriptor::default());

        // Create other resources
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let mx_total = Self::generate_matrix(config.width as f32 / config.height as f32);
        let mx_ref: &[f32; 16] = mx_total.as_ref();
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(mx_ref),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create the render pipeline
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("draw.wgsl"))),
        });

        let draw_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("draw"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[config.format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Create bind group
        let bind_group_layout = draw_pipeline.get_bind_group_layout(0);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&random_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: None,
        });

        random_texture.randomize();
        random_texture.write(queue, &random_texture_handle);

        Example {
            bind_group,
            uniform_buf,
            draw_pipeline,
            clear_color: wgpu::Color::default(), // not correct, but doesn't impact perf
            random_texture,
            random_texture_handle,
        }
    }

    fn update(&mut self, _event: winit::event::WindowEvent) {
        //empty
    }

    fn resize(
        &mut self,
        config: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let mx_total = Self::generate_matrix(config.width as f32 / config.height as f32);
        let mx_ref: &[f32; 16] = mx_total.as_ref();
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(mx_ref));
    }

    fn render(
        &mut self,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _spawner: &framework::Spawner,
    ) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.draw_pipeline);
            rpass.set_bind_group(0, &self.bind_group, &[]);
            rpass.draw(0..4, 0..1);
        }

        self.random_texture.randomize();
        self.random_texture
            .write(queue, &self.random_texture_handle);

        self.clear_color = {
            let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("readback buffer"),
                size: 256,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            Self::generate_mipmaps(
                &mut encoder,
                device,
                &self.random_texture_handle,
                MIP_LEVEL_COUNT,
            );

            encoder.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: &self.random_texture_handle,
                    mip_level: MIP_PASS_COUNT,
                    origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &read_buf,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(NonZeroU32::new(256).unwrap()),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            // The key difference from dynamic-mipmap is that by restructuring the loop,
            // we only have one call to queue.submit().
            // This doesn't seem to impact performance.
            // However, adding a special queue submit for the texture write causes slowdowns
            queue.submit(Some(encoder.finish()));
            let read_slice = read_buf.slice(..);
            let read_future = read_slice.map_async(wgpu::MapMode::Read);
            device.poll(wgpu::Maintain::Wait);
            pollster::block_on(read_future).unwrap();
            let data = read_slice.get_mapped_range();
            wgpu::Color::from_u8(data[0..4].try_into().unwrap()).to_linear()
        };
    }
}

fn main() {
    framework::run::<Example>("mipmap");
}

#[test]
fn mipmap() {
    framework::test::<Example>(framework::FrameworkRefTest {
        image_path: "/examples/mipmap/screenshot.png",
        width: 1024,
        height: 768,
        optional_features: wgpu::Features::default(),
        base_test_parameters: framework::test_common::TestParameters::default()
            .backend_failure(wgpu::Backends::GL),
        tolerance: 50,
        max_outliers: 5000, // Mipmap sampling is highly variant between impls. This is currently bounded by lavapipe
    });
}
