use glium::backend::Facade;
use glium::{DrawParameters, IndexBuffer, Surface, uniform, VertexBuffer};
use glium::index::PrimitiveType;
use crate::vertex::え;

const OVERLAY_OFFSET_PIXELS: u16 = 10;
const RIGHTS_NOTATION_RATIO_IN_Y_AXIS: f32 = 0.16;
const WATERMARK_RATIO_IN_Y_AXIS: f32 = 0.12;

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface>(
    dimension: &(u32, u32),
    aspect: &f32,
    facade: &F,
    surface: &mut S,
    resources: &crate::resources::Resources,
    params: &DrawParameters,
) {
    let rights_position = calculate_rights_notation_position(dimension, aspect);
    let watermark_position = calculate_watermark_position(dimension, aspect);


    let vertices = [
        え::new(rights_position[0], (0.0, 0.5)),
        え::new(rights_position[1], (1.0, 0.5)),
        え::new(rights_position[2], (0.0, 0.75)),
        え::new(rights_position[3], (1.0, 0.75)),
        え::new(watermark_position[0], (0.0, 0.75)),
        え::new(watermark_position[1], (1.0, 0.75)),
        え::new(watermark_position[2], (0.0, 1.0)),
        え::new(watermark_position[3], (1.0, 1.0)),
    ];
    let indices = [0_u32, 1, 2, 3, 3, 4, 4, 5, 6, 7];

    let vertex_buffer = VertexBuffer::dynamic(facade, &vertices).unwrap();
    let index_buffer = IndexBuffer::dynamic(facade, PrimitiveType::TriangleStrip, &indices).unwrap();

    surface.draw(
        &vertex_buffer,
        &index_buffer,
        &resources.shader.textured,
        &uniform! {
            texture_sampler: &resources.texture.overlay,
        },
        params
    )
    .unwrap();
}

fn calculate_rights_notation_position(dimension: &(u32, u32), aspect: &f32) -> [(f32, f32); 4] {
    let x_offset = (2.0 / dimension.0 as f32) * OVERLAY_OFFSET_PIXELS as f32;
    let y_offset = (2.0 / dimension.1 as f32) * OVERLAY_OFFSET_PIXELS as f32;
    [
        (-1.0 + x_offset, -1.0 + y_offset),
        (-1.0 + x_offset + RIGHTS_NOTATION_RATIO_IN_Y_AXIS * aspect * 2.0 * 4.0, -1.0 + y_offset),
        (-1.0 + x_offset, -1.0 + RIGHTS_NOTATION_RATIO_IN_Y_AXIS * 2.0 + y_offset),
        (-1.0 + x_offset + RIGHTS_NOTATION_RATIO_IN_Y_AXIS * aspect * 2.0 * 4.0, -1.0 + RIGHTS_NOTATION_RATIO_IN_Y_AXIS * 2.0 + y_offset),
    ]
}

fn calculate_watermark_position(dimension: &(u32, u32), aspect: &f32) -> [(f32, f32); 4] {
    let x_offset = (2.0 / dimension.0 as f32) * OVERLAY_OFFSET_PIXELS as f32;
    let y_offset = (2.0 / dimension.1 as f32) * OVERLAY_OFFSET_PIXELS as f32;
    [
        (1.0 - x_offset - WATERMARK_RATIO_IN_Y_AXIS * aspect * 2.0 * 4.0, 1.0 - WATERMARK_RATIO_IN_Y_AXIS * 2.0 - y_offset),
        (1.0 - x_offset, 1.0 - WATERMARK_RATIO_IN_Y_AXIS * 2.0 - y_offset),
        (1.0 - x_offset - WATERMARK_RATIO_IN_Y_AXIS * aspect * 2.0 * 4.0, 1.0 - y_offset),
        (1.0 - x_offset, 1.0 - y_offset),
    ]
}
