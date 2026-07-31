use crate::worker::fonts::{Font, Offset, Origin};
use crate::worker::inset::BorderSides;
use crate::worker::notch::ResolvedNotch;
use crate::worker::vertex::{ShapeUniform, ShapeVertex};
use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::draw_parameters::{StencilOperation, StencilTest};
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

    let side_bottom = if sides.bottom { inner_bottom } else { -1.0 };
    let side_top = if sides.top { inner_top } else { 1.0 };

    let mut vertices = Vec::with_capacity(30);
    if sides.top {
        push_quad(&mut vertices, -1.0, inner_top, 1.0, 1.0);
    }
    if sides.bottom {
        push_quad(&mut vertices, -1.0, -1.0, 1.0, inner_bottom);
    }
    if sides.left {
        push_quad(&mut vertices, -1.0, side_bottom, inner_left, side_top);
    }
    if sides.right {
        push_quad(&mut vertices, inner_right, side_bottom, 1.0, side_top);
    }
    vertices
}

pub fn draw_stencil_mask<F: ?Sized + Facade, S: ?Sized + Surface>(
    frame_context: &FrameContext<F, S>,
    notch: &ResolvedNotch,
) {
    let vertices: Vec<_> = notch
        .kept_polygon
        .as_slice()
        .iter()
        .map(|&position| ShapeVertex { position })
        .collect();

    let buffer = &frame_context.resources.inset.notch_mask_vertex_buffer;
    let slice = buffer.slice(0..vertices.len()).unwrap();
    slice.write(&vertices);

    let mut draw_parameters = frame_context.draw_parameters.clone();
    draw_parameters.color_mask = (false, false, false, false);
    draw_parameters.stencil =
        crate::worker::stencil_params(StencilTest::AlwaysPass, StencilOperation::Replace);

    frame_context
        .resources
        .shader
        .shape
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            slice,
            NoIndices(PrimitiveType::TriangleFan),
            &ShapeUniform {
                color: [0.0, 0.0, 0.0, 0.0],
            },
            &draw_parameters,
        )
        .unwrap();
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

fn push_notch_border(
    vertices: &mut Vec<ShapeVertex>,
    notch: &ResolvedNotch,
    width_x: f32,
    width_y: f32,
) {
    let [ax, ay] = notch.a;
    let [bx, by] = notch.b;

    let dir = [(bx - ax) / width_x, (by - ay) / width_y];
    let length = f32::hypot(dir[0], dir[1]);
    let normal = [-dir[1] / length, dir[0] / length];

    let mut offset = [normal[0] * width_x, normal[1] * width_y];
    if notch.side([ax + offset[0], ay + offset[1]]).signum() != notch.keep_sign() {
        offset = [-offset[0], -offset[1]];
    }

    let inner_a = [ax + offset[0], ay + offset[1]];
    let inner_b = [bx + offset[0], by + offset[1]];

    let quad = [notch.a, notch.b, inner_b, notch.a, inner_b, inner_a];
    vertices.extend(quad.map(|position| ShapeVertex { position }));
}

fn label_offset_x(notch: Option<&ResolvedNotch>, viewport_width: f32) -> i32 {
    let Some(notch) = notch else {
        return 0;
    };
    const EPSILON: f32 = 1e-3;
    let (min, max, count) = notch
        .kept_polygon
        .as_slice()
        .iter()
        .filter(|[_, y]| *y <= -1.0 + EPSILON)
        .fold(
            (f32::INFINITY, f32::NEG_INFINITY, 0usize),
            |(min, max, count), [x, _]| (min.min(*x), max.max(*x), count + 1),
        );
    if count < 2 {
        return 0;
    }

    ((min + max) / 2.0 * viewport_width / 2.0).round() as i32
}

pub fn draw_border_and_label<F: ?Sized + Facade, S: ?Sized + Surface>(
    frame_context: &FrameContext<F, S>,
    sides: &BorderSides,
    notch: Option<&ResolvedNotch>,
    label: &str,
) {
    let theme = frame_context.theme;
    let image_size = frame_context.camera.image_size.to_f32();

    let width_x = 2.0 * theme.inset_border_width / image_size.x();
    let width_y = 2.0 * theme.inset_border_width / image_size.y();

    let mut vertices = border_vertices(width_x, width_y, sides);
    if let Some(notch) = notch {
        push_notch_border(&mut vertices, notch, width_x, width_y);
    }

    if !vertices.is_empty() {
        let buffer = &frame_context.resources.inset.border_vertex_buffer;
        let slice = buffer.slice(0..vertices.len()).unwrap();
        slice.write(&vertices);

        frame_context
            .resources
            .shader
            .shape
            .draw(
                frame_context.surface.borrow_mut().deref_mut(),
                slice,
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
                label_offset_x(notch, image_size.x()),
                theme.inset_label_offset_y,
            ),
            frame_context.camera.image_size.into(),
            frame_context.resources,
            frame_context.facade,
            frame_context.surface.borrow_mut().deref_mut(),
            frame_context.draw_parameters,
        );
}
