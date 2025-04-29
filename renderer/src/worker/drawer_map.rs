use crate::worker::vertex::{BorderLineUniform, MapUniform};
use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::Surface;
use std::ops::DerefMut;

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface>(frame_context: &FrameContext<F, S>) {
    let theme = frame_context.theme;
    let params = frame_context.draw_parameters;
    let resources = frame_context.resources;
    let scale = frame_context.scale;
    let aspect_ratio = frame_context.aspect_ratio();
    let offset = frame_context.offset.to_slice();
    let dimension = {
        let dimension = frame_context.dimension();
        [dimension.0 as f32, dimension.1 as f32]
    };

    resources
        .shader
        .map
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            &resources.buffer.vertex,
            &resources.buffer.map,
            &MapUniform {
                aspect_ratio,
                offset,
                zoom: scale,
                color: theme.ground_color,
            },
            &params,
        )
        .unwrap();

    resources
        .shader
        .border_line
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            &resources.buffer.vertex,
            resources.buffer.get_area_line_by_scale(scale).unwrap(),
            &BorderLineUniform {
                dimension,
                offset,
                zoom: scale,
                line_width: theme.area_border_width,
                color: theme.area_border_color,
            },
            &params,
        )
        .unwrap();

    resources
        .shader
        .border_line
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            &resources.buffer.vertex,
            resources.buffer.get_pref_line_by_scale(scale).unwrap(),
            &BorderLineUniform {
                dimension,
                offset,
                zoom: scale,
                line_width: theme.prefectural_border_width,
                color: theme.prefectural_border_color,
            },
            &params,
        )
        .unwrap();
}
