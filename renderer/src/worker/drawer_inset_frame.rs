use crate::worker::fonts::{Font, Offset, Origin};
use crate::worker::vertex::{ShapeUniform, ShapeVertex};
use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::Surface;
use rusttype::Scale;
use std::ops::DerefMut;

pub(in crate::worker) fn border_vertices(width_x: f32, width_y: f32) -> [ShapeVertex; 10] {
    let inner_left = -1.0 + width_x;
    let inner_right = 1.0 - width_x;
    let inner_bottom = -1.0 + width_y;
    let inner_top = 1.0 - width_y;

    [
        // 左上
        ShapeVertex {
            position: [-1.0, 1.0],
        },
        ShapeVertex {
            position: [inner_left, inner_top],
        },
        // 右上
        ShapeVertex {
            position: [1.0, 1.0],
        },
        ShapeVertex {
            position: [inner_right, inner_top],
        },
        // 右下
        ShapeVertex {
            position: [1.0, -1.0],
        },
        ShapeVertex {
            position: [inner_right, inner_bottom],
        },
        // 左下
        ShapeVertex {
            position: [-1.0, -1.0],
        },
        ShapeVertex {
            position: [inner_left, inner_bottom],
        },
        // 始点に戻して左辺を閉じる
        ShapeVertex {
            position: [-1.0, 1.0],
        },
        ShapeVertex {
            position: [inner_left, inner_top],
        },
    ]
}

pub fn draw_background<F: ?Sized + Facade, S: ?Sized + Surface>(
    frame_context: &FrameContext<F, S>,
) {
    frame_context
        .resources
        .shader
        .shape
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            &frame_context.resources.inset_background_vertex_buffer,
            NoIndices(PrimitiveType::TriangleStrip),
            &ShapeUniform {
                color: frame_context.theme.inset_background_color,
            },
            frame_context.draw_parameters,
        )
        .unwrap();
}

pub fn draw_border_and_label<F: ?Sized + Facade, S: ?Sized + Surface>(
    frame_context: &FrameContext<F, S>,
    label: &str,
) {
    let theme = frame_context.theme;
    let image_size = frame_context.image_size.to_f32();

    let width_x = 2.0 * theme.inset_border_width / image_size.x();
    let width_y = 2.0 * theme.inset_border_width / image_size.y();

    frame_context
        .resources
        .border_vertex_buffer
        .write(&border_vertices(width_x, width_y));

    frame_context
        .resources
        .shader
        .shape
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            &frame_context.resources.border_vertex_buffer,
            NoIndices(PrimitiveType::TriangleStrip),
            &ShapeUniform {
                color: theme.inset_border_color,
            },
            frame_context.draw_parameters,
        )
        .unwrap();

    frame_context
        .font_manager
        .borrow_mut()
        .deref_mut()
        .draw_text(
            label,
            Font::BizUDPGothicBold,
            frame_context.theme.tsunami_legend_color,
            Scale::uniform(22.0),
            Offset::new(Origin::CenterDown, Origin::CenterDown, 0, -6),
            frame_context.image_size.into(),
            frame_context.resources,
            frame_context.facade,
            frame_context.surface.borrow_mut().deref_mut(),
            frame_context.draw_parameters,
        );
}
