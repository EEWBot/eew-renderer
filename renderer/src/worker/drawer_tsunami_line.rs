use crate::model::{津波情報, RenderingError};
use crate::worker::frame::Frame;
use crate::worker::theme::Theme;
use renderer_assets::QueryInterface;
use renderer_types::InsetRegion;
use std::ops::Range;

pub type AreaLevels = [Option<津波情報>; QueryInterface::TSUNAMI_AREA_CODE_COUNT];

pub fn area_levels<T>(tsunami_payload: &T) -> Result<AreaLevels, RenderingError>
where
    T: crate::frame_context::HasTsunamiForecastLevels,
{
    let mut levels = [None; QueryInterface::TSUNAMI_AREA_CODE_COUNT];

    for (level, areas) in tsunami_payload.forecast_levels().iter() {
        for area in areas {
            let code = QueryInterface::tsunami_area_code_to_internal_code(*area)
                .ok_or(RenderingError::UnknownTsunamiArea(area.0))?;
            levels[code as usize] = Some(level);
        }
    }

    Ok(levels)
}

fn line_color(theme: &Theme, level: 津波情報) -> [f32; 3] {
    match level {
        津波情報::津波予報 => theme.tsunami_colors.forecast,
        津波情報::津波注意報 => theme.tsunami_colors.advisory,
        津波情報::津波警報 => theme.tsunami_colors.warning,
        津波情報::大津波警報 => theme.tsunami_colors.major_warning,
    }
}

/// 重大なレベルの線が隠れないよう、軽いレベルから順に描いて重ねる。
/// 発表のある予報区の線分だけを描き、発表のないレベルは描画自体を省く。
pub fn draw(frame: &mut Frame, region: InsetRegion, levels: &AreaLevels) {
    let areas = frame.resources.tsunami_area_ranges(region);
    let mut ranges: Vec<Range<u32>> = Vec::new();

    for level in DRAW_ORDER {
        ranges.clear();
        for (code, range) in areas {
            if levels[*code as usize] != Some(level) {
                continue;
            }

            // 線分列上で隣り合う予報区は1回の描画にまとめる
            match ranges.last_mut() {
                Some(last) if last.end == range.start => last.end = range.end,
                _ => ranges.push(range.clone()),
            }
        }

        if !ranges.is_empty() {
            let color = line_color(frame.theme, level);
            frame.tsunami_lines(&ranges, frame.theme.tsunami_width, color);
        }
    }
}

const DRAW_ORDER: [津波情報; 4] = [
    津波情報::津波予報,
    津波情報::津波注意報,
    津波情報::津波警報,
    津波情報::大津波警報,
];
