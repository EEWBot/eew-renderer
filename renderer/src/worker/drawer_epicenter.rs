use crate::worker::frame::Frame;
use crate::worker::pipelines::SpriteInstance;
use renderer_types::{GeoDegree, Vertex};

/// 震央のマーカーを描く。`icon_ratio_in_y_axis` は、ビューポートの高さの半分に対するアイコンの半分の高さ。
pub fn draw(frame: &mut Frame, epicenters: &[Vertex<GeoDegree>], icon_ratio_in_y_axis: f32) {
    let instances: Vec<_> = epicenters
        .iter()
        .map(|epicenter| SpriteInstance {
            center: epicenter.to_mercator().into(),
            uv_rect: [0.0, 0.0, 1.0, 1.0],
        })
        .collect();

    let half_size = [
        icon_ratio_in_y_axis * frame.view.aspect_ratio(),
        icon_ratio_in_y_axis,
    ];

    frame.sprites(&frame.resources.epicenter_texture, &instances, half_size);
}
