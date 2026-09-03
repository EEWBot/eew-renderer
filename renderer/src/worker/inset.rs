use super::camera::Camera;
use glium::Rect;
use renderer_types::{
    BoundingBox, GeoDegree, InsetRegion, Size, Vertex, OGASAWARA_VIEW_BBOX, OKINAWA_VIEW_BBOX,
};

pub struct BorderSides {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl BorderSides {
    pub const ALL: Self = Self {
        top: true,
        bottom: true,
        left: true,
        right: true,
    };
}

pub struct NotchLine {
    pub from: Vertex<GeoDegree>,
    pub to: Vertex<GeoDegree>,
}

pub struct InsetPass {
    pub region: InsetRegion,
    pub view_bbox: BoundingBox<GeoDegree>,
    pub viewport: Rect,
    pub border_sides: BorderSides,
    pub notch: Option<NotchLine>,
    pub label: &'static str,
}

pub const OKINAWA_INSET: InsetPass = InsetPass {
    region: InsetRegion::Okinawa,
    view_bbox: OKINAWA_VIEW_BBOX,
    viewport: Rect {
        left: 0,
        bottom: 321,
        width: 447,
        height: 447,
    },
    border_sides: BorderSides {
        top: false,
        bottom: true,
        left: false,
        right: true,
    },
    // 大東諸島の南東を通る斜め線
    notch: Some(NotchLine {
        from: Vertex::new(132.2, 25.9),
        to: Vertex::new(129.4, 23.9),
    }),
    label: "沖縄",
};

pub const OGASAWARA_INSET: InsetPass = InsetPass {
    region: InsetRegion::Ogasawara,
    view_bbox: OGASAWARA_VIEW_BBOX,
    viewport: Rect {
        left: 750,
        bottom: 180,
        width: 180,
        height: 260,
    },
    border_sides: BorderSides::ALL,
    notch: None,
    label: "小笠原",
};

pub const ALL_INSETS: [&InsetPass; 2] = [&OKINAWA_INSET, &OGASAWARA_INSET];

impl InsetPass {
    pub fn camera(&self) -> Camera {
        let image_size = Size::new(self.viewport.width, self.viewport.height);
        Camera::fit(&self.view_bbox, image_size)
    }
}
