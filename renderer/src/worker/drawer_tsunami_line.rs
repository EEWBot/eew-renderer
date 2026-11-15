use crate::worker::frame::Frame;
use crate::worker::pipelines::{TsunamiUniform, TSUNAMI_LEVEL_CAPACITY};
use crate::worker::theme::Theme;
use renderer_assets::QueryInterface;
use renderer_types::InsetRegion;

/// 予報区ごとの発表レベルと線の色を、シェーダーに渡す形にまとめる。
pub fn build_uniform<T>(theme: &Theme, tsunami_payload: &T) -> TsunamiUniform
where
    T: crate::frame_context::HasTsunamiForecastLevels,
{
    assert!(
        QueryInterface::tsunami_area_code_count() <= TSUNAMI_LEVEL_CAPACITY,
        "津波予報区の数がシェーダーの上限を超えている"
    );

    let mut levels = [0_u32; TSUNAMI_LEVEL_CAPACITY];

    for (level, areas) in tsunami_payload.forecast_levels().iter() {
        for area in areas {
            let code = QueryInterface::tsunami_area_code_to_internal_code(*area).unwrap();
            levels[code as usize] = level as u32;
        }
    }

    let opaque = |color: [f32; 3]| [color[0], color[1], color[2], 1.0];

    TsunamiUniform {
        colors: [
            [0.0; 4],
            opaque(theme.tsunami_colors.forecast),
            opaque(theme.tsunami_colors.advisory),
            opaque(theme.tsunami_colors.warning),
            opaque(theme.tsunami_colors.major_warning),
        ],
        levels,
    }
}

pub fn draw(frame: &mut Frame, region: InsetRegion) {
    frame.tsunami_lines(
        frame.resources.tsunami_range(region),
        frame.theme.tsunami_width,
    );
}
