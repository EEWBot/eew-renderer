use crate::worker::vertex::{ShapeUniform, ShapeVertex, TsunamiUniform};
use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::texture::{
    ClientFormat, MipmapsOption, RawImage1d, UncompressedUintFormat, UnsignedTexture1d,
};
use glium::{Surface, VertexBuffer};
use renderer_assets::QueryInterface;
use std::borrow::Cow;
use std::ops::DerefMut;
use glium::index::{NoIndices, PrimitiveType};
use rusttype::Scale;
use crate::model::津波情報;
use crate::worker::fonts::{Font, Offset, Origin};

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface>(
    frame_context: &FrameContext<F, S>,
    rendering_context: &crate::rendering_context::Tsunami,
) {
    let facade = frame_context.facade;
    let resources = frame_context.resources;
    let offset = frame_context.offset;
    let scale = frame_context.scale;
    let dimension = {
        let dimension = frame_context.dimension();
        [dimension.0 as f32, dimension.1 as f32]
    };
    let draw_parameters = frame_context.draw_parameters;
    let theme = frame_context.theme;

    let area_code_count = QueryInterface::tsunami_area_code_count();
    println!("AreaCodeCount: {area_code_count}");

    let mut levels = vec![0_u8; area_code_count];
    rendering_context
        .forecast_levels
        .iter()
        .for_each(|(level, areas)| {
            areas.iter().for_each(|area| {
                levels
                    [QueryInterface::tsunami_area_code_to_internal_code(*area).unwrap() as usize] =
                    level as u8
            })
        });

    println!("{:?}", levels);
    let levels = RawImage1d {
        data: Cow::from(&levels),
        width: levels.len() as u32,
        format: ClientFormat::U8,
    };
    let levels = UnsignedTexture1d::with_format(
        facade,
        levels,
        UncompressedUintFormat::U8,
        MipmapsOption::NoMipmap,
    )
    .unwrap();

    resources
        .shader
        .tsunami
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            &frame_context.resources.buffer.tsunami_vertex,
            &frame_context.resources.buffer.tsunami_indices,
            &TsunamiUniform {
                dimension,
                offset: offset.to_slice(),
                zoom: scale,
                colors: theme.tsunami_colors,
                levels,
                line_width: theme.tsunami_width,
            },
            draw_parameters,
        )
        .unwrap();

    let mut forecast_levels = rendering_context
        .forecast_levels
        .iter()
        .fold(Vec::<津波情報>::new(), |mut v, (level, entries)| {
            if !entries.is_empty() { v.push(level) }
            v
        });
    forecast_levels.sort();

    forecast_levels
        .iter()
        .enumerate()
        .for_each(|(i, forecast_level)| {
            let (shape, text_origin) = calculate_legend_position(dimension, i);
            let shape = VertexBuffer::dynamic(facade, &shape).unwrap();

            let color = match forecast_level {
                津波情報::津波予報 => theme.tsunami_colors.forecast,
                津波情報::津波注意報 => theme.tsunami_colors.advisory,
                津波情報::津波警報 => theme.tsunami_colors.warning,
                津波情報::大津波警報 => theme.tsunami_colors.major_warning,
            };
            let color = [color[0], color[1], color[2], 1.0];

            frame_context
                .resources
                .shader
                .shape
                .draw(
                    frame_context.surface.borrow_mut().deref_mut(),
                    &shape,
                    NoIndices(PrimitiveType::TriangleStrip),
                    &ShapeUniform {
                        color,
                    },
                    draw_parameters,
                )
                .unwrap();

            frame_context
                .font_manager
                .borrow_mut()
                .deref_mut()
                .draw_text(
                    &forecast_level.to_string(),
                    Font::BizUDPGothicBold,
                    theme.tsunami_legend_color,
                    Scale::uniform(22.0),
                    Offset::new(Origin::RightDown, Origin::LeftUp, text_origin.0, text_origin.1),
                    frame_context.dimension(),
                    resources,
                    facade,
                    frame_context.surface.borrow_mut().deref_mut(),
                    draw_parameters,
                );
        })
}

fn calculate_legend_position(dimension: [f32; 2], index: usize) -> ([ShapeVertex; 4], (i32, i32)) {
    let text_origin = (-300, -240 - 26 * index as i32);

    let x_origin = 1.0 - 300.0 / (dimension[0] / 2.0);
    let y_origin = -1.0 + (240.0 + 26.0 * index as f32) / (dimension[1] / 2.0);
    let shape_text_gap = 15.0 / (dimension[0] / 2.0);
    let shape_shape_gap = 10.4 / (dimension[1] / 2.0);
    let shape_width = 46.8 / (dimension[0] / 2.0);
    let shape_height = 15.6 / (dimension[1] / 2.0);

    let shape_left = x_origin - shape_text_gap - shape_width;
    let shape_right = x_origin - shape_text_gap;
    let shape_top = y_origin - shape_shape_gap / 2.0;
    let shape_bottom = y_origin - shape_shape_gap / 2.0 - shape_height;

    let shape = [
        ShapeVertex { position: [shape_left, shape_bottom] },
        ShapeVertex { position: [shape_right, shape_bottom] },
        ShapeVertex { position: [shape_left, shape_top] },
        ShapeVertex { position: [shape_right, shape_top] },
    ];

    println!("y_origin: {y_origin}, shape: {:?}", shape);

    (shape, text_origin)
}
