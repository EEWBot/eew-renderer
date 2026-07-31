use super::camera::Camera;
use super::inset::NotchLine;
use renderer_types::{GeoDegree, Vertex};

pub struct ResolvedNotch {
    pub a: [f32; 2],
    pub b: [f32; 2],
}

fn to_ndc(camera: &Camera, geo: Vertex<GeoDegree>) -> [f32; 2] {
    geo.to_mercator()
        .to_screen(camera.offset, camera.scale)
        .into()
}

fn clip_line_to_square(origin: [f32; 2], dir: [f32; 2]) -> ([f32; 2], [f32; 2]) {
    let mut t_enter = f32::NEG_INFINITY;
    let mut t_exit = f32::INFINITY;
    for axis in 0..2 {
        let (o, d) = (origin[axis], dir[axis]);
        if d.abs() <= f32::EPSILON {
            continue;
        }
        let t0 = (-1.0 - o) / d;
        let t1 = (1.0 - o) / d;
        t_enter = t_enter.max(t0.min(t1));
        t_exit = t_exit.min(t0.max(t1));
    }
    debug_assert!(t_enter.is_finite() && t_exit.is_finite() && t_enter < t_exit);
    (
        [origin[0] + t_enter * dir[0], origin[1] + t_enter * dir[1]],
        [origin[0] + t_exit * dir[0], origin[1] + t_exit * dir[1]],
    )
}

pub fn resolve(camera: &Camera, notch: &NotchLine) -> ResolvedNotch {
    let a = to_ndc(camera, notch.from);
    let b = to_ndc(camera, notch.to);
    let dir = [b[0] - a[0], b[1] - a[1]];
    let (a, b) = clip_line_to_square(a, dir);
    ResolvedNotch { a, b }
}

pub fn kept_polygon(notch: &ResolvedNotch) -> Vec<[f32; 2]> {
    let square = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];

    let side = |p: [f32; 2]| -> f32 {
        (notch.b[0] - notch.a[0]) * (p[1] - notch.a[1])
            - (notch.b[1] - notch.a[1]) * (p[0] - notch.a[0])
    };

    let keep_sign = side([0.0, 0.0]).signum();

    let mut result = Vec::with_capacity(5);
    for i in 0..square.len() {
        let current = square[i];
        let next = square[(i + 1) % square.len()];
        let current_inside = side(current) * keep_sign >= 0.0;
        let next_inside = side(next) * keep_sign >= 0.0;

        if current_inside {
            result.push(current);
        }
        if current_inside != next_inside {
            let d_current = side(current);
            let d_next = side(next);
            let t = d_current / (d_current - d_next);
            result.push([
                current[0] + t * (next[0] - current[0]),
                current[1] + t * (next[1] - current[1]),
            ]);
        }
    }
    result
}
