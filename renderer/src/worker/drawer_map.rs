use std::ops::DerefMut;
use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::{uniform, Surface};
use crate::worker::vertex::MapUniform;

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface>(frame_context: &FrameContext<F, S>) {
    let mut theme = frame_context.theme;

    let mut params = frame_context.draw_parameters.clone();
    params.line_width = Some(theme.area_border_width);
    
    let resources = frame_context.resources;
    let scale = frame_context.scale;
    let aspect_ratio = frame_context.aspect_ratio();
    let offset = frame_context.offset.to_slice();

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

    frame_context
        .surface
        .borrow_mut()
        .draw(
            &resources.buffer.vertex,
            resources.buffer.get_area_line_by_scale(scale).unwrap(),
            &resources.shader.border_line,
            &uniform! {
                aspect_ratio: aspect_ratio,
                offset: offset,
                zoom: scale,
                color: theme.area_border_color,
            },
            &params,
        )
        .unwrap();

    let mut params = params.clone();
    params.line_width = Some(theme.prefectural_border_width);

    frame_context
        .surface
        .borrow_mut()
        .draw(
            &resources.buffer.vertex,
            resources.buffer.get_pref_line_by_scale(scale).unwrap(),
            &resources.shader.border_line,
            &uniform! {
                aspect_ratio: aspect_ratio,
                offset: offset,
                zoom: scale,
                color: theme.prefectural_border_color,
            },
            &params,
        )
        .unwrap();
}
