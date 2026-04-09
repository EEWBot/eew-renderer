use crate::frame_context::FramePayload;
use crate::model::Message;
use crate::worker::fonts::FontManager;
use crate::worker::theme::Theme;
use glium::backend::Facade;
use glium::glutin::surface::{GlSurface, SwapInterval};
use glium::{
    draw_parameters::{Blend, LinearBlendingFactor},
    framebuffer::SimpleFrameBuffer,
    glutin::{
        config::ConfigTemplateBuilder,
        context::{ContextAttributesBuilder, NotCurrentGlContext},
        display::{GetGlDisplay, GlDisplay},
        surface::{SurfaceAttributesBuilder, WindowSurface},
    },
    BlendingFunction, Display, DrawParameters, Surface, Texture2d,
};
use glutin_winit::DisplayBuilder;
use image_buffer::RGBAImageData;
use renderer_types::*;
use std::cell::RefCell;
use std::error::Error;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Instant;
use tokio::sync::mpsc;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;
use winit::{raw_window_handle::HasWindowHandle, window::WindowAttributes};

mod drawer_epicenter;
mod drawer_intensity_icon;
mod drawer_map;
mod drawer_overlay;
mod drawer_tsunami_legends;
mod drawer_tsunami_line;
mod fonts;
pub mod image_buffer;
mod resources;
mod shader;
mod theme;
mod vertex;

const DIMENSION: (u32, u32) = (1024, 768);
const MAXIMUM_SCALE: f32 = 100.0;
const SCALE_FACTOR: f32 = 1.2;
const ICON_RATIO_IN_Y_AXIS: f32 = 0.05;

pub async fn run(mut rx: mpsc::Receiver<Message>) -> Result<(), Box<dyn Error>> {
    let event_loop = winit::event_loop::EventLoop::<Message>::with_user_event().build()?;

    let proxy = event_loop.create_proxy();

    tokio::spawn(async move {
        loop {
            let message = rx.recv().await.unwrap();
            proxy.send_event(message).unwrap();
        }
    });

    event_loop.run_app(&mut App::default()).unwrap();

    Ok(())
}

pub struct FrameContext<'a, 'b, F: ?Sized + Facade, S: ?Sized + Surface> {
    pub facade: &'a F,
    pub surface: Rc<RefCell<S>>,
    pub image_size: Size<u32>, // TODO: Themeに移動する
    pub theme: &'a Theme,
    pub resources: &'a resources::Resources<'a>,
    pub font_manager: Rc<RefCell<&'a mut FontManager<'b>>>,
    pub draw_parameters: &'a DrawParameters<'a>,
    pub scale: f32,
    pub offset: Vertex<Mercator>,
}

#[derive(Default)]
struct App<'a> {
    display: Option<Display<WindowSurface>>,
    resources: Option<resources::Resources<'a>>,
    font_manager: Option<FontManager<'a>>,
}

impl ApplicationHandler<Message> for App<'_> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if cause != StartCause::Init {
            return;
        }

        let display = create_gl_context(event_loop);

        let gl_vendor = display.get_opengl_vendor_string();
        let gl_renderer = display.get_opengl_renderer_string();
        let gl_version = display.get_opengl_version_string();

        tracing::info!("GL_VENDOR: {gl_vendor}");
        tracing::info!("GL_RENDERER: {gl_renderer}");
        tracing::info!("GL_VERSION: {gl_version}");

        let resources = resources::Resources::load(&display);
        let font_manager = FontManager::new(&display);

        self.display = Some(display);
        self.resources = Some(resources);
        self.font_manager = Some(font_manager);
    }

    fn resumed(&mut self, _: &ActiveEventLoop) {}

    fn user_event(&mut self, _: &ActiveEventLoop, event: Message) {
        let Message::FrameRequest((request_frame_context, response_socket)) = event;

        let start_at = std::time::Instant::now();

        let display = self.display.as_ref().unwrap();
        let resources = self.resources.as_ref().unwrap();
        let font_manager = self.font_manager.as_mut().unwrap();
        let font_manager = Rc::new(RefCell::new(font_manager));

        let image_size = Size::from(DIMENSION);

        let bounding_box = calculate_bounding_box(&request_frame_context.payload);

        let rendering_bbox = BoundingBox::from_vertices_float(
            &bounding_box
                .gl_vertices()
                .iter()
                .map(|v| v.to_mercator())
                .collect::<Vec<_>>(),
        );
        let offset = -rendering_bbox.center();
        let scale = calculate_map_scale(rendering_bbox, image_size);

        let draw_parameters = DrawParameters {
            multisampling: false,
            blend: Blend {
                color: BlendingFunction::Addition {
                    source: LinearBlendingFactor::SourceAlpha,
                    destination: LinearBlendingFactor::OneMinusSourceAlpha,
                },
                alpha: BlendingFunction::Max,
                constant_value: (0.0, 0.0, 0.0, 0.0),
            },
            ..Default::default()
        };

        let t_before_alloc = Instant::now();

        let texture = Texture2d::empty(display, image_size.x(), image_size.y()).unwrap();
        let frame_buffer = SimpleFrameBuffer::new(display, &texture).unwrap();
        let frame_buffer = Rc::new(RefCell::new(frame_buffer));

        let t_before_render = Instant::now();

        let frame_context = FrameContext {
            facade: display,
            surface: frame_buffer.clone(),
            image_size,
            theme: &theme::DEFAULT,
            resources,
            font_manager,
            draw_parameters: &draw_parameters,
            scale,
            offset,
        };

        let clear_color = frame_context.theme.clear_color;
        frame_buffer.borrow_mut().clear_color(
            clear_color[0],
            clear_color[1],
            clear_color[2],
            clear_color[3],
        );

        match &request_frame_context.payload {
            FramePayload::Earthquake(earthquake) => {
                drawer_map::draw(&frame_context, true);
                drawer_intensity_icon::draw_all(&frame_context, earthquake);
                drawer_epicenter::draw(&frame_context, earthquake);
                drawer_overlay::draw(&frame_context, earthquake);
            }
            FramePayload::TsunamiFirst(tsunami) => {
                drawer_map::draw(&frame_context, false);
                drawer_tsunami_line::draw(&frame_context, tsunami);
                drawer_tsunami_legends::draw(&frame_context, tsunami);
                drawer_epicenter::draw(&frame_context, tsunami);
                drawer_overlay::draw(&frame_context, tsunami);
            }
            FramePayload::TsunamiSecond(tsunami) => {
                drawer_map::draw(&frame_context, false);
                drawer_tsunami_legends::draw(&frame_context, tsunami);
                drawer_epicenter::draw(&frame_context, tsunami);
                drawer_overlay::draw(&frame_context, tsunami);
            }
        }

        let t_before_bufcpy = Instant::now();

        let image: RGBAImageData = texture.read();

        let t_done = Instant::now();

        tracing::info!(
            "Init: {:?} Alloc: {:?} Render: {:?} BufCpy: {:?} ({})",
            t_before_alloc - start_at,
            t_before_render - t_before_alloc,
            t_before_bufcpy - t_before_render,
            t_done - t_before_bufcpy,
            request_frame_context.request_identity,
        );

        let _ = response_socket.send(Ok(image));
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

fn create_gl_context(event_loop: &ActiveEventLoop) -> Display<WindowSurface> {
    let display_builder =
        DisplayBuilder::new().with_window_attributes(Some(WindowAttributes::default()));

    let (window, gl_config) = display_builder
        .build(event_loop, ConfigTemplateBuilder::new(), |mut configs| {
            configs.next().unwrap()
        })
        .unwrap();

    let window = window.unwrap();

    let attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window.window_handle().unwrap().as_raw(),
        NonZeroU32::new(1).unwrap(),
        NonZeroU32::new(1).unwrap(),
    );

    let surface = unsafe {
        gl_config
            .display()
            .create_window_surface(&gl_config, &attributes)
            .unwrap()
    };

    let attributes =
        ContextAttributesBuilder::new().build(Some(window.window_handle().unwrap().as_raw()));

    let current_context = unsafe {
        gl_config
            .display()
            .create_context(&gl_config, &attributes)
            .unwrap()
    }
    .make_current(&surface)
    .unwrap();

    surface
        .set_swap_interval(&current_context, SwapInterval::DontWait)
        .unwrap();

    Display::from_context_surface(current_context, surface).unwrap()
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
                    renderer_assets::QueryInterface::query_bounding_box_by_地震情報細分区域(
                        *code,
                    )
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
            BoundingBox::new(Vertex::new(122.9, 24.0), Vertex::new(148.9, 45.5))
        }
    }
}

fn calculate_map_scale(bounding_box: BoundingBox<Mercator>, image_size: Size<u32>) -> f32 {
    let x_scale = 1.0 / bounding_box.size().x();
    let y_scale = 1.0 / bounding_box.size().y() * image_size.aspect_ratio();

    f32::min(f32::min(x_scale, y_scale) * 2.0, MAXIMUM_SCALE) / SCALE_FACTOR
}
