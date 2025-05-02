pub struct Theme {
    pub clear_color: [f32; 4],
    pub ground_color: [f32; 3],
    pub prefectural_border_color: [f32; 3],
    pub prefectural_border_width: f32,
    pub area_border_color: [f32; 3],
    pub area_border_width: f32,
    pub occurrence_time_color: [f32; 4],
}

pub const DEFAULT: Theme = Theme {
    clear_color: [0.5, 0.8, 1.0, 1.0],
    ground_color: [0.8, 0.8, 0.8],
    prefectural_border_color: [0.35, 0.25, 0.19],
    prefectural_border_width: 2.0,
    area_border_color: [0.35, 0.25, 0.19],
    area_border_width: 1.0,
    occurrence_time_color: [0.0, 0.0, 0.0, 0.63],
};

pub const DARK_DEMO: Theme = Theme {
    clear_color: [0.1, 0.12, 0.15, 1.0],
    ground_color: [0.35, 0.35, 0.35],
    prefectural_border_color: [0.75, 0.75, 0.75],
    prefectural_border_width: 5.0,
    area_border_color: [0.6, 0.6, 0.6],
    area_border_width: 2.0,
    occurrence_time_color: [1.0, 1.0, 1.0, 0.63],
};
