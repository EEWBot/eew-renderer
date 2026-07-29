use glium::Rect;
use renderer_types::{
    BoundingBox, GeoDegree, InsetRegion, Mercator, Size, Vertex, ViewBBox, OGASAWARA_VIEW_BBOX,
    OKINAWA_VIEW_BBOX,
};

pub struct InsetPass {
    pub region: InsetRegion,
    pub view_bbox: ViewBBox,
    pub viewport: Rect,
    pub label: &'static str,
}

pub const OKINAWA_INSET: InsetPass = InsetPass {
    region: InsetRegion::Okinawa,
    view_bbox: OKINAWA_VIEW_BBOX,
    viewport: Rect {
        left: 10,
        bottom: 413,
        width: 345,
        height: 345,
    },
    label: "沖縄",
};

pub const OGASAWARA_INSET: InsetPass = InsetPass {
    region: InsetRegion::Ogasawara,
    view_bbox: OGASAWARA_VIEW_BBOX,
    viewport: Rect {
        left: 834,
        bottom: 180,
        width: 180,
        height: 330,
    },
    label: "小笠原",
};

pub const ALL_INSETS: [&InsetPass; 2] = [&OKINAWA_INSET, &OGASAWARA_INSET];

const INSET_SCALE_FACTOR: f32 = 1.2;

pub struct InsetCamera {
    pub offset: Vertex<Mercator>,
    pub scale: f32,
    pub image_size: Size<u32>,
}

impl InsetPass {
    pub fn camera(&self) -> InsetCamera {
        let ((min_lon, min_lat), (max_lon, max_lat)) = self.view_bbox;
        let bounding_box: BoundingBox<GeoDegree> =
            BoundingBox::new(Vertex::new(min_lon, min_lat), Vertex::new(max_lon, max_lat));
        let rendering_bbox = BoundingBox::from_vertices_float(
            &bounding_box
                .gl_vertices()
                .iter()
                .map(|v| v.to_mercator())
                .collect::<Vec<_>>(),
        );

        let image_size = Size::new(self.viewport.width, self.viewport.height);
        let offset = -rendering_bbox.center();
        let scale = super::calculate_map_scale(rendering_bbox, image_size) * INSET_SCALE_FACTOR;

        InsetCamera {
            offset,
            scale,
            image_size,
        }
    }
}
