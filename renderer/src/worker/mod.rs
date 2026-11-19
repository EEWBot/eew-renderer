use crate::frame_context::{
    FramePayload, HasEpicenter, HasTime, HasTsunamiForecastLevels, MapLayerConfig,
};
use crate::model::{Message, RenderingError};
use crate::worker::fonts::{Fonts, GlyphCache, GLYPH_ATLAS_SIZE};
use crate::worker::frame::{
    DynamicBuffers, Frame, FrameData, FrameError, StagedSprites, View, UNIFORM_CAPACITY,
    VERTEX_CAPACITY,
};
use crate::worker::gpu::Gpu;
use crate::worker::pipelines::{DrawUniform, Pipelines, TsunamiUniform};
use crate::worker::resources::Resources;
use crate::worker::target::RenderTarget;
use image_buffer::RGBAImageData;
use renderer_types::*;
use std::error::Error;
use std::num::NonZeroU64;
use std::time::Instant;
use tokio::sync::mpsc;

mod camera;
mod drawer_epicenter;
mod drawer_inset_frame;
mod drawer_intensity_icon;
mod drawer_map;
mod drawer_overlay;
mod drawer_tsunami_legends;
mod drawer_tsunami_line;
mod fonts;
mod frame;
pub mod gpu;
pub mod image_buffer;
mod inset;
mod notch;
mod pipelines;
mod resources;
mod target;
mod theme;

const DIMENSION: (u32, u32) = (1024, 768);
const MAP_MARGIN_FACTOR: f32 = 1.2;
const ICON_RATIO_IN_Y_AXIS: f32 = 0.05;

pub async fn run(
    mut rx: mpsc::Receiver<Message>,
    gpu_options: gpu::GpuOptions,
) -> Result<(), Box<dyn Error>> {
    let mut renderer = Renderer::new(Gpu::new(&gpu_options).await?);

    while let Some(message) = rx.recv().await {
        let Message::FrameRequest((frame_context, response_socket)) = message;

        let result = match renderer.render(&frame_context) {
            Ok(image) => Ok(image),
            Err(RenderError::Request(e)) => {
                tracing::warn!(
                    "Rejected a request: {e} ({})",
                    frame_context.request_identity
                );
                Err(e)
            }
            Err(RenderError::Readback(e)) => return Err(e.into()),
        };

        let _ = response_socket.send(result);
    }

    // rx.recv() failing directly indicates that the Web thread has terminated abnormally,
    // and the worker thread should keep waiting for the program’s error-handling routine,
    // which is the correct behavior.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    Err("An abnormal termination of the web thread was detected!".into())
}

#[derive(thiserror::Error, Debug)]
enum RenderError {
    /// リクエストの内容が原因で描画できなかった。レンダラは使い続けられる。
    #[error(transparent)]
    Request(#[from] RenderingError),
    /// GPUからの読み戻しに失敗した。回復できない。
    #[error(transparent)]
    Readback(#[from] target::ReadError),
}

struct Renderer {
    gpu: Gpu,
    target: RenderTarget,
    pipelines: Pipelines,
    resources: Resources,
    dynamic: DynamicBuffers,
    fonts: Fonts,
    glyphs: GlyphCache,
}

impl Renderer {
    fn new(gpu: Gpu) -> Self {
        let device = &gpu.device;

        let target = RenderTarget::new(device, Size::from(DIMENSION));
        let pipelines = Pipelines::new(device);
        let resources = Resources::load(device, &gpu.queue, &pipelines.texture_layout);
        let dynamic = create_dynamic_buffers(device, &pipelines, &resources);

        Self {
            gpu,
            target,
            pipelines,
            resources,
            dynamic,
            fonts: Fonts::new(),
            glyphs: GlyphCache::new(),
        }
    }

    fn render(
        &mut self,
        frame_context: &crate::frame_context::FrameContext,
    ) -> Result<RGBAImageData, RenderError> {
        let start_at = Instant::now();

        let theme = &theme::DEFAULT;
        let payload = &frame_context.payload;

        let bounding_box = calculate_bounding_box(payload);
        let camera = camera::Camera::fit(&bounding_box, Size::from(DIMENSION))
            .with_margin(MAP_MARGIN_FACTOR);

        let had_glyphs = !self.glyphs.is_empty();
        let (mut encoder, frame_data) = match self.record(theme, payload, &camera) {
            Err(FrameError::GlyphAtlasFull) if had_glyphs => {
                self.glyphs.clear();
                self.record(theme, payload, &camera)
            }
            result => result,
        }
        .map_err(RenderingError::from)?;

        let queue = &self.gpu.queue;

        // 動的なデータは、描画命令を送信する前に書き込んでおけば反映される
        queue.write_buffer(&self.dynamic.uniforms, 0, &frame_data.uniforms);
        queue.write_buffer(&self.dynamic.vertices, 0, &frame_data.vertices);
        if let Some((y, rows, pixels)) = self.glyphs.take_dirty() {
            resources::write_texture(
                queue,
                &self.dynamic.glyph_texture,
                0,
                y,
                GLYPH_ATLAS_SIZE,
                rows,
                pixels,
            );
        }

        self.target.copy_to_readback(&mut encoder);
        queue.submit([encoder.finish()]);

        let t_before_bufcpy = Instant::now();

        let image = self.target.read(&self.gpu.device)?;

        let t_done = Instant::now();

        // GPUでの描画の完了待ちは BufCpy 側に含まれる
        tracing::info!(
            "Render: {:?} BufCpy: {:?} ({})",
            t_before_bufcpy - start_at,
            t_done - t_before_bufcpy,
            frame_context.request_identity,
        );

        Ok(image)
    }

    fn record(
        &mut self,
        theme: &theme::Theme,
        payload: &FramePayload,
        camera: &camera::Camera,
    ) -> Result<(wgpu::CommandEncoder, FrameData), FrameError> {
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });

        let frame_data = {
            let pass = self.target.begin_pass(&mut encoder, theme.clear_color);
            let mut frame = Frame::new(
                &self.resources,
                &self.pipelines,
                &self.dynamic,
                &self.fonts,
                &mut self.glyphs,
                theme,
                pass,
                View::from_camera(camera),
            );

            let map_layers = payload.map_layers();

            match payload {
                FramePayload::Earthquake(earthquake) => {
                    drawer_map::draw(&mut frame, map_layers, None);
                    drawer_intensity_icon::draw_all(&mut frame, earthquake);
                    let epicenters = drawer_epicenter::prepare(&mut frame, earthquake.epicenter());
                    drawer_epicenter::draw(&mut frame, epicenters.as_ref(), ICON_RATIO_IN_Y_AXIS);
                    drawer_overlay::draw(&mut frame, earthquake);
                }
                FramePayload::TsunamiFirst(tsunami) => {
                    let levels = drawer_tsunami_line::area_levels(tsunami);
                    self.gpu.queue.write_buffer(
                        &self.dynamic.tsunami,
                        0,
                        bytemuck::bytes_of(&drawer_tsunami_line::build_uniform(theme, &levels)),
                    );
                    draw_tsunami_frame(&mut frame, tsunami, map_layers, Some(&levels));
                }
                FramePayload::TsunamiSecond(tsunami) => {
                    draw_tsunami_frame(&mut frame, tsunami, map_layers, None);
                }
            }

            frame.finish()?
        };

        Ok((encoder, frame_data))
    }
}

fn create_dynamic_buffers(
    device: &wgpu::Device,
    pipelines: &Pipelines,
    resources: &Resources,
) -> DynamicBuffers {
    let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("draw uniforms"),
        size: UNIFORM_CAPACITY as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let draw_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("draw"),
        layout: &pipelines.draw_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &uniforms,
                offset: 0,
                size: NonZeroU64::new(size_of::<DrawUniform>() as u64),
            }),
        }],
    });

    let vertices = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dynamic vertices"),
        size: VERTEX_CAPACITY as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let tsunami = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("tsunami"),
        size: size_of::<TsunamiUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let tsunami_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("tsunami"),
        layout: &pipelines.tsunami_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 2,
            resource: tsunami.as_entire_binding(),
        }],
    });

    let glyph_texture =
        resources::create_texture(device, "glyphs", GLYPH_ATLAS_SIZE, GLYPH_ATLAS_SIZE);
    let glyph_bind_group = resources::texture_bind_group(
        device,
        &pipelines.texture_layout,
        &glyph_texture,
        &resources.sampler,
    );

    DynamicBuffers {
        uniforms,
        draw_bind_group,
        vertices,
        tsunami,
        tsunami_bind_group,
        glyph_texture,
        glyph_bind_group,
    }
}

/// マップの描画範囲を決定する。
/// 地震の場合、震度情報または震央のいずれかまたは両方があればSomeを返す。
/// どちらも存在しない場合は不正値であり範囲が計算できないのでNoneを返す。
/// 津波の場合、発報範囲に関わらず固定値を返す。
pub fn calculate_bounding_box(payload: &FramePayload) -> BoundingBox<GeoDegree> {
    match payload {
        FramePayload::Earthquake(payload) => {
            let areas = payload
                .area_intensities
                .values()
                .flatten()
                .copied()
                .collect::<Vec<_>>();
            let bbox = areas
                .iter()
                .filter_map(|code| {
                    renderer_assets::QueryInterface::query_bounding_box_by_area(*code)
                })
                .fold(
                    BoundingBox {
                        min: Vertex::new(180.0, 90.0),
                        max: Vertex::new(-180.0, -90.0),
                    },
                    |acc, e| acc.merge_float(&e),
                );

            let bbox = payload
                .epicenter
                .iter()
                .fold(bbox, |bbox, epicenter| bbox.encapsulate_float(epicenter));

            if bbox.size().x() < 0.0 {
                panic!("Failed to determinate bounding_box");
            }

            bbox
        }
        FramePayload::TsunamiFirst(_) | FramePayload::TsunamiSecond(_) => {
            BoundingBox::new(Vertex::new(128.3, 30.0), Vertex::new(148.9, 45.5))
        }
    }
}

fn draw_tsunami_frame<T>(
    frame: &mut Frame,
    payload: &T,
    map_layers: MapLayerConfig,
    lines: Option<&drawer_tsunami_line::AreaLevels>,
) where
    T: HasEpicenter + HasTime + HasTsunamiForecastLevels,
{
    let main_view = frame.view;

    drawer_map::draw(frame, map_layers, Some(InsetRegion::Main));
    if let Some(levels) = lines {
        drawer_tsunami_line::draw(frame, InsetRegion::Main, levels);
    }
    drawer_tsunami_legends::draw(frame, payload);
    let epicenters = drawer_epicenter::prepare(frame, payload.epicenter());
    drawer_epicenter::draw(frame, epicenters.as_ref(), ICON_RATIO_IN_Y_AXIS);

    draw_insets(frame, epicenters.as_ref(), map_layers, lines);

    frame.set_view(main_view, None);
    drawer_overlay::draw(frame, payload);
}

fn draw_insets(
    frame: &mut Frame,
    epicenters: Option<&StagedSprites>,
    layers: MapLayerConfig,
    lines: Option<&drawer_tsunami_line::AreaLevels>,
) {
    let base_height = frame.view.size.y() as f32;

    for inset in inset::ALL_INSETS {
        let camera = inset.camera();
        let notch = inset.notch.as_ref().map(|n| notch::resolve(&camera, n));

        frame.set_view(View::from_camera(&camera), Some(inset.viewport));

        // 切り欠きがある場合は、残す側のステンシルを1にして、そこにだけ描く
        if let Some(notch) = &notch {
            frame.set_stencil_reference(1);
            drawer_inset_frame::draw_stencil_mask(frame, notch);
        }

        drawer_inset_frame::draw_background(frame);
        drawer_map::draw(frame, layers, Some(inset.region));
        if let Some(levels) = lines {
            drawer_tsunami_line::draw(frame, inset.region, levels);
        }

        // アイコンの大きさを本体の地図と揃える
        let icon_ratio = ICON_RATIO_IN_Y_AXIS * (base_height / inset.viewport.height as f32);
        drawer_epicenter::draw(frame, epicenters, icon_ratio);

        drawer_inset_frame::draw_border_and_label(
            frame,
            &inset.border_sides,
            notch.as_ref(),
            inset.label,
        );

        // 後続の描画が切り抜かれないよう、ステンシルを0に戻す
        if let Some(notch) = &notch {
            frame.set_stencil_reference(0);
            drawer_inset_frame::draw_stencil_mask(frame, notch);
        }
    }
}
