use std::error::Error;
use std::marker::PhantomData;
use std::num::NonZeroU32;

use glium::framebuffer::SimpleFrameBuffer;
use glium::glutin::context::NotCurrentGlContext;
use glium::glutin::display::{GetGlDisplay, GlDisplay};
use glium::{glutin, uniform, Display, Surface, Texture2d};
use winit::raw_window_handle::HasWindowHandle;

use crate::model::Message;
use image_buffer::RGBAImageData;
use renderer_types::*;

mod drawer_border_line;
mod drawer_intensity_icon;
mod drawer_overlay;
mod image_buffer;
mod resources;
mod vertex;

const DIMENSION: (u32, u32) = (1440, 1080);
const MAXIMUM_SCALE: f32 = 100.0;
const SCALE_FACTOR: f32 = 1.1;

pub async fn run(mut rx: tokio::sync::mpsc::Receiver<Message>) -> Result<(), Box<dyn Error>> {
    let event_loop = winit::event_loop::EventLoop::<Message>::with_user_event().build()?;

    let proxy = event_loop.create_proxy();

    tokio::spawn(async move {
        loop {
            let message = rx.recv().await.unwrap();
            proxy.send_event(message).unwrap();
        }
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
        .run(move |event, _window_target| {
            use winit::event::Event::*;

            let (rendering_context, response_socket) = match event {
                UserEvent(Message::RenderingRequest(v)) => v,
                _ => return,
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

            drawer_border_line::draw(
                offset,
                aspect_ratio,
                scale,
                &resources,
                &mut frame_buffer,
                &params,
            );

            drawer_intensity_icon::draw_all(
                rendering_context.epicenter.as_ref(),
                &rendering_context.area_intensities,
                offset,
                aspect_ratio,
                scale,
                &display,
                &mut frame_buffer,
                &resources,
                &params,
            );

            drawer_overlay::draw(
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
