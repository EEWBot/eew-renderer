use std::ops::DerefMut;
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::{Surface, VertexBuffer};
use crate::rendering_context::HasEpicenter;
use crate::worker::FrameContext;
use crate::worker::vertex::{EpicenterUniform, EpicenterVertex};

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface, C: HasEpicenter>(frame_context: &FrameContext<F, S>, rendering_context: &C) {
    let facade = frame_context.facade;
    let resources = frame_context.resources;
    let aspect_ratio = frame_context.aspect_ratio();
    let offset = frame_context.offset;
    let scale = frame_context.scale;
    let draw_parameters = frame_context.draw_parameters;

    if let Some(epicenter) = rendering_context.epicenter() {
        let epicenter_data = VertexBuffer::dynamic(
            facade,
            &[EpicenterVertex {
                position: epicenter.to_slice()
            }],
        )
            .unwrap();

        resources
            .shader
            .epicenter
            .draw(
                frame_context.surface.borrow_mut().deref_mut(),
                &epicenter_data,
                NoIndices(PrimitiveType::Points),
                &EpicenterUniform {
                    aspect_ratio,
                    offset: offset.to_slice(),
                    zoom: scale,
                    icon_ratio_in_y_axis: super::ICON_RATIO_IN_Y_AXIS,
                    texture_sampler: &resources.texture.epicenter,
                },
                draw_parameters,
            )
            .unwrap();
    }
}
