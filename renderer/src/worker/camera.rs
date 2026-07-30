use renderer_types::{BoundingBox, GeoDegree, Mercator, Size, Vertex};

const MAXIMUM_SCALE: f32 = 100.0;
const SCALE_FACTOR: f32 = 1.2;

pub struct Camera {
    pub offset: Vertex<Mercator>,
    pub scale: f32,
    pub image_size: Size<u32>,
}

impl Camera {
    pub fn fit(view_bbox: &BoundingBox<GeoDegree>, image_size: Size<u32>) -> Camera {
        let rendering_bbox = BoundingBox::from_vertices_float(
            &view_bbox
                .gl_vertices()
                .iter()
                .map(|v| v.to_mercator())
                .collect::<Vec<_>>(),
        );

        Camera {
            offset: -rendering_bbox.center(),
            scale: calculate_map_scale(rendering_bbox, image_size),
            image_size,
        }
    }

    pub fn zoomed(self, factor: f32) -> Camera {
        Camera {
            scale: self.scale * factor,
            ..self
        }
    }
}

fn calculate_map_scale(bounding_box: BoundingBox<Mercator>, image_size: Size<u32>) -> f32 {
    let x_scale = 1.0 / bounding_box.size().x();
    let y_scale = 1.0 / bounding_box.size().y() * image_size.aspect_ratio();

    f32::min(f32::min(x_scale, y_scale) * 2.0, MAXIMUM_SCALE) / SCALE_FACTOR
}
