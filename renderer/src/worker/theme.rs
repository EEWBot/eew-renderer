#![allow(clippy::eq_op)]
use crate::worker::vertex::TsunamiLineColors;
use renderer_macros::{rgb, rgba};

pub struct Theme {
    pub clear_color: [f32; 4],
    pub ground_color: [f32; 3],
    pub prefectural_border_color: [f32; 3],
    pub prefectural_border_width: f32,
    pub area_border_color: [f32; 3],
    pub area_border_width: f32,
    pub tsunami_colors: TsunamiLineColors,
    pub tsunami_width: f32,
    pub tsunami_legend_color: [f32; 4],
    pub occurrence_time_color: [f32; 4],
    pub inset_background_color: [f32; 4],
    pub inset_border_color: [f32; 4],
    pub inset_border_width: f32,
    pub inset_label_color: [f32; 4],
    pub inset_label_font_size: f32,
    pub inset_label_offset_y: i32,
}

pub const DEFAULT: Theme = Theme {
    #[allow(clippy::eq_op)]
    clear_color: rgba!("#82BCFF", 1.0),
    ground_color: rgb!("#DEE2E5"),
    prefectural_border_color: rgb!("#949799"),
    prefectural_border_width: 2.0,
    area_border_color: rgb!("#949799"),
    area_border_width: 1.0,
    tsunami_colors: TsunamiLineColors {
        forecast: rgb!("#00BFFF"),
        advisory: rgb!("#FAF500"),
        warning: rgb!("#FF2800"),
        major_warning: rgb!("#CB00FF"),
    },
    tsunami_width: 3.0,
    tsunami_legend_color: rgba!("#000000", 0.8),
    occurrence_time_color: rgba!("#000000", 0.63),
    inset_background_color: rgba!("#82BCFF", 1.0),
    inset_border_color: rgba!("#969BA0", 1.0),
    inset_border_width: 3.0,
    inset_label_color: rgba!("#000000", 0.8),
    inset_label_font_size: 22.0,
    inset_label_offset_y: -6,
};

#[allow(dead_code)]
pub const DARK_DEMO: Theme = Theme {
    clear_color: [0.1, 0.12, 0.15, 1.0],
    ground_color: [0.35, 0.35, 0.35],
    prefectural_border_color: [0.75, 0.75, 0.75],
    prefectural_border_width: 5.0,
    area_border_color: [0.6, 0.6, 0.6],
    area_border_width: 2.0,
    tsunami_colors: TsunamiLineColors {
        forecast: rgb!("#00BFFF"),
        advisory: rgb!("#FAF500"),
        warning: rgb!("#FF2800"),
        major_warning: rgb!("#CB00FF"),
    },
    tsunami_width: 8.0,
    tsunami_legend_color: [0.0, 0.0, 0.0, 0.8],
    occurrence_time_color: [1.0, 1.0, 1.0, 0.63],
    inset_background_color: [0.1, 0.12, 0.15, 1.0],
    inset_border_color: [0.55, 0.55, 0.55, 1.0],
    inset_border_width: 3.0,
    inset_label_color: [1.0, 1.0, 1.0, 0.8],
    inset_label_font_size: 22.0,
    inset_label_offset_y: -6,
};
