pub mod quake_prefecture {
    include!(concat!(env!("OUT_DIR"), "/quake_prefecture_v0.rs"));
}

mod border_line;
mod endpoint;
mod intensity;
mod intensity_icon;
mod model;
mod overlay;
mod rendering_context_v0;
mod resources;
mod vertex;

use std::borrow::Cow;
use std::error::Error;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::num::NonZeroU32;

use clap::Parser;
use glium::framebuffer::SimpleFrameBuffer;
use glium::glutin::context::NotCurrentGlContext;
use glium::glutin::display::{GetGlDisplay, GlDisplay};
use glium::texture::Texture2dDataSink;
use glium::{glutin, uniform, Display, Surface, Texture2d};
use winit::raw_window_handle::HasWindowHandle;

use crate::intensity_icon::EarthquakeInformation;
use crate::model::*;
use crate::rendering_context_v0::RenderingContextV0;
use renderer_types::*;

const DIMENSION: (u32, u32) = (1440, 1080);
const MAXIMUM_SCALE: f32 = 100.0;
const SCALE_FACTOR: f32 = 1.1;

struct RGBAImageData {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

#[derive(Parser)]
struct Cli {
    #[clap(env, long, default_value = "")]
    hmac_key: String,

    #[clap(env, long, default_value = "[not specified]")]
    instance_name: String,

    #[clap(long, env)]
    #[clap(default_value = "0.0.0.0:3000")]
    listen: SocketAddr,

    #[clap(env, long, default_value_t = false)]
    allow_demo: bool,
}

impl Texture2dDataSink<(u8, u8, u8, u8)> for RGBAImageData {
    fn from_raw(data: Cow<'_, [(u8, u8, u8, u8)]>, width: u32, height: u32) -> Self
    where
        [(u8, u8, u8, u8)]: ToOwned,
    {
        let data = data.into_owned();

        let ptr = data.as_ptr() as *mut u8;
        let length = data.len() * 4;
        let capacity = data.capacity() * 4;

        std::mem::forget(data);

        RGBAImageData {
            data: unsafe { Vec::from_raw_parts(ptr, length, capacity) },
            width,
            height,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = winit::event_loop::EventLoop::<UserEvent>::with_user_event().build()?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<UserEvent>(16);

    let proxy = event_loop.create_proxy();

    let cli = Cli::parse();

    tokio::spawn(async move {
        loop {
            let message = rx.recv().await.unwrap();
            proxy.send_event(message).unwrap();
        }
    });

    tokio::spawn(async move {
        endpoint::run(
            cli.listen,
            tx,
            &cli.hmac_key,
            &cli.instance_name,
            cli.allow_demo,
        )
        .await
    });

    let display = create_gl_context(&event_loop);

    let resources = resources::Resources::load(&display);

    let texture = &Texture2d::empty(&display, DIMENSION.0, DIMENSION.1).unwrap();
    let mut frame_buffer = SimpleFrameBuffer::new(&display, texture).unwrap();
    let aspect_ratio = DIMENSION.1 as f32 / DIMENSION.0 as f32;

    let params = glium::DrawParameters {
        multisampling: false,
        blend: glium::draw_parameters::Blend {
            color: glium::BlendingFunction::Addition {
                source: glium::draw_parameters::LinearBlendingFactor::SourceAlpha,
                destination: glium::draw_parameters::LinearBlendingFactor::OneMinusSourceAlpha,
            },
            alpha: glium::BlendingFunction::Max,
            constant_value: (0.0, 0.0, 0.0, 0.0),
        },
        ..Default::default()
    };

    event_loop
        .run(move |event, window_target| {
            use winit::event::Event::*;

            if matches!(event, UserEvent(model::UserEvent::Shutdown)) {
                window_target.exit();
                return;
            }

            let UserEvent(model::UserEvent::RenderingRequest((rendering_context, response_socket))) = event
            else {
                return;
            };

            frame_buffer.clear_color(0.5, 0.8, 1.0, 1.0);

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

            frame_buffer
                .draw(
                    &resources.buffer.vertex,
                    &resources.buffer.map,
                    &resources.shader.map,
                    &uniform! {
                        aspect_ratio: aspect_ratio,
                        color: [0.8_f32, 0.8, 0.8],
                        offset: offset.to_slice(),
                        zoom: scale,
                    },
                    &params,
                )
                .unwrap();

            border_line::draw(
                offset,
                aspect_ratio,
                scale,
                &resources,
                &mut frame_buffer,
                &params,
            );

            intensity_icon::draw_all(
                &EarthquakeInformation::new(
                    rendering_context.epicenter.as_ref(),
                    &rendering_context.area_intensities,
                ),
                offset,
                aspect_ratio,
                scale,
                &display,
                &mut frame_buffer,
                &resources,
                &params,
            );

            overlay::draw(
                &DIMENSION,
                &aspect_ratio,
                &display,
                &mut frame_buffer,
                &resources,
                &params,
            );

            println!("Rendered!");


            let pixel_buffer = texture.read_to_pixel_buffer();
            let pixels: RGBAImageData = pixel_buffer.read_as_texture_2d().unwrap();

            tokio::spawn(async move {
                use image::codecs::png::*;
                use image::ImageEncoder;

                if response_socket.is_closed() {
                    println!("もういらないっていわれちゃった……");
                    return;
                }

                let mut target = std::io::Cursor::new(Vec::new());

                let encoder = PngEncoder::new_with_quality(
                    &mut target,
                    CompressionType::Fast,
                    FilterType::Adaptive,
                );

                let image =
                    image::RgbaImage::from_raw(pixels.width, pixels.height, pixels.data).unwrap();
                let image = image::DynamicImage::ImageRgba8(image).flipv();

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
            });
        })
        .unwrap();

    Ok(())
}

fn create_gl_context<T>(
    event_loop: &winit::event_loop::EventLoop<T>,
) -> Display<glutin::surface::WindowSurface> {
    let attributes = winit::window::WindowAttributes::default().with_visible(false);
    let display_builder =
        glutin_winit::DisplayBuilder::new().with_window_attributes(Some(attributes));
    let config_template_builder = glutin::config::ConfigTemplateBuilder::new();

    let (window, gl_config) = display_builder
        .build(event_loop, config_template_builder, |mut configs| {
            configs.next().unwrap()
        })
        .unwrap();
    let window = window.unwrap();

    let attributes =
        glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new().build(
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

    let attributes = glutin::context::ContextAttributesBuilder::new()
        .build(Some(window.window_handle().unwrap().as_raw()));
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
