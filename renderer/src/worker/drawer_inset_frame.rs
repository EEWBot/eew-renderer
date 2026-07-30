use crate::worker::fonts::{Font, Offset, Origin};
use crate::worker::inset::BorderSides;
use crate::worker::vertex::{ShapeUniform, ShapeVertex};
use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::Surface;
use rusttype::Scale;
use std::ops::DerefMut;

fn push_quad(vertices: &mut Vec<ShapeVertex>, x0: f32, y0: f32, x1: f32, y1: f32) {
    let quad = [[x0, y0], [x1, y0], [x1, y1], [x0, y0], [x1, y1], [x0, y1]];
    vertices.extend(quad.map(|position| ShapeVertex { position }));
}

fn border_vertices(width_x: f32, width_y: f32, sides: &BorderSides) -> Vec<ShapeVertex> {
    let inner_left = -1.0 + width_x;
    let inner_right = 1.0 - width_x;
    let inner_bottom = -1.0 + width_y;
    let inner_top = 1.0 - width_y;

    let mut vertices = Vec::with_capacity(24);
    if sides.top {
        push_quad(&mut vertices, -1.0, inner_top, 1.0, 1.0);
    }
    if sides.bottom {
        push_quad(&mut vertices, -1.0, -1.0, 1.0, inner_bottom);
    }
    if sides.left {
        push_quad(&mut vertices, -1.0, -1.0, inner_left, 1.0);
    }
    if sides.right {
        push_quad(&mut vertices, inner_right, -1.0, 1.0, 1.0);
    }
    vertices
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
            &frame_context.resources.inset.background_vertex_buffer,
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
    sides: &BorderSides,
    label: &str,
) {
    let theme = frame_context.theme;
    let image_size = frame_context.camera.image_size.to_f32();

    let width_x = 2.0 * theme.inset_border_width / image_size.x();
    let width_y = 2.0 * theme.inset_border_width / image_size.y();

    let mut vertices = border_vertices(width_x, width_y, sides);
    if !vertices.is_empty() {
        vertices.resize(
            frame_context.resources.inset.border_vertex_buffer.len(),
            ShapeVertex {
                position: [0.0, 0.0],
            },
        );
        frame_context
            .resources
            .inset
            .border_vertex_buffer
            .write(&vertices);

        frame_context
            .resources
            .shader
            .shape
            .draw(
                frame_context.surface.borrow_mut().deref_mut(),
                &frame_context.resources.inset.border_vertex_buffer,
                NoIndices(PrimitiveType::TrianglesList),
                &ShapeUniform {
                    color: theme.inset_border_color,
                },
                frame_context.draw_parameters,
            )
            .unwrap();
    }

    frame_context
        .font_manager
        .borrow_mut()
        .deref_mut()
        .draw_text(
            label,
            Font::BizUDPGothicBold,
            theme.inset_label_color,
            Scale::uniform(theme.inset_label_font_size),
            Offset::new(
                Origin::CenterDown,
                Origin::CenterDown,
                0,
                theme.inset_label_offset_y,
            ),
            frame_context.camera.image_size.into(),
            frame_context.resources,
            frame_context.facade,
            frame_context.surface.borrow_mut().deref_mut(),
            frame_context.draw_parameters,
        );
}
