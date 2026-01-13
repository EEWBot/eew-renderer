use crate::worker::vertex::TsunamiLineColors;

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
}

pub const DEFAULT: Theme = Theme {
    #[allow(clippy::eq_op)]
    clear_color: [130.0 / 255.0, 188.0 / 255.0, 255.0 / 255.0, 1.0],
    ground_color: [222.0 / 255.0, 226.0 / 255.0, 229.0 / 255.0],
    prefectural_border_color: [148.0 / 255.0, 151.0 / 255.0, 153.0 / 255.0],
    prefectural_border_width: 2.0,
    area_border_color: [148.0 / 255.0, 151.0 / 255.0, 153.0 / 255.0],
    area_border_width: 1.0,
    tsunami_colors: TsunamiLineColors {
        forecast: [0.0 / 255.0, 191.0 / 255.0, 255.0 / 255.0],
        advisory: [250.0 / 255.0, 245.0 / 255.0, 0.0 / 255.0],
        warning: [255.0 / 255.0, 40.0 / 255.0, 0.0 / 255.0],
        major_warning: [200.0 / 255.0, 0.0 / 255.0, 255.0 / 255.0],
    },
    tsunami_width: 3.0,
    tsunami_legend_color: [0.0, 0.0, 0.0, 0.8],
    occurrence_time_color: [0.0, 0.0, 0.0, 0.63],
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
        forecast: [0.0 / 255.0, 191.0 / 255.0, 255.0 / 255.0],
        advisory: [250.0 / 255.0, 245.0 / 255.0, 0.0 / 255.0],
        warning: [255.0 / 255.0, 40.0 / 255.0, 0.0 / 255.0],
        major_warning: [200.0 / 255.0, 0.0 / 255.0, 255.0 / 255.0],
    },
    tsunami_width: 8.0,
    tsunami_legend_color: [0.0, 0.0, 0.0, 0.8],
    occurrence_time_color: [1.0, 1.0, 1.0, 0.63],
};
