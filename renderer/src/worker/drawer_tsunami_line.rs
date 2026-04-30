use crate::worker::vertex::TsunamiUniform;
use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::texture::{
    ClientFormat, MipmapsOption, RawImage1d, UncompressedUintFormat, UnsignedTexture1d,
};
use glium::Surface;
use renderer_assets::QueryInterface;
use std::borrow::Cow;
use std::ops::DerefMut;

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface>(
    frame_context: &FrameContext<F, S>,
    tsunami_payload: &crate::frame_context::TsunamiFirstPayload,
) {
    let facade = frame_context.facade;
    let resources = frame_context.resources;
    let offset = frame_context.offset;
    let scale = frame_context.scale;
    let draw_parameters = frame_context.draw_parameters;
    let theme = frame_context.theme;

    let area_code_count = QueryInterface::internal_tsunami_area_code_count();
    // println!("AreaCodeCount: {area_code_count}");

    let mut levels = vec![0_u8; area_code_count];

    tsunami_payload
        .forecast_levels
        .iter()
        .for_each(|(level, areas)| {
            areas.iter().for_each(|area| {
                levels[QueryInterface::津波予報区_to_internal_tsunami_area_code(*area)
                    .unwrap()
                    .0 as usize] = level as u8
            })
        });

    // println!("{:?}", levels);
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
                dimension: frame_context.image_size.to_f32().into(),
                offset: offset.into(),
                zoom: scale,
                colors: theme.tsunami_colors,
                levels,
                line_width: theme.tsunami_width,
            },
            draw_parameters,
        )
        .unwrap();
}
