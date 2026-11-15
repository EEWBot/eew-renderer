use crate::worker::frame::Frame;
use crate::worker::pipelines::SpriteInstance;

/// アイコン画像を 64x64 の格子とみなしたときの、アイコン1つ分の間隔
const ICON_PITCH: f32 = 21.0 / 64.0;
/// 隣のアイコンがにじまないよう、各アイコンの外周を切り落とす幅
const ICON_MARGIN: f32 = 1.0 / 64.0;

/// 震度アイコンは画像の左上から3列で並んでいる。(左, 上, 右, 下) を返す。
fn 震度_to_uv_rect(震度_i: usize) -> [f32; 4] {
    let column = (震度_i % 3) as f32;
    let row = (震度_i / 3) as f32;

    [
        ICON_PITCH * column + ICON_MARGIN,
        ICON_PITCH * row + ICON_MARGIN,
        ICON_PITCH * (column + 1.0),
        ICON_PITCH * (row + 1.0),
    ]
}

pub fn draw_all(frame: &mut Frame, earthquake_payload: &crate::frame_context::EarthquakePayload) {
    // 震度の小さい順に描くので、大きい震度のアイコンが上に重なる
    let instances: Vec<_> = earthquake_payload
        .area_intensities
        .iter()
        .flat_map(|(震度, area_codes)| {
            let uv_rect = 震度_to_uv_rect(震度 as usize);

            area_codes.iter().filter_map(move |code| {
                let nearest_station_coord =
                    renderer_assets::QueryInterface::query_rendering_center_by_area(*code)?;

                Some(SpriteInstance {
                    center: nearest_station_coord.to_mercator().into(),
                    uv_rect,
                })
            })
        })
        .collect();

    let half_size = [
        super::ICON_RATIO_IN_Y_AXIS * frame.view.aspect_ratio(),
        super::ICON_RATIO_IN_Y_AXIS,
    ];

    frame.sprites(&frame.resources.intensity_texture, &instances, half_size);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_rect_follows_the_grid() {
        assert_eq!(
            震度_to_uv_rect(0),
            [1.0 / 64.0, 1.0 / 64.0, 21.0 / 64.0, 21.0 / 64.0]
        );
        // 震度5強: 2行目の3列目
        assert_eq!(
            震度_to_uv_rect(5),
            [43.0 / 64.0, 22.0 / 64.0, 63.0 / 64.0, 42.0 / 64.0]
        );
    }
}
