use super::image_buffer::RGBAImageData;
use renderer_types::Size;

pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub const STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Stencil8;
pub const SAMPLE_COUNT: u32 = 4;

const BYTES_PER_PIXEL: u32 = 4;

pub struct RenderTarget {
    size: Size<u32>,
    msaa_color: wgpu::TextureView,
    msaa_stencil: wgpu::TextureView,
    resolve: wgpu::Texture,
    resolve_view: wgpu::TextureView,
    readback: wgpu::Buffer,
    padded_bytes_per_row: u32,
}

impl RenderTarget {
    pub fn new(device: &wgpu::Device, size: Size<u32>) -> Self {
        let extent = wgpu::Extent3d {
            width: size.x(),
            height: size.y(),
            depth_or_array_layers: 1,
        };

        let create_texture = |label, format, sample_count, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: extent,
                mip_level_count: 1,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };

        let msaa_color = create_texture(
            "msaa color",
            COLOR_FORMAT,
            SAMPLE_COUNT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .create_view(&Default::default());

        let msaa_stencil = create_texture(
            "msaa stencil",
            STENCIL_FORMAT,
            SAMPLE_COUNT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .create_view(&Default::default());

        let resolve = create_texture(
            "resolve",
            COLOR_FORMAT,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let resolve_view = resolve.create_view(&Default::default());

        // テクスチャ→バッファのコピーは行のバイト数がアライメントの倍数である必要がある
        let padded_bytes_per_row =
            (size.x() * BYTES_PER_PIXEL).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);

        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded_bytes_per_row * size.y()) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            size,
            msaa_color,
            msaa_stencil,
            resolve,
            resolve_view,
            readback,
            padded_bytes_per_row,
        }
    }

    /// 色とステンシルをクリアして描画パスを開始する。
    /// `clear_color` はガンマ空間の値で、そのままターゲットに書き込まれる。
    pub fn begin_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        clear_color: [f32; 4],
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("frame"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.msaa_color,
                depth_slice: None,
                resolve_target: Some(&self.resolve_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: clear_color[0] as f64,
                        g: clear_color[1] as f64,
                        b: clear_color[2] as f64,
                        a: clear_color[3] as f64,
                    }),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.msaa_stencil,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Discard,
                }),
            }),
            ..Default::default()
        })
    }

    /// resolve済みの画像を読み戻し用バッファへコピーするコマンドを積む。
    pub fn copy_to_readback(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.resolve,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_bytes_per_row),
                    rows_per_image: Some(self.size.y()),
                },
            },
            wgpu::Extent3d {
                width: self.size.x(),
                height: self.size.y(),
                depth_or_array_layers: 1,
            },
        );
    }

    /// 読み戻し用バッファの内容を取り出す。GPUの完了を待つのでブロックする。
    pub fn read(&self, device: &wgpu::Device) -> Result<RGBAImageData, ReadError> {
        let slice = self.readback.slice(..);

        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv().map_err(|_| ReadError::MapCallbackDropped)??;

        let bytes_per_row = (self.size.x() * BYTES_PER_PIXEL) as usize;

        let data = {
            let mapped = slice.get_mapped_range()?;
            if self.padded_bytes_per_row as usize == bytes_per_row {
                mapped.to_vec()
            } else {
                let mut data = Vec::with_capacity(bytes_per_row * self.size.y() as usize);
                for row in mapped.chunks_exact(self.padded_bytes_per_row as usize) {
                    data.extend_from_slice(&row[..bytes_per_row]);
                }
                data
            }
        };

        self.readback.unmap();

        Ok(RGBAImageData {
            data,
            width: self.size.x(),
            height: self.size.y(),
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ReadError {
    #[error("failed to wait for GPU completion: {0}")]
    Poll(#[from] wgpu::PollError),

    #[error("readback buffer map callback was dropped")]
    MapCallbackDropped,

    #[error("failed to map readback buffer: {0}")]
    Map(#[from] wgpu::BufferAsyncError),

    #[error("failed to access readback buffer: {0}")]
    MapRange(#[from] wgpu::MapRangeError),
}
