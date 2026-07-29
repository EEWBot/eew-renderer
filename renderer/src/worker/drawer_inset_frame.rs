use crate::worker::fonts::{Font, Offset, Origin};
use crate::worker::vertex::{ShapeUniform, ShapeVertex};
use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::{Surface, VertexBuffer};
use rusttype::Scale;
use std::ops::DerefMut;

pub fn draw_background<F: ?Sized + Facade, S: ?Sized + Surface>(
    frame_context: &FrameContext<F, S>,
) {
    draw_quad(
        frame_context,
        (-1.0, -1.0, 1.0, 1.0),
        frame_context.theme.inset_background_color,
    );
}

pub fn draw_border_and_label<F: ?Sized + Facade, S: ?Sized + Surface>(
    frame_context: &FrameContext<F, S>,
    label: &str,
) {
    let theme = frame_context.theme;
    let image_size = frame_context.image_size.to_f32();

    let width_x = 2.0 * theme.inset_border_width / image_size.x();
    let width_y = 2.0 * theme.inset_border_width / image_size.y();

    let strips = [
        (-1.0, -1.0, -1.0 + width_x, 1.0), // 左
        (1.0 - width_x, -1.0, 1.0, 1.0),   // 右
        (-1.0, 1.0 - width_y, 1.0, 1.0),   // 上
        (-1.0, -1.0, 1.0, -1.0 + width_y), // 下
    ];
    let vertices: Vec<ShapeVertex> = strips
        .iter()
        .flat_map(|&(left, bottom, right, top)| {
            [
                [left, bottom],
                [right, bottom],
                [left, top],
                [right, bottom],
                [right, top],
                [left, top],
            ]
        })
        .map(|position| ShapeVertex { position })
        .collect();
    let vertices = VertexBuffer::dynamic(frame_context.facade, &vertices).unwrap();

    frame_context
        .resources
        .shader
        .shape
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            &vertices,
            NoIndices(PrimitiveType::TrianglesList),
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

fn draw_quad<F: ?Sized + Facade, S: ?Sized + Surface>(
    frame_context: &FrameContext<F, S>,
    (left, bottom, right, top): (f32, f32, f32, f32),
    color: [f32; 4],
) {
    let shape = [
        ShapeVertex {
            position: [left, bottom],
        },
        ShapeVertex {
            position: [right, bottom],
        },
        ShapeVertex {
            position: [left, top],
        },
        ShapeVertex {
            position: [right, top],
        },
    ];
    let shape = VertexBuffer::dynamic(frame_context.facade, &shape).unwrap();

    frame_context
        .resources
        .shader
        .shape
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            &shape,
            NoIndices(PrimitiveType::TriangleStrip),
            &ShapeUniform { color },
            frame_context.draw_parameters,
        )
        .unwrap();
}
