use crate::model::津波情報;
use crate::worker::fonts::{Font, Offset, Origin};
use crate::worker::frame::Frame;
use renderer_types::Size;

pub fn draw<T>(frame: &mut Frame, tsunami_payload: &T)
where
    T: crate::frame_context::HasTsunamiForecastLevels,
{
    let theme = frame.theme;
    let image_size = frame.view.size;

    let mut forecast_levels: Vec<津波情報> = tsunami_payload
        .forecast_levels()
        .iter()
        .filter(|(_, entries)| !entries.is_empty())
        .map(|(level, _)| level)
        .collect();
    forecast_levels.sort();

    for (i, forecast_level) in forecast_levels.iter().enumerate() {
        let (shape, text_origin) = calculate_legend_position(image_size, i);

        let color = match forecast_level {
            津波情報::津波予報 => theme.tsunami_colors.forecast,
            津波情報::津波注意報 => theme.tsunami_colors.advisory,
            津波情報::津波警報 => theme.tsunami_colors.warning,
            津波情報::大津波警報 => theme.tsunami_colors.major_warning,
        };

        frame.fill_shape(&shape, [color[0], color[1], color[2], 1.0]);

        frame.text(
            &forecast_level.to_string(),
            Font::BizUDPGothicBold,
            theme.tsunami_legend_color,
            22.0,
            Offset::new(
                Origin::RightDown,
                Origin::LeftUp,
                text_origin.0,
                text_origin.1,
            ),
        );
    }
}

/// 凡例の色見本 (三角形リスト) と、ラベルの位置。
fn calculate_legend_position(image_size: Size<u32>, index: usize) -> ([[f32; 2]; 6], (i32, i32)) {
    let image_size = image_size.to_f32();
    let text_origin = (-300, -76 - 26 * index as i32);

    let x_origin = 1.0 - 300.0 / (image_size.x() / 2.0);
    let y_origin = -1.0 + (76.0 + 26.0 * index as f32) / (image_size.y() / 2.0);
    let shape_text_gap = 15.0 / (image_size.x() / 2.0);
    let shape_shape_gap = 10.4 / (image_size.y() / 2.0);
    let shape_width = 46.8 / (image_size.x() / 2.0);
    let shape_height = 15.6 / (image_size.y() / 2.0);

    // 縁がにじまないよう、画素の境界に揃える
    let snap_x = |x: f32| (x * image_size.x() / 2.0).round() / (image_size.x() / 2.0);
    let snap_y = |y: f32| (y * image_size.y() / 2.0).round() / (image_size.y() / 2.0);

    let left = snap_x(x_origin - shape_text_gap - shape_width);
    let right = snap_x(x_origin - shape_text_gap);
    let top = snap_y(y_origin - shape_shape_gap / 2.0);
    let bottom = snap_y(y_origin - shape_shape_gap / 2.0 - shape_height);

    let shape = [
        [left, bottom],
        [right, bottom],
        [left, top],
        [left, top],
        [right, bottom],
        [right, top],
    ];

    (shape, text_origin)
}
