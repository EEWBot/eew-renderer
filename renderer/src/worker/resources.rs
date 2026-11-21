use super::pipelines::{PositionVertex, SegmentInstance};
use renderer_assets::QueryInterface;
use renderer_types::InsetRegion;
use std::ops::Range;
use wgpu::util::DeviceExt;

pub struct Resources {
    pub map: Mesh,
    map_ranges: [Range<u32>; InsetRegion::COUNT],
    pub lake: Mesh,
    pub world: Mesh,

    /// 細分区域・都道府県の境界線 (全LODを連結したもの)
    pub border_segments: wgpu::Buffer,
    area_line_ranges: Vec<Range<u32>>,
    pref_line_ranges: Vec<Range<u32>>,

    pub tsunami_segments: wgpu::Buffer,
    tsunami_area_ranges: [Vec<(u16, Range<u32>)>; InsetRegion::COUNT],

    pub sampler: wgpu::Sampler,
    pub intensity_texture: wgpu::BindGroup,
    pub epicenter_texture: wgpu::BindGroup,
    pub overlay_texture: wgpu::BindGroup,
}

/// インデックス付きの三角形メッシュ。頂点は投影済み (メルカトル)。
pub struct Mesh {
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub index_count: u32,
}

impl Mesh {
    fn new(device: &wgpu::Device, label: &str, vertices: &[(f32, f32)], indices: &[u32]) -> Self {
        let vertices: Vec<_> = vertices
            .iter()
            .map(|&(x, y)| PositionVertex { position: [x, y] })
            .collect();

        Self {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
            index_count: indices.len() as u32,
        }
    }
}

impl Resources {
    pub fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let geometries = QueryInterface::geometries();
        let lake = QueryInterface::lake_geometries();
        let world = QueryInterface::world_geometries();
        let tsunami = QueryInterface::tsunami_geometries();

        let mut cursor = 0;
        let map_ranges = std::array::from_fn(|i| {
            let start = cursor;
            cursor += geometries.map_triangles[i].len() as u32;
            start..cursor
        });
        let map = Mesh::new(
            device,
            "map",
            geometries.vertices,
            &geometries.map_triangles.concat(),
        );

        let lake = Mesh::new(device, "lake", lake.vertices, lake.indices);
        let world = Mesh::new(device, "world", world.vertices, world.indices);

        let mut border_segments = Vec::new();
        let mut append_lods = |lods: &[&[renderer_assets::Segment]]| -> Vec<Range<u32>> {
            lods.iter()
                .map(|segments| {
                    let start = border_segments.len() as u32;
                    border_segments.extend(segments.iter().map(|&(x0, y0, x1, y1)| {
                        SegmentInstance {
                            segment: [x0, y0, x1, y1],
                        }
                    }));
                    start..border_segments.len() as u32
                })
                .collect()
        };
        let area_line_ranges = append_lods(geometries.area_lines);
        let pref_line_ranges = append_lods(geometries.pref_lines);

        let border_segments = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("border segments"),
            contents: bytemuck::cast_slice(&border_segments),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mut tsunami_instances = Vec::new();
        let tsunami_area_ranges = std::array::from_fn(|i| {
            let base = tsunami_instances.len() as u32;
            tsunami_instances.extend(tsunami.segments[i].iter().map(|&(x0, y0, x1, y1)| {
                SegmentInstance {
                    segment: [x0, y0, x1, y1],
                }
            }));
            tsunami.area_ranges[i]
                .iter()
                .map(|&(code, start, end)| (code, base + start..base + end))
                .collect()
        });
        let tsunami_segments = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tsunami segments"),
            contents: bytemuck::cast_slice(&tsunami_instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let load_png = |label, png: &[u8]| {
            let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)
                .unwrap()
                .into_rgba8();
            let (width, height) = image.dimensions();
            let mut pixels = image.into_raw();
            premultiply(&mut pixels);

            let texture = create_texture(device, label, width, height);
            write_texture(queue, &texture, 0, 0, width, height, &pixels);
            texture_bind_group(device, texture_layout, &texture, &sampler)
        };

        let intensity_texture = load_png(
            "intensity",
            include_bytes!("../../../assets/image/intensity.png"),
        );
        let epicenter_texture = load_png(
            "epicenter",
            include_bytes!("../../../assets/image/epicenter.png"),
        );
        let overlay_texture = load_png(
            "overlay",
            include_bytes!("../../../assets/image/overlay.png"),
        );

        Self {
            map,
            map_ranges,
            lake,
            world,
            border_segments,
            area_line_ranges,
            pref_line_ranges,
            tsunami_segments,
            tsunami_area_ranges,
            sampler,
            intensity_texture,
            epicenter_texture,
            overlay_texture,
        }
    }

    /// 地図ポリゴンのインデックス範囲。`None` なら全域。
    pub fn map_range(&self, region: Option<InsetRegion>) -> Range<u32> {
        match region {
            Some(region) => self.map_ranges[region.index()].clone(),
            None => 0..self.map.index_count,
        }
    }

    pub fn tsunami_area_ranges(&self, region: InsetRegion) -> &[(u16, Range<u32>)] {
        &self.tsunami_area_ranges[region.index()]
    }

    pub fn area_line_range(&self, scale: f32) -> Range<u32> {
        lod_range(&self.area_line_ranges, scale)
    }

    pub fn pref_line_range(&self, scale: f32) -> Range<u32> {
        lod_range(&self.pref_line_ranges, scale)
    }
}

/// 縮尺に合うLODの線分範囲。対応するLODがないほど縮小されている場合は最も粗いLODを使う。
fn lod_range(ranges: &[Range<u32>], scale: f32) -> Range<u32> {
    QueryInterface::query_lod_level_by_scale(scale)
        .and_then(|level| ranges.get(level))
        .or(ranges.last())
        .cloned()
        .unwrap_or(0..0)
}

#[inline(always)]
fn mul_alpha(c: u8, a: u16) -> u8 {
    let t = c as u16 * a + 128;
    ((t + (t >> 8)) >> 8) as u8
}

/// RGBA8 の RGB にアルファを掛ける。
fn premultiply(pixels: &mut [u8]) {
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let a = pixel[3] as u16;

        pixel[0] = mul_alpha(pixel[0], a);
        pixel[1] = mul_alpha(pixel[1], a);
        pixel[2] = mul_alpha(pixel[2], a);
    }
}

pub fn create_texture(
    device: &wgpu::Device,
    label: &str,
    width: u32,
    height: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

pub fn write_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    pixels: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

pub fn texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    texture: &wgpu::Texture,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&Default::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}
