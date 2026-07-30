use crate::worker::vertex::TsunamiUniform;
use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::texture::{
    ClientFormat, MipmapsOption, RawImage1d, UncompressedUintFormat, UnsignedTexture1d,
};
use glium::Surface;
use renderer_assets::QueryInterface;
use renderer_types::InsetRegion;
use std::borrow::Cow;
use std::ops::DerefMut;

pub fn build_levels_texture<F: ?Sized + Facade, T>(
    facade: &F,
    tsunami_payload: &T,
) -> UnsignedTexture1d
where
    T: crate::frame_context::HasTsunamiForecastLevels,
{
    let area_code_count = QueryInterface::tsunami_area_code_count();

    let mut levels = vec![0_u8; area_code_count];

    tsunami_payload
        .forecast_levels()
        .iter()
        .for_each(|(level, areas)| {
            areas.iter().for_each(|area| {
                levels
                    [QueryInterface::tsunami_area_code_to_internal_code(*area).unwrap() as usize] =
                    level as u8
            })
        });

    let levels = RawImage1d {
        data: Cow::from(&levels),
        width: levels.len() as u32,
        format: ClientFormat::U8,
    };

    UnsignedTexture1d::with_format(
        facade,
        levels,
        UncompressedUintFormat::U8,
        MipmapsOption::NoMipmap,
    )
    .unwrap()
}

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface>(
    frame_context: &FrameContext<F, S>,
    region: InsetRegion,
    levels: &UnsignedTexture1d,
) {
    let resources = frame_context.resources;
    let offset = frame_context.camera.offset;
    let scale = frame_context.camera.scale;
    let draw_parameters = frame_context.draw_parameters;
    let theme = frame_context.theme;

    resources
        .shader
        .tsunami
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            &frame_context.resources.buffer.tsunami_vertex,
            frame_context
                .resources
                .buffer
                .get_tsunami_indices_by_region(region),
            &TsunamiUniform {
                dimension: frame_context.camera.image_size.to_f32().into(),
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
