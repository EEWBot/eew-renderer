use crate::frame_context::HasEpicenter;
use crate::worker::vertex::{EpicenterUniform, EpicenterVertex};
use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::{Surface, VertexBuffer};
use renderer_types::{GeoDegree, Vertex};
use std::ops::DerefMut;

pub fn create_vertex_buffer<F: ?Sized + Facade>(
    facade: &F,
    epicenters: &[Vertex<GeoDegree>],
) -> Option<VertexBuffer<EpicenterVertex>> {
    if epicenters.is_empty() {
        return None;
    }

    let vertices = epicenters
        .iter()
        .map(|epicenter| EpicenterVertex {
            position: (*epicenter).into(),
        })
        .collect::<Vec<_>>();

    Some(VertexBuffer::dynamic(facade, &vertices).unwrap())
}

pub fn draw_vertex_buffer<F: ?Sized + Facade, S: ?Sized + Surface>(
    frame_context: &FrameContext<F, S>,
    vertex_buffer: &VertexBuffer<EpicenterVertex>,
    icon_ratio_in_y_axis: f32,
) {
    let resources = frame_context.resources;

    resources
        .shader
        .epicenter
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            vertex_buffer,
            NoIndices(PrimitiveType::Points),
            &EpicenterUniform {
                aspect_ratio: frame_context.image_size.aspect_ratio(),
                offset: frame_context.offset.into(),
                zoom: frame_context.scale,
                icon_ratio_in_y_axis,
                texture_sampler: &resources.texture.epicenter,
            },
            frame_context.draw_parameters,
        )
        .unwrap();
}

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface, C: HasEpicenter>(
    frame_context: &FrameContext<F, S>,
    rendering_context: &C,
) {
    if let Some(vertex_buffer) =
        create_vertex_buffer(frame_context.facade, rendering_context.epicenter())
    {
        draw_vertex_buffer(frame_context, &vertex_buffer, super::ICON_RATIO_IN_Y_AXIS);
    }
}
