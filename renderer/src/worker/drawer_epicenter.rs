use crate::worker::frame::{Frame, StagedSprites};
use crate::worker::pipelines::SpriteInstance;
use renderer_types::{GeoDegree, Vertex};

pub fn prepare(frame: &mut Frame, epicenters: &[Vertex<GeoDegree>]) -> Option<StagedSprites> {
    let instances: Vec<_> = epicenters
        .iter()
        .map(|epicenter| SpriteInstance {
            center: epicenter.to_mercator().into(),
            uv_rect: [0.0, 0.0, 1.0, 1.0],
        })
        .collect();

    frame.stage_sprites(&instances)
}

pub fn draw(frame: &mut Frame, markers: Option<&StagedSprites>, icon_ratio_in_y_axis: f32) {
    let Some(markers) = markers else {
        return;
    };

    let half_size = [
        icon_ratio_in_y_axis * frame.view.aspect_ratio(),
        icon_ratio_in_y_axis,
    ];

    frame.draw_sprites(&frame.resources.epicenter_texture, markers, half_size);
}
