use super::target::{COLOR_FORMAT, SAMPLE_COUNT, STENCIL_FORMAT};
use std::num::NonZeroU64;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawUniform {
    pub offset: [f32; 2],
    pub scale: [f32; 2],
    pub viewport: [f32; 2],
    pub param: [f32; 2],
    pub color: [f32; 4],
}

pub const TSUNAMI_LEVEL_CAPACITY: usize = 128;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TsunamiUniform {
    pub colors: [[f32; 4]; 5],
    pub levels: [u32; TSUNAMI_LEVEL_CAPACITY],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PositionVertex {
    pub position: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SegmentInstance {
    pub segment: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TsunamiSegmentInstance {
    pub segment: [f32; 4],
    pub code: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    pub center: [f32; 2],
    pub uv_rect: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TexturedVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}

const CAP_SLICES: u32 = 4;

pub const SEGMENT_VERTEX_COUNT: u32 = 6 + 2 * 3 * CAP_SLICES;

const SHADER_CONSTANTS: &[(&str, f64)] = &[("CAP_SLICES", CAP_SLICES as f64)];

pub struct Pipelines {
    pub draw_layout: wgpu::BindGroupLayout,
    pub texture_layout: wgpu::BindGroupLayout,
    pub tsunami_layout: wgpu::BindGroupLayout,

    pub fill: wgpu::RenderPipeline,
    pub mask: wgpu::RenderPipeline,
    pub line: wgpu::RenderPipeline,
    pub tsunami_line: wgpu::RenderPipeline,
    pub sprite: wgpu::RenderPipeline,
    pub textured: wgpu::RenderPipeline,
}

impl Pipelines {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("renderer"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../../assets/shader/renderer.wgsl").into(),
            ),
        });

        let draw_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("draw"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: NonZeroU64::new(size_of::<DrawUniform>() as u64),
                },
                count: None,
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture"),
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
            ],
        });

        let tsunami_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tsunami"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(size_of::<TsunamiUniform>() as u64),
                },
                count: None,
            }],
        });

        let builder = PipelineBuilder {
            device,
            shader: &shader,
            draw_only: device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("draw"),
                bind_group_layouts: &[Some(&draw_layout)],
                ..Default::default()
            }),
            with_texture: device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("draw + texture"),
                bind_group_layouts: &[Some(&draw_layout), Some(&texture_layout)],
                ..Default::default()
            }),
            with_tsunami: device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("draw + tsunami"),
                bind_group_layouts: &[Some(&draw_layout), Some(&tsunami_layout)],
                ..Default::default()
            }),
        };

        let position_vertex = wgpu::VertexBufferLayout {
            array_stride: size_of::<PositionVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2],
        };

        let fill = builder.build(
            "fill",
            &builder.draw_only,
            "vs_fill",
            "fs_fill",
            position_vertex.clone(),
            false,
        );

        let mask = builder.build(
            "mask",
            &builder.draw_only,
            "vs_fill",
            "fs_fill",
            position_vertex,
            true,
        );

        let line = builder.build(
            "line",
            &builder.draw_only,
            "vs_line",
            "fs_fill",
            wgpu::VertexBufferLayout {
                array_stride: size_of::<SegmentInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![0 => Float32x4],
            },
            false,
        );

        let tsunami_line = builder.build(
            "tsunami line",
            &builder.with_tsunami,
            "vs_tsunami_line",
            "fs_vertex_color",
            wgpu::VertexBufferLayout {
                array_stride: size_of::<TsunamiSegmentInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Uint32],
            },
            false,
        );

        let sprite = builder.build(
            "sprite",
            &builder.with_texture,
            "vs_sprite",
            "fs_textured",
            wgpu::VertexBufferLayout {
                array_stride: size_of::<SpriteInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
            },
            false,
        );

        let textured = builder.build(
            "textured",
            &builder.with_texture,
            "vs_textured",
            "fs_textured",
            wgpu::VertexBufferLayout {
                array_stride: size_of::<TexturedVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
            },
            false,
        );

        Self {
            draw_layout,
            texture_layout,
            tsunami_layout,
            fill,
            mask,
            line,
            tsunami_line,
            sprite,
            textured,
        }
    }
}

struct PipelineBuilder<'a> {
    device: &'a wgpu::Device,
    shader: &'a wgpu::ShaderModule,
    draw_only: wgpu::PipelineLayout,
    with_texture: wgpu::PipelineLayout,
    with_tsunami: wgpu::PipelineLayout,
}

impl PipelineBuilder<'_> {
    fn build(
        &self,
        label: &str,
        layout: &wgpu::PipelineLayout,
        vertex_entry: &str,
        fragment_entry: &str,
        vertex_buffer: wgpu::VertexBufferLayout<'_>,
        writes_stencil: bool,
    ) -> wgpu::RenderPipeline {
        let face = if writes_stencil {
            wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::Always,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Replace,
            }
        } else {
            wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::Equal,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Keep,
            }
        };

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: self.shader,
                    entry_point: Some(vertex_entry),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: SHADER_CONSTANTS,
                        ..Default::default()
                    },
                    buffers: &[Some(vertex_buffer)],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: STENCIL_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil: wgpu::StencilState {
                        front: face,
                        back: face,
                        read_mask: 0xff,
                        write_mask: if writes_stencil { 0xff } else { 0x00 },
                    },
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: SAMPLE_COUNT,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: self.shader,
                    entry_point: Some(fragment_entry),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: SHADER_CONSTANTS,
                        ..Default::default()
                    },
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOR_FORMAT,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: if writes_stencil {
                            wgpu::ColorWrites::empty()
                        } else {
                            wgpu::ColorWrites::ALL
                        },
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
    }
}
