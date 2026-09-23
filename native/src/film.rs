use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use iced::widget::shader::{self, Viewport};
use iced::{mouse, wgpu, Element, Length, Rectangle};

const FORGOTTEN_AFTER: u64 = 120;

static NEXT: AtomicU64 = AtomicU64::new(1);

fn next() -> u64 {
    NEXT.fetch_add(1, Ordering::Relaxed)
}

pub fn reel() -> u64 {
    next()
}

#[derive(Clone)]
pub struct Frame {
    reel: u64,
    number: u64,
    width: u32,
    height: u32,
    pixels: Arc<Vec<u8>>,
}

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Frame({} of reel {}, {}×{})", self.number, self.reel, self.width, self.height)
    }
}

impl Frame {
    pub fn new(reel: u64, width: u32, height: u32, pixels: Vec<u8>) -> Self {
        debug_assert_eq!(pixels.len(), (width * height * 4) as usize);
        Frame { reel, number: next(), width, height, pixels: Arc::new(pixels) }
    }

    pub fn still(&self) -> iced::widget::image::Handle {
        iced::widget::image::Handle::from_rgba(self.width, self.height, self.pixels.as_ref().clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fit {
    Cover,
    Contain,
}

pub fn show<'a, Message: 'a>(frame: &Frame, fit: Fit, opacity: f32) -> Element<'a, Message> {
    shader::Shader::new(Showing { frame: frame.clone(), fit, opacity }).width(Length::Fill).height(Length::Fill).into()
}

struct Showing {
    frame: Frame,
    fit: Fit,
    opacity: f32,
}

impl<Message> shader::Program<Message> for Showing {
    type State = ();
    type Primitive = Shown;

    fn draw(&self, _state: &(), _cursor: mouse::Cursor, _bounds: Rectangle) -> Shown {
        Shown { frame: self.frame.clone(), fit: self.fit, opacity: self.opacity.clamp(0.0, 1.0) }
    }
}

#[derive(Debug)]
pub struct Shown {
    frame: Frame,
    fit: Fit,
    opacity: f32,
}

fn placed(fit: Fit, bounds: (f32, f32), image: (f32, f32)) -> ([f32; 4], [f32; 4]) {
    let (bw, bh) = (bounds.0.max(1.0), bounds.1.max(1.0));
    let (iw, ih) = (image.0.max(1.0), image.1.max(1.0));
    match fit {
        Fit::Cover => {
            let scale = (bw / iw).max(bh / ih);
            let (seen_x, seen_y) = ((bw / (iw * scale)).min(1.0), (bh / (ih * scale)).min(1.0));
            ([0.0, 0.0, 1.0, 1.0], [(1.0 - seen_x) / 2.0, (1.0 - seen_y) / 2.0, seen_x, seen_y])
        }
        Fit::Contain => {
            let scale = (bw / iw).min(bh / ih);
            let (wide, high) = ((iw * scale / bw).min(1.0), (ih * scale / bh).min(1.0));
            ([(1.0 - wide) / 2.0, (1.0 - high) / 2.0, wide, high], [0.0, 0.0, 1.0, 1.0])
        }
    }
}

impl shader::Primitive for Shown {
    type Pipeline = Projector;

    fn prepare(&self, pipeline: &mut Projector, device: &wgpu::Device, queue: &wgpu::Queue, bounds: &Rectangle, _viewport: &Viewport) {
        let (dest, uv) = placed(self.fit, (bounds.width, bounds.height), (self.frame.width as f32, self.frame.height as f32));
        pipeline.load(device, queue, &self.frame);
        pipeline.place(queue, self.frame.reel, dest, uv, self.opacity);
    }

    fn draw(&self, pipeline: &Projector, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        let Some(reel) = pipeline.reels.get(&self.frame.reel) else {
            return true;
        };
        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_bind_group(0, &reel.bind_group, &[]);
        render_pass.draw(0..4, 0..1);
        true
    }
}

struct Reel {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    place: wgpu::Buffer,
    size: (u32, u32),
    number: u64,
    seen: u64,
}

pub struct Projector {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    texels: wgpu::TextureFormat,
    reels: HashMap<u64, Reel>,
    tick: u64,
}

const SHADER: &str = "
struct Place {
    dest: vec4<f32>,
    uv: vec4<f32>,
    look: vec4<f32>,
};

@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var soft: sampler;
@group(0) @binding(2) var<uniform> place: Place;

struct Out {
    @builtin(position) at: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> Out {
    let corner = vec2<f32>(f32(i & 1u), f32((i >> 1u) & 1u));
    let spot = place.dest.xy + corner * place.dest.zw;
    var out: Out;
    out.at = vec4<f32>(spot.x * 2.0 - 1.0, 1.0 - spot.y * 2.0, 0.0, 1.0);
    out.uv = place.uv.xy + corner * place.uv.zw;
    return out;
}

@fragment
fn fs(input: Out) -> @location(0) vec4<f32> {
    let colour = textureSample(picture, soft, input.uv);
    return vec4<f32>(colour.rgb, colour.a * place.look.x);
}
";

impl shader::Pipeline for Projector {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("dossier film"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER)),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dossier film"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("dossier film"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("dossier film"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("dossier film"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let texels = if format.is_srgb() { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm };
        Projector { pipeline, layout, sampler, texels, reels: HashMap::new(), tick: 0 }
    }

    fn trim(&mut self) {
        self.tick += 1;
        let tick = self.tick;
        self.reels.retain(|_, reel| tick.saturating_sub(reel.seen) < FORGOTTEN_AFTER);
    }
}

impl Projector {
    fn load(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, frame: &Frame) {
        let size = (frame.width.max(1), frame.height.max(1));
        let fits = self.reels.get(&frame.reel).is_some_and(|reel| reel.size == size);
        if !fits {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("dossier film"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.texels,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let place = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("dossier film"),
                size: std::mem::size_of::<[f32; 12]>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("dossier film"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: place.as_entire_binding() },
                ],
            });
            self.reels.insert(frame.reel, Reel { texture, bind_group, place, size, number: 0, seen: self.tick });
        }
        let Some(reel) = self.reels.get_mut(&frame.reel) else {
            return;
        };
        reel.seen = self.tick;
        if reel.number == frame.number {
            return;
        }
        reel.number = frame.number;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &reel.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &frame.pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * size.0), rows_per_image: Some(size.1) },
            wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
        );
    }

    fn place(&mut self, queue: &wgpu::Queue, reel: u64, dest: [f32; 4], uv: [f32; 4], opacity: f32) {
        let Some(reel) = self.reels.get(&reel) else {
            return;
        };
        let values: [f32; 12] = [dest[0], dest[1], dest[2], dest[3], uv[0], uv[1], uv[2], uv[3], opacity, 0.0, 0.0, 0.0];
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        queue.write_buffer(&reel.place, 0, &bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shader_is_wgsl_the_device_accepts() {
        let module = naga::front::wgsl::parse_str(SHADER).expect("the shader parses");
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
            .validate(&module)
            .expect("the shader validates");
    }

    #[test]
    fn a_cover_fills_the_box_and_crops_the_long_side_evenly() {
        let (dest, uv) = placed(Fit::Cover, (1000.0, 1000.0), (960.0, 540.0));
        assert_eq!(dest, [0.0, 0.0, 1.0, 1.0]);
        assert!((uv[3] - 1.0).abs() < 1e-6, "the short side is shown whole");
        assert!((uv[2] - 0.5625).abs() < 1e-4, "a square shows 540/960 of a wide frame's width");
        assert!((uv[0] - (1.0 - uv[2]) / 2.0).abs() < 1e-6, "the crop is centred");
    }

    #[test]
    fn a_contained_frame_shows_whole_and_centred() {
        let (dest, uv) = placed(Fit::Contain, (1000.0, 1000.0), (960.0, 540.0));
        assert_eq!(uv, [0.0, 0.0, 1.0, 1.0]);
        assert!((dest[2] - 1.0).abs() < 1e-6);
        assert!((dest[3] - 0.5625).abs() < 1e-4);
        assert!((dest[1] - (1.0 - dest[3]) / 2.0).abs() < 1e-6);
    }

    #[test]
    fn every_frame_is_its_own_and_a_reel_keeps_its_name() {
        let reel = reel();
        let a = Frame::new(reel, 2, 1, vec![0; 8]);
        let b = Frame::new(reel, 2, 1, vec![255; 8]);
        assert_ne!(a.number, b.number);
        assert_eq!(a.reel, b.reel);
        assert_ne!(reel, super::reel());
    }
}
