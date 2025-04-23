use std::cell::RefCell;
use crate::model::Message;
use crate::worker::fonts::{Font, FontManager, Offset, Origin};
use chrono_tz::Japan;
use glium::{
    draw_parameters::{Blend, LinearBlendingFactor},
    framebuffer::SimpleFrameBuffer,
    glutin::{
        config::ConfigTemplateBuilder,
        context::{ContextAttributesBuilder, NotCurrentGlContext},
        display::{GetGlDisplay, GlDisplay},
        surface::{SurfaceAttributesBuilder, WindowSurface},
    },
    uniform, BlendingFunction, Display, DrawParameters, Surface, Texture2d,
};
use glutin_winit::DisplayBuilder;
use image_buffer::RGBAImageData;
use renderer_types::*;
use rusttype::Scale;
use std::error::Error;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::ops::DerefMut;
use std::rc::Rc;
use glium::backend::Facade;
use tokio::sync::{mpsc, oneshot};
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;
use winit::{raw_window_handle::HasWindowHandle, window::WindowAttributes};
use crate::rendering_context_v0::RenderingContextV0;
use crate::worker::vertex::MapUniform;

mod drawer_border_line;
mod drawer_intensity_icon;
mod drawer_overlay;
mod fonts;
mod image_buffer;
mod resources;
mod vertex;
mod shader;

const DIMENSION: (u32, u32) = (1440, 1080);
const BACKGROUND_COLOR: (f32, f32, f32, f32) = (0.5, 0.8, 1.0, 1.0);
const GROUND_COLOR: [f32; 3] = [0.8, 0.8, 0.8];
const MAXIMUM_SCALE: f32 = 100.0;
const SCALE_FACTOR: f32 = 1.1;

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

pub struct FrameContext<'a, F: ?Sized + Facade, S: ?Sized + Surface> {
    pub facade: &'a F,
    pub surface: Rc<RefCell<S>>,
    pub rendering_context: &'a RenderingContextV0,
    pub resources: &'a resources::Resources<'a>,
    pub font_manager: &'a FontManager<'a>,
    pub draw_parameters: &'a DrawParameters<'a>,
    pub scale: f32,
    pub offset: Vertex<Screen>,
}

impl<F: ?Sized + Facade, S: ?Sized + Surface> FrameContext<'_, F, S> {
    pub fn dimension(&self) -> (u32, u32) {
        self.surface.borrow().get_dimensions()
    }

    pub fn aspect_ratio(&self) -> f32 {
        let dimension = self.dimension();
        dimension.1 as f32 / dimension.0 as f32
    }
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
        let resources = resources::Resources::load(&display);
        let font_manager = FontManager::new(&display);

        self.display = Some(display);
        self.resources = Some(resources);
        self.font_manager = Some(font_manager)
    }

    fn resumed(&mut self, _: &ActiveEventLoop) {}

    fn user_event(&mut self, _: &ActiveEventLoop, event: Message) {
        let (rendering_context, response_socket) = match event {
            Message::RenderingRequest(v) => v,
        };

        let display = self.display.as_ref().unwrap();
        let resources = self.resources.as_ref().unwrap();
        let font_manager = self.font_manager.as_mut().unwrap();

        let aspect_ratio = DIMENSION.1 as f32 / DIMENSION.0 as f32;

        let bounding_box = calculate_bounding_box(
            &rendering_context
                .area_intensities
                .values()
                .flatten()
                .copied()
                .collect::<Vec<_>>(),
            rendering_context.epicenter.as_ref(),
        );

        let rendering_bbox = BoundingBox::from_vertices(
            &bounding_box
                .gl_vertices()
                .iter()
                .map(|v| v.to_screen())
                .collect::<Vec<_>>(),
        );
        let offset = -rendering_bbox.center();
        let scale = calculate_map_scale(rendering_bbox, aspect_ratio);

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

        let texture = Texture2d::empty(display, DIMENSION.0, DIMENSION.1).unwrap();
        let frame_buffer = SimpleFrameBuffer::new(display, &texture).unwrap();
        let frame_buffer = Rc::new(RefCell::new(frame_buffer));

        let frame_context = FrameContext {
            facade: display,
            surface: frame_buffer.clone(),
            rendering_context: &rendering_context,
            resources,
            font_manager,
            draw_parameters: &draw_parameters,
            scale,
            offset,
        };

        frame_buffer
            .borrow_mut()
            .clear_color(
                BACKGROUND_COLOR.0,
                BACKGROUND_COLOR.1,
                BACKGROUND_COLOR.2,
                BACKGROUND_COLOR.3,
            );

        resources
            .shader
            .map
            .draw(
                frame_buffer.borrow_mut().deref_mut(),
                &resources.buffer.vertex,
                &resources.buffer.map,
                &MapUniform {
                    aspect_ratio: aspect_ratio,
                    offset: offset.to_slice(),
                    zoom: scale,
                    color: GROUND_COLOR,
                },
                &draw_parameters,
            )
            .unwrap();

        drawer_border_line::draw(&frame_context);

        drawer_intensity_icon::draw_all(&frame_context);

        drawer_overlay::draw(&frame_context);

        let time_text = rendering_context
            .time
            .with_timezone(&Japan)
            .format("%Y年%m月%d日 %H時%M分頃発生")
            .to_string();
        let font = Font::BizUDPGothicBold;
        let color = [0.0f32, 0.0, 0.0, 0.63];
        let scale = Scale::uniform(30.0);
        let offset = Offset::new(Origin::RightDown, Origin::RightDown, -30, -30);
        font_manager.draw_text(
            &time_text,
            font,
            color,
            scale,
            offset,
            DIMENSION,
            resources,
            display,
            frame_buffer.borrow_mut().deref_mut(),
            &draw_parameters,
        );

        println!("Rendered!");

        let pixel_buffer = texture.read_to_pixel_buffer();
        let image: RGBAImageData = pixel_buffer.read_as_texture_2d().unwrap();

        tokio::spawn(async move { image_writeback(response_socket, image).await });
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

async fn image_writeback(response_socket: oneshot::Sender<Vec<u8>>, image: RGBAImageData) {
    use std::io::Cursor;

    use image::codecs::png::*;
    use image::{DynamicImage, ImageEncoder, RgbaImage};

    if response_socket.is_closed() {
        println!("もういらないっていわれちゃった……");
        return;
    }

    let mut target = Cursor::new(Vec::new());

    let encoder =
        PngEncoder::new_with_quality(&mut target, CompressionType::Fast, FilterType::Adaptive);

    let image = RgbaImage::from_raw(image.width, image.height, image.data).unwrap();
    let image = DynamicImage::ImageRgba8(image).flipv();

    encoder
        .write_image(
            image.as_bytes(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();

    let target: Vec<u8> = target.into_inner();

    println!("Encoded");

    let 相手はもういらないかもしれない = response_socket.send(target);

    if 相手はもういらないかもしれない.is_err() {
        println!("えんこーどまでしたのにー…むきーっ！");
    }
}

fn create_gl_context(event_loop: &ActiveEventLoop) -> Display<WindowSurface> {
    let display_builder = DisplayBuilder::new()
        .with_window_attributes(Some(WindowAttributes::default().with_visible(false)));

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

    Display::from_context_surface(current_context, surface).unwrap()
}

pub fn calculate_bounding_box(
    areas: &[u32],
    epicenter: Option<&Vertex<GeoDegree>>,
) -> BoundingBox<GeoDegree> {
    let bbox = areas
        .iter()
        .filter_map(|code| renderer_assets::QueryInterface::query_bounding_box_by_area(*code))
        .fold(
            BoundingBox {
                min: Vertex {
                    x: 180.0,
                    y: 90.0,
                    _type: PhantomData,
                },
                max: Vertex {
                    x: -180.0,
                    y: -90.0,
                    _type: PhantomData,
                },
            },
            |acc, e| acc.extends_with(&e),
        );

    if let Some(epicenter) = epicenter {
        bbox.extends_by_vertex(epicenter)
    } else {
        bbox
    }
}

fn calculate_map_scale(bounding_box: BoundingBox<Screen>, aspect_ratio: f32) -> f32 {
    let x_scale = 1.0 / bounding_box.size().x;
    let y_scale = 1.0 / bounding_box.size().y * aspect_ratio;

    f32::min(f32::min(x_scale, y_scale) * 2.0, MAXIMUM_SCALE) / SCALE_FACTOR
}
