mod api_scheme;
mod border_line;
mod endpoint;
mod intensity;
mod intensity_icon;
mod model;
mod overlay;
mod resources;
mod temporary_data;
mod vertex;

use std::borrow::Cow;
use std::error::Error;
use std::io::Write;
use std::marker::PhantomData;

use enum_map::enum_map;
use glium::framebuffer::SimpleFrameBuffer;
use glium::texture::Texture2dDataSink;
use glium::{uniform, Surface, Texture2d};

use crate::intensity::震度;
use crate::model::*;

use renderer_types::*;
use crate::intensity_icon::EarthquakeInformation;

const DIMENSION: (u32, u32) = (1440, 1080);
const MAXIMUM_SCALE: f32 = 100.0;
const SCALE_FACTOR: f32 = 1.1;

struct RGBAImageData {
    data: Vec<u8>,
    width: u32,
    height: u32,
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
    let event_loop = winit::event_loop::EventLoopBuilder::<UserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();

    tokio::spawn(async move { endpoint::run("0.0.0.0:3000", proxy).await });

    let (_window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .with_inner_size(DIMENSION.0, DIMENSION.1)
        .build(&event_loop);

    let resources = resources::Resources::load(&display);
    let mut context: RenderingContext = Default::default();

    let render_texture = Texture2d::empty(&display, DIMENSION.0, DIMENSION.1).unwrap();
    let mut render_frame_buffer = SimpleFrameBuffer::new(&display, &render_texture).unwrap();
    let screenshot_texture = &Texture2d::empty(&display, DIMENSION.0, DIMENSION.1).unwrap();
    let screenshot_frame_buffer = SimpleFrameBuffer::new(&display, screenshot_texture).unwrap();
    let aspect_ratio = DIMENSION.1 as f32 / DIMENSION.0 as f32;

    // 2024-04-17 23:14 豊後水道
    // let earthquake_information = enum_map! {
    //     震度::震度1 => vec![720, 736, 431, 460, 551, 354, 400, 401, 354, 400, 401, 421, 422, 500, 501],
    //     震度::震度2 => vec![775, 531, 532, 535, 562, 575, 601, 710, 712, 730, 432, 451, 461, 462, 510, 511, 520, 521, 540, 550],
    //     震度::震度3 => vec![630, 631, 590, 591, 700, 702, 741, 743, 762, 771, 530, 560, 563, 570, 571, 580, 581, 600, 610, 611, 711, 713, 721, 731, 732],
    //     震度::震度4 => vec![620, 621, 750, 753, 592, 703, 704, 740, 742, 760, 761, 763, 770],
    //     震度::震度5弱 => vec![751, 752],
    //     震度::震度6弱 => vec![622, 632],
    //     _ => vec![]
    // };
    // let epicenter = Vertex::new(132.4, 33.2);

    // 2024-01-01 16:10 石川県能登地方
    let earthquake_information = enum_map!{
        震度::震度1 => vec![211, 355, 357, 203, 590, 622, 632, 741, 101, 106, 107, 161, 700, 703, 704, 711, 713],
        震度::震度2 => vec![332, 440, 532, 210, 213, 351, 352, 354, 356, 551, 571, 601, 611, 200, 201, 202, 591, 592, 620, 621, 630, 631, 721, 740, 751, 763, 770],
        震度::震度3 => vec![241, 251, 301, 311, 321, 331, 441, 442, 450, 461, 462, 510, 521, 531, 535, 562, 563, 212, 220, 221, 222, 230, 231, 232, 233, 340, 341, 342, 350, 360, 361, 411, 412, 550, 570, 575, 580, 581, 600, 610],
        震度::震度4 => vec![401, 421, 422, 431, 432, 240, 242, 243, 250, 252, 300, 310, 320, 330, 443, 451, 460, 500, 501, 511, 520, 530, 540, 560],
        震度::震度5弱 => vec![420, 430],
        震度::震度5強 => vec![391, 370, 372, 375, 380, 381, 400],
        震度::震度6弱 => vec![371],
        震度::震度7 => vec![390],
        _ => vec![]
    };
    let epicenter = Vertex::<GeoDegree>::new(137.2, 37.5);

    for code in earthquake_information.values().flatten() {
        match renderer_assets::QueryInterface::query_bounding_box_by_area(*code) {
            None => println!("{code} is requested but †Unknown code†"),
            Some(_bbox) => {},
        }
    }

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
            use winit::event::WindowEvent::*;
            use winit::keyboard::*;

            let reason: RedrawReason = match event {
                WindowEvent { event: we, .. } => match we {
                    CloseRequested => {
                        window_target.exit();
                        return;
                    }
                    KeyboardInput { event: ke, .. } if !ke.repeat && ke.state.is_pressed() => {
                        match ke.physical_key {
                            PhysicalKey::Code(KeyCode::KeyQ) => {
                                window_target.exit();
                                return;
                            }
                            PhysicalKey::Code(KeyCode::Space) => RedrawReason::ScreenShot,
                            _ => return,
                        }
                    }
                    RedrawRequested => RedrawReason::Other,
                    _ => return,
                },
                UserEvent(model::UserEvent::Shutdown) => {
                    window_target.exit();
                    return;
                }
                UserEvent(model::UserEvent::RenderingRequest(sender)) => RedrawReason::Web(sender),
                _ => return,
            };

            render_frame_buffer.clear_color(0.5, 0.8, 1.0, 1.0);

            let bounding_box = calculate_bounding_box(
                &earthquake_information
                    .values()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>(),
            );
            let rendering_bbox = BoundingBox::from_vertices(
                &bounding_box.gl_vertices().iter().map(|v| v.to_screen()).collect::<Vec<_>>()
            );
            let offset = -rendering_bbox.center();
            let scale = calculate_map_scale(rendering_bbox, aspect_ratio);

            render_frame_buffer
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
                &mut render_frame_buffer,
                &params,
            );

            intensity_icon::draw_all(
                &EarthquakeInformation::new(&epicenter, &earthquake_information),
                offset,
                aspect_ratio,
                scale,
                &display,
                &mut render_frame_buffer,
                &resources,
                &params,
            );

            overlay::draw(
                &DIMENSION,
                &aspect_ratio,
                &display,
                &mut render_frame_buffer,
                &resources,
                &params,
            );

            let frame = display.draw();

            let size = SizeU::from_tuple(DIMENSION).to_f();
            let target = SizeU::from_tuple(frame.get_dimensions()).to_f();
            let size = size.capped_fit(&target);
            let offset = (target - size).scale(0.5).to_i();
            let size = size.to_i();

            let source = glium::Rect {
                left: 0,
                bottom: 0,
                width: DIMENSION.0,
                height: DIMENSION.1,
            };

            render_frame_buffer.blit_color(
                &source,
                &frame,
                &glium::BlitTarget {
                    left: offset.x,
                    bottom: offset.y,
                    width: size.x as i32,
                    height: size.y as i32,
                },
                glium::uniforms::MagnifySamplerFilter::Linear,
            );

            frame.finish().unwrap();

            render_frame_buffer.blit_color(
                &source,
                &screenshot_frame_buffer,
                &glium::BlitTarget {
                    left: 0,
                    bottom: DIMENSION.1,
                    width: DIMENSION.0 as i32,
                    height: -(DIMENSION.1 as i32),
                },
                glium::uniforms::MagnifySamplerFilter::Nearest,
            );

            println!("Rendered!");

            if !reason.is_buffer_needed() {
                return;
            }

            let pixel_buffer = screenshot_texture.read_to_pixel_buffer();
            let pixels: RGBAImageData = pixel_buffer.read_as_texture_2d().unwrap();

            let enc_context = match reason {
                RedrawReason::ScreenShot => {
                    context.screenshot_count += 1;
                    EncodeContext::ScreenShot(context.screenshot_count)
                }
                RedrawReason::Web(sender) => EncodeContext::Web(sender),
                RedrawReason::Other => unreachable!(),
            };

            tokio::spawn(async move {
                use image::codecs::png::*;
                use image::ImageEncoder;

                if let EncodeContext::Web(socket) = &enc_context {
                    if socket.is_closed() {
                        println!("もういらないっていわれちゃった……");
                        return;
                    }
                }

                let mut target = std::io::Cursor::new(Vec::new());

                let encoder = PngEncoder::new_with_quality(
                    &mut target,
                    CompressionType::Fast,
                    FilterType::Adaptive,
                );

                encoder
                    .write_image(
                        &pixels.data,
                        pixels.width,
                        pixels.height,
                        image::ExtendedColorType::Rgba8,
                    )
                    .unwrap();

                let target: Vec<u8> = target.into_inner();

                println!("Encoded");

                match enc_context {
                    EncodeContext::ScreenShot(n) => {
                        let _ = std::fs::File::create(format!("screenshot-{n}.png"))
                            .unwrap()
                            .write(&target)
                            .unwrap();
                    }
                    EncodeContext::Web(socket) => {
                        let 相手はもういらないかもしれない = socket.send(target);

                        if 相手はもういらないかもしれない.is_err() {
                            println!("えんこーどまでしたのにー…むきーっ！");
                        }
                    }
                }
            });
        })
        .unwrap();

    Ok(())
}

pub fn calculate_bounding_box(areas: &[u32]) -> BoundingBox<GeoDegree> {
    areas
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
        )
}

fn calculate_map_scale(bounding_box: BoundingBox<Screen>, aspect_ratio: f32) -> f32 {
    let x_scale = 1.0 / bounding_box.size().x;
    let y_scale = 1.0 / bounding_box.size().y * aspect_ratio;

    f32::min(f32::min(x_scale, y_scale) * 2.0, MAXIMUM_SCALE) / SCALE_FACTOR
}
