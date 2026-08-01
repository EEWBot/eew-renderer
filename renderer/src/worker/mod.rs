use crate::frame_context::{FramePayload, HasEpicenter, HasTime, HasTsunamiForecastLevels};
use crate::model::Message;
use crate::worker::camera::Camera;
use crate::worker::fonts::FontManager;
use crate::worker::theme::Theme;
use glium::backend::Facade;
use glium::glutin::surface::{GlSurface, SwapInterval};
use glium::{
    draw_parameters::{Blend, LinearBlendingFactor, Stencil, StencilOperation, StencilTest},
    framebuffer::{SimpleFrameBuffer, StencilRenderBuffer},
    glutin::{
        config::ConfigTemplateBuilder,
        context::{ContextAttributesBuilder, NotCurrentGlContext},
        display::{GetGlDisplay, GlDisplay},
        surface::{SurfaceAttributesBuilder, WindowSurface},
    },
    texture::StencilFormat,
    uniforms::MagnifySamplerFilter,
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
use winit::window::{Window, WindowId};
use winit::{raw_window_handle::HasWindowHandle, window::WindowAttributes};

mod camera;
mod drawer_epicenter;
mod drawer_inset_frame;
mod drawer_intensity_icon;
mod drawer_map;
mod drawer_overlay;
mod drawer_tsunami_legends;
mod drawer_tsunami_line;
mod fonts;
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
mod headless;
pub mod image_buffer;
mod inset;
mod notch;
mod resources;
mod shader;
mod theme;
mod vertex;

const DIMENSION: (u32, u32) = (1024, 768);
const MAP_MARGIN_FACTOR: f32 = 1.2;
const ICON_RATIO_IN_Y_AXIS: f32 = 0.05;

pub async fn run(
    rx: mpsc::Receiver<Message>,
    headless: bool,
    egl_device_index: usize,
) -> Result<(), Box<dyn Error>> {
    if headless {
        run_headless(rx, egl_device_index).await
    } else {
        run_windowed(rx).await
    }
}

async fn run_windowed(mut rx: mpsc::Receiver<Message>) -> Result<(), Box<dyn Error>> {
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

async fn run_headless(
    mut rx: mpsc::Receiver<Message>,
    egl_device_index: usize,
) -> Result<(), Box<dyn Error>> {
    let context = create_headless_context(egl_device_index)?;

    log_gl_info(&context);

    let resources = resources::Resources::load(&context);
    let mut font_manager = FontManager::new(&context);
    let render_target = RenderTarget::new(&context);

    while let Some(message) = rx.recv().await {
        render_frame(&context, &render_target, &resources, &mut font_manager, message);
    }

    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn create_headless_context(
    egl_device_index: usize,
) -> Result<Rc<glium::backend::Context>, Box<dyn Error>> {
    headless::create(egl_device_index)
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn create_headless_context(_: usize) -> Result<Rc<glium::backend::Context>, Box<dyn Error>> {
    Err("Headless mode requires EGL, which is not available on this platform".into())
}

fn log_gl_info<F: Facade>(facade: &F) {
    let context = facade.get_context();

    tracing::info!("GL_VENDOR: {}", context.get_opengl_vendor_string());
    tracing::info!("GL_RENDERER: {}", context.get_opengl_renderer_string());
    tracing::info!("GL_VERSION: {}", context.get_opengl_version_string());
}

pub struct FrameContext<'a, 'b, 'c, F: ?Sized + Facade, S: ?Sized + Surface> {
    pub facade: &'a F,
    pub surface: Rc<RefCell<S>>,
    pub theme: &'a Theme,
    pub resources: &'a resources::Resources<'a>,
    pub font_manager: Rc<RefCell<&'a mut FontManager<'b>>>,
    pub draw_parameters: &'c DrawParameters<'c>,
    pub camera: Camera,
}

#[derive(Default)]
struct App<'a> {
    display: Option<Display<WindowSurface>>,
    window: Option<Window>,
    resources: Option<resources::Resources<'a>>,
    font_manager: Option<FontManager<'a>>,
    render_target: Option<RenderTarget>,
}

impl ApplicationHandler<Message> for App<'_> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if cause != StartCause::Init {
            return;
        }

        let (display, window) = create_gl_context(event_loop);

        log_gl_info(&display);

        let resources = resources::Resources::load(&display);
        let font_manager = FontManager::new(&display);

        self.render_target = Some(RenderTarget::new(&display));
        self.display = Some(display);
        self.window = Some(window);
        self.resources = Some(resources);
        self.font_manager = Some(font_manager);
    }

    fn resumed(&mut self, _: &ActiveEventLoop) {}

    fn user_event(&mut self, _: &ActiveEventLoop, event: Message) {
        let display = self.display.as_ref().unwrap();
        let render_target = self.render_target.as_ref().unwrap();
        let resources = self.resources.as_ref().unwrap();
        let font_manager = self.font_manager.as_mut().unwrap();

        render_frame(display, render_target, resources, font_manager, event);
        present(display, &render_target.texture);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(display) = self.display.as_ref() {
                    display.resize((size.width, size.height));
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(display), Some(render_target)) =
                    (self.display.as_ref(), self.render_target.as_ref())
                {
                    present(display, &render_target.texture);
                }
            }
            _ => {}
        }
    }
}

struct RenderTarget {
    texture: Texture2d,
    stencil_buffer: StencilRenderBuffer,
}

impl RenderTarget {
    fn new<F: ?Sized + Facade>(facade: &F) -> Self {
        let image_size = Size::from(DIMENSION);

        let texture = Texture2d::empty(facade, image_size.x(), image_size.y()).unwrap();
        let stencil_buffer =
            StencilRenderBuffer::new(facade, StencilFormat::I8, image_size.x(), image_size.y())
                .unwrap();

        Self {
            texture,
            stencil_buffer,
        }
    }

    fn frame_buffer<F: ?Sized + Facade>(&self, facade: &F) -> SimpleFrameBuffer<'_> {
        SimpleFrameBuffer::with_stencil_buffer(facade, &self.texture, &self.stencil_buffer)
            .unwrap()
    }
}

fn render_frame<F: ?Sized + Facade>(
    facade: &F,
    target: &RenderTarget,
    resources: &resources::Resources<'_>,
    font_manager: &mut FontManager<'_>,
    event: Message,
) {
    let Message::FrameRequest((request_frame_context, response_socket)) = event;

    let start_at = std::time::Instant::now();

    let font_manager = Rc::new(RefCell::new(font_manager));

    let image_size = Size::from(DIMENSION);

    let bounding_box = calculate_bounding_box(&request_frame_context.payload);

    let camera = camera::Camera::fit(&bounding_box, image_size).with_margin(MAP_MARGIN_FACTOR);

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

    let frame_buffer = Rc::new(RefCell::new(target.frame_buffer(facade)));

    let t_before_render = Instant::now();

    let frame_context = FrameContext {
        facade,
        surface: frame_buffer.clone(),
        theme: &theme::DEFAULT,
        resources,
        font_manager,
        draw_parameters: &draw_parameters,
        camera,
    };

    let clear_color = frame_context.theme.clear_color;
    frame_buffer.borrow_mut().clear(
        None,
        Some((
            clear_color[0],
            clear_color[1],
            clear_color[2],
            clear_color[3],
        )),
        false,
        None,
        Some(0),
    );

    let map_layers = request_frame_context.payload.map_layers();

    match &request_frame_context.payload {
        FramePayload::Earthquake(earthquake) => {
            drawer_map::draw(&frame_context, map_layers, None);
            drawer_intensity_icon::draw_all(&frame_context, earthquake);
            drawer_epicenter::draw(&frame_context, earthquake);
            drawer_overlay::draw(&frame_context, earthquake);
        }
        FramePayload::TsunamiFirst(tsunami) => {
            draw_tsunami_frame(&frame_context, tsunami, map_layers, true);
        }
        FramePayload::TsunamiSecond(tsunami) => {
            draw_tsunami_frame(&frame_context, tsunami, map_layers, false);
        }
    }

    let t_before_bufcpy = Instant::now();

    let image: RGBAImageData = target.texture.read();

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

fn present(display: &Display<WindowSurface>, texture: &Texture2d) {
    let frame = display.draw();
    texture
        .as_surface()
        .fill(&frame, MagnifySamplerFilter::Nearest);
    if let Err(err) = frame.finish() {
        tracing::warn!("Failed to present frame: {err:?}");
    }
}

fn create_gl_context(event_loop: &ActiveEventLoop) -> (Display<WindowSurface>, Window) {
    let window_attributes = WindowAttributes::default()
        .with_title("eew-renderer")
        .with_inner_size(winit::dpi::PhysicalSize::new(DIMENSION.0, DIMENSION.1))
        .with_resizable(false);
    let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));

    let (window, gl_config) = display_builder
        .build(event_loop, ConfigTemplateBuilder::new(), |mut configs| {
            configs.next().unwrap()
        })
        .unwrap();

    let window = window.unwrap();

    let size = window.inner_size();
    let attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window.window_handle().unwrap().as_raw(),
        NonZeroU32::new(size.width).unwrap_or(NonZeroU32::new(1).unwrap()),
        NonZeroU32::new(size.height).unwrap_or(NonZeroU32::new(1).unwrap()),
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

    (
        Display::from_context_surface(current_context, surface).unwrap(),
        window,
    )
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

fn draw_tsunami_frame<F: ?Sized + Facade, S: ?Sized + Surface, T>(
    frame_context: &FrameContext<F, S>,
    payload: &T,
    map_layers: crate::frame_context::MapLayerConfig,
    with_lines: bool,
) where
    T: HasEpicenter + HasTime + HasTsunamiForecastLevels,
{
    let facade = frame_context.facade;

    let epicenter_buffer = drawer_epicenter::create_vertex_buffer(facade, payload.epicenter());
    let levels = with_lines.then(|| drawer_tsunami_line::build_levels_texture(facade, payload));

    drawer_map::draw(frame_context, map_layers, Some(InsetRegion::Main));
    if let Some(levels) = &levels {
        drawer_tsunami_line::draw(frame_context, InsetRegion::Main, levels);
    }
    drawer_tsunami_legends::draw(frame_context, payload);
    if let Some(buffer) = &epicenter_buffer {
        drawer_epicenter::draw_vertex_buffer(frame_context, buffer, ICON_RATIO_IN_Y_AXIS);
    }
    draw_insets(
        frame_context,
        epicenter_buffer.as_ref(),
        map_layers,
        levels.as_ref(),
    );
    drawer_overlay::draw(frame_context, payload);
}

fn draw_insets<F: ?Sized + Facade, S: ?Sized + Surface>(
    base: &FrameContext<F, S>,
    epicenter_buffer: Option<&glium::VertexBuffer<vertex::EpicenterVertex>>,
    layers: crate::frame_context::MapLayerConfig,
    levels: Option<&glium::texture::UnsignedTexture1d>,
) {
    for inset in inset::ALL_INSETS {
        let camera = inset.camera();
        let notch = inset.notch.as_ref().map(|n| notch::resolve(&camera, n));

        let mut draw_parameters = base.draw_parameters.clone();
        draw_parameters.viewport = Some(inset.viewport);
        if notch.is_some() {
            draw_parameters.stencil =
                stencil_params(StencilTest::IfEqual { mask: 0xff }, StencilOperation::Keep);
        }

        let frame_context = FrameContext {
            facade: base.facade,
            surface: base.surface.clone(),
            theme: base.theme,
            resources: base.resources,
            font_manager: base.font_manager.clone(),
            draw_parameters: &draw_parameters,
            camera,
        };

        if let Some(notch) = &notch {
            drawer_inset_frame::draw_stencil_mask(&frame_context, notch);
        }
        drawer_inset_frame::draw_background(&frame_context);
        drawer_map::draw(&frame_context, layers, Some(inset.region));
        if let Some(levels) = levels {
            drawer_tsunami_line::draw(&frame_context, inset.region, levels);
        }
        if let Some(buffer) = epicenter_buffer {
            let icon_ratio = ICON_RATIO_IN_Y_AXIS
                * (base.camera.image_size.y() as f32 / inset.viewport.height as f32);
            drawer_epicenter::draw_vertex_buffer(&frame_context, buffer, icon_ratio);
        }
        drawer_inset_frame::draw_border_and_label(
            &frame_context,
            &inset.border_sides,
            notch.as_ref(),
            inset.label,
        );
    }
}

fn stencil_params(test: StencilTest, operation: StencilOperation) -> Stencil {
    Stencil {
        test_clockwise: test,
        reference_value_clockwise: 1,
        fail_operation_clockwise: operation,
        pass_depth_fail_operation_clockwise: operation,
        depth_pass_operation_clockwise: operation,
        test_counter_clockwise: test,
        reference_value_counter_clockwise: 1,
        fail_operation_counter_clockwise: operation,
        pass_depth_fail_operation_counter_clockwise: operation,
        depth_pass_operation_counter_clockwise: operation,
        ..Default::default()
    }
}
