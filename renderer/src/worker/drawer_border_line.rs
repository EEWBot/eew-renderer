use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::{uniform, Surface};

const PREFECTURAL_BORDER_WIDTH: f32 = 5.0;
const AREA_BORDER_WIDTH: f32 = 2.0;

const PREFECTURAL_BORDER_COLOR: [f32; 3] = [0.35, 0.25, 0.19];
const AREA_BORDER_COLOR: [f32; 3] = [0.35, 0.25, 0.19];

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface>(frame_context: &FrameContext<F, S>) {
    let mut params = frame_context.draw_parameters.clone();
    params.line_width = Some(AREA_BORDER_WIDTH);
    
    let resources = frame_context.resources;
    let scale = frame_context.scale;
    let aspect_ratio = frame_context.aspect_ratio();
    let offset = frame_context.offset.to_slice();

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
                color: AREA_BORDER_COLOR,
            },
            &params,
        )
        .unwrap();

    let mut params = params.clone();
    params.line_width = Some(PREFECTURAL_BORDER_WIDTH);

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
                color: PREFECTURAL_BORDER_COLOR,
            },
            &params,
        )
        .unwrap();
}
