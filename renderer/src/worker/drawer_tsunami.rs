use glium::backend::Facade;
use glium::Surface;
use glium::texture::{ClientFormat, MipmapsOption, RawImage1d, Texture1d, UncompressedFloatFormat};
use std::borrow::Cow;
use std::ops::DerefMut;
use crate::worker::FrameContext;
use crate::worker::vertex::TsunamiUniform;

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface>(frame_context: &FrameContext<F, S>, rendering_context: &crate::rendering_context::Tsunami) {
    let facade = frame_context.facade;
    let resources = frame_context.resources;
    let offset = frame_context.offset;
    let scale = frame_context.scale;
    let dimension = {
        let dimension = frame_context.dimension();
        [dimension.0 as f32, dimension.1 as f32]
    };
    let draw_parameters = frame_context.draw_parameters;

    let mut levels = [0_u8; 1024];
    rendering_context
        .forecast_levels
        .iter()
        .for_each(|(level, areas)| {
            areas.iter().for_each(|area| levels[*area as usize] = level as u8)
        });
    let levels = RawImage1d {
        data: Cow::from(&levels),
        width: levels.len() as u32,
        format: ClientFormat::U8,
    };
    let levels = Texture1d::with_format(
        facade,
        levels,
        UncompressedFloatFormat::U8,
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
            &TsunamiUniform::new(
                facade,
                dimension,
                offset.to_slice(),
                scale,
                frame_context.theme.tsunami_colors,
                levels,
                frame_context.theme.tsunami_width,
            ),
            draw_parameters,
        )
        .unwrap();
}
