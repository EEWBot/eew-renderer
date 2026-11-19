use crate::model::津波情報;
use crate::worker::frame::Frame;
use crate::worker::pipelines::{TsunamiUniform, TSUNAMI_LEVEL_CAPACITY};
use crate::worker::theme::Theme;
use renderer_assets::QueryInterface;
use renderer_types::InsetRegion;
use std::ops::Range;

pub type AreaLevels = [u32; TSUNAMI_LEVEL_CAPACITY];

pub fn area_levels<T>(tsunami_payload: &T) -> AreaLevels
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

    levels
}

pub fn build_uniform(theme: &Theme, levels: &AreaLevels) -> TsunamiUniform {
    let opaque = |color: [f32; 3]| [color[0], color[1], color[2], 1.0];

    TsunamiUniform {
        colors: [
            [0.0; 4],
            opaque(theme.tsunami_colors.forecast),
            opaque(theme.tsunami_colors.advisory),
            opaque(theme.tsunami_colors.warning),
            opaque(theme.tsunami_colors.major_warning),
        ],
        levels: *levels,
    }
}

/// 重大なレベルの線が隠れないよう、軽いレベルから順に描いて重ねる。
/// 発表のある予報区の線分だけを描き、発表のないレベルは描画自体を省く。
pub fn draw(frame: &mut Frame, region: InsetRegion, levels: &AreaLevels) {
    let areas = frame.resources.tsunami_area_ranges(region);
    let mut ranges: Vec<Range<u32>> = Vec::new();

    for level in DRAW_ORDER {
        let level = level as u32;

        ranges.clear();
        for (code, range) in areas {
            if levels[*code as usize] != level {
                continue;
            }

            // 線分列上で隣り合う予報区は1回の描画にまとめる
            match ranges.last_mut() {
                Some(last) if last.end == range.start => last.end = range.end,
                _ => ranges.push(range.clone()),
            }
        }

        if !ranges.is_empty() {
            frame.tsunami_lines(&ranges, frame.theme.tsunami_width, level);
        }
    }
}

const DRAW_ORDER: [津波情報; 4] = [
    津波情報::津波予報,
    津波情報::津波注意報,
    津波情報::津波警報,
    津波情報::大津波警報,
];
