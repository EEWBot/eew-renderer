use crate::rendering_context::HasEpicenter;
use crate::worker::vertex::{EpicenterUniform, EpicenterVertex};
use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::{Surface, VertexBuffer};
use std::ops::DerefMut;

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface, C: HasEpicenter>(
    frame_context: &FrameContext<F, S>,
    rendering_context: &C,
) {
    let facade = frame_context.facade;
    let resources = frame_context.resources;
    let aspect_ratio = frame_context.image_size.aspect_ratio();
    let offset = frame_context.offset;
    let scale = frame_context.scale;
    let draw_parameters = frame_context.draw_parameters;

    if rendering_context.epicenter().is_empty() {
        return;
    }

    let vb = rendering_context
        .epicenter()
        .iter()
        .map(|epicenter| EpicenterVertex {
            position: (*epicenter).into(),
        })
        .collect::<Vec<_>>();
    let vb = VertexBuffer::dynamic(facade, &vb).unwrap();

    resources
        .shader
        .epicenter
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            &vb,
            NoIndices(PrimitiveType::Points),
            &EpicenterUniform {
                aspect_ratio,
                offset: offset.into(),
                zoom: scale,
                icon_ratio_in_y_axis: super::ICON_RATIO_IN_Y_AXIS,
                texture_sampler: &resources.texture.epicenter,
            },
            draw_parameters,
        )
        .unwrap();
}
