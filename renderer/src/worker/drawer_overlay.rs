use crate::frame_context::HasTime;
use crate::worker::fonts::{Font, Offset, Origin};
use crate::worker::frame::Frame;
use crate::worker::pipelines::TexturedVertex;
use chrono_tz::Tz::Japan;
use renderer_types::Size;

const OVERLAY_OFFSET_PIXELS: u16 = 10;
const RIGHTS_NOTATION_RATIO_IN_Y_AXIS: f32 = 0.16;
const WATERMARK_RATIO_IN_Y_AXIS: f32 = 0.12;

/// オーバーレイ画像の中の、権利表記が占める縦方向の範囲 (上端, 下端)
const RIGHTS_NOTATION_V: (f32, f32) = (0.25, 0.5);
/// オーバーレイ画像の中の、透かしが占める縦方向の範囲 (上端, 下端)
const WATERMARK_V: (f32, f32) = (0.0, 0.25);

/// クリップ座標の矩形 (左下, 右上) に、テクスチャの横幅いっぱい・縦方向 `v` の範囲を貼る。
fn textured_quad(min: [f32; 2], max: [f32; 2], v: (f32, f32)) -> [TexturedVertex; 6] {
    let (top, bottom) = v;

    let left_bottom = TexturedVertex {
        position: [min[0], min[1]],
        uv: [0.0, bottom],
    };
    let right_bottom = TexturedVertex {
        position: [max[0], min[1]],
        uv: [1.0, bottom],
    };
    let left_top = TexturedVertex {
        position: [min[0], max[1]],
        uv: [0.0, top],
    };
    let right_top = TexturedVertex {
        position: [max[0], max[1]],
        uv: [1.0, top],
    };

    [
        left_bottom,
        right_bottom,
        left_top,
        left_top,
        right_bottom,
        right_top,
    ]
}

pub fn draw<C: HasTime>(frame: &mut Frame, rendering_context: &C) {
    let theme = frame.theme;
    let image_size = frame.view.size;

    let (rights_min, rights_max) = calculate_rights_notation_position(image_size);
    let (watermark_min, watermark_max) = calculate_watermark_position(image_size);

    let mut vertices = Vec::with_capacity(12);
    vertices.extend(textured_quad(rights_min, rights_max, RIGHTS_NOTATION_V));
    vertices.extend(textured_quad(watermark_min, watermark_max, WATERMARK_V));

    frame.textured(&frame.resources.overlay_texture, &vertices, [1.0; 4]);

    let mut time_text = rendering_context
        .time()
        .with_timezone(&Japan)
        .format("%Y年%m月%d日 %H時%M分頃")
        .to_string();
    time_text.push_str(&rendering_context.time_kind().to_string());

    frame.text(
        &time_text,
        Font::BizUDPGothicBold,
        theme.occurrence_time_color,
        20.0, // TODO: calculate from dimension
        Offset::new(Origin::RightDown, Origin::RightDown, -20, -20),
    );
}

/// 権利表記の矩形 (左下, 右上)。画像の左下に置く。
fn calculate_rights_notation_position(image_size: Size<u32>) -> ([f32; 2], [f32; 2]) {
    let x_offset = (2.0 / image_size.x() as f32) * OVERLAY_OFFSET_PIXELS as f32;
    let y_offset = (2.0 / image_size.y() as f32) * OVERLAY_OFFSET_PIXELS as f32;

    let width = RIGHTS_NOTATION_RATIO_IN_Y_AXIS * image_size.aspect_ratio() * 2.0 * 4.0;
    let height = RIGHTS_NOTATION_RATIO_IN_Y_AXIS * 2.0;

    (
        [-1.0 + x_offset, -1.0 + y_offset],
        [-1.0 + x_offset + width, -1.0 + y_offset + height],
    )
}

/// 透かしの矩形 (左下, 右上)。画像の右上に置く。
fn calculate_watermark_position(image_size: Size<u32>) -> ([f32; 2], [f32; 2]) {
    let x_offset = (2.0 / image_size.x() as f32) * OVERLAY_OFFSET_PIXELS as f32;
    let y_offset = (2.0 / image_size.y() as f32) * OVERLAY_OFFSET_PIXELS as f32;

    let width = WATERMARK_RATIO_IN_Y_AXIS * image_size.aspect_ratio() * 2.0 * 4.0;
    let height = WATERMARK_RATIO_IN_Y_AXIS * 2.0;

    (
        [1.0 - x_offset - width, 1.0 - y_offset - height],
        [1.0 - x_offset, 1.0 - y_offset],
    )
}
