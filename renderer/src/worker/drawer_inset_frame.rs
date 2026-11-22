use crate::worker::fonts::{Font, Offset, Origin};
use crate::worker::frame::Frame;
use crate::worker::inset::BorderSides;
use crate::worker::notch::ResolvedNotch;

/// ビューポート全体を覆う矩形 (三角形リスト)
const FULL_QUAD: [[f32; 2]; 6] = [
    [-1.0, -1.0],
    [1.0, -1.0],
    [1.0, 1.0],
    [-1.0, -1.0],
    [1.0, 1.0],
    [-1.0, 1.0],
];

fn push_quad(vertices: &mut Vec<[f32; 2]>, x0: f32, y0: f32, x1: f32, y1: f32) {
    vertices.extend([[x0, y0], [x1, y0], [x1, y1], [x0, y0], [x1, y1], [x0, y1]]);
}

fn border_vertices(width_x: f32, width_y: f32, sides: &BorderSides) -> Vec<[f32; 2]> {
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

/// 切り欠きで残す側の多角形 (凸) を三角形リストにする。
fn kept_polygon_triangles(notch: &ResolvedNotch) -> Vec<[f32; 2]> {
    let polygon = notch.kept_polygon.as_slice();

    (1..polygon.len().saturating_sub(1))
        .flat_map(|i| [polygon[0], polygon[i], polygon[i + 1]])
        .collect()
}

/// 切り欠きで残す側のステンシルを、現在の参照値で上書きする。
pub fn draw_stencil_mask(frame: &mut Frame, notch: &ResolvedNotch) {
    frame.write_stencil(&kept_polygon_triangles(notch));
}

pub fn draw_background(frame: &mut Frame) {
    frame.fill_shape(&FULL_QUAD, frame.theme.inset_background_color);
}

fn push_notch_border(
    vertices: &mut Vec<[f32; 2]>,
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

    vertices.extend([notch.a, notch.b, inner_b, notch.a, inner_b, inner_a]);
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

pub fn draw_border_and_label(
    frame: &mut Frame,
    sides: &BorderSides,
    notch: Option<&ResolvedNotch>,
    label: &str,
) {
    let theme = frame.theme;
    let image_size = frame.view.size.to_f32();

    let width_x = 2.0 * theme.inset_border_width / image_size.x();
    let width_y = 2.0 * theme.inset_border_width / image_size.y();

    let mut vertices = border_vertices(width_x, width_y, sides);
    if let Some(notch) = notch {
        push_notch_border(&mut vertices, notch, width_x, width_y);
    }

    frame.fill_shape(&vertices, theme.inset_border_color);

    frame.text(
        label,
        Font::BizUDPGothicBold,
        theme.inset_label_color,
        theme.inset_label_font_size,
        Offset::new(
            Origin::CenterDown,
            Origin::CenterDown,
            label_offset_x(notch, image_size.x()),
            theme.inset_label_offset_y,
        ),
    );
}
