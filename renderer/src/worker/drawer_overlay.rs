use std::ops::DerefMut;
use chrono_tz::Tz::Japan;
use glium::backend::Facade;
use glium::index::PrimitiveType;
use glium::{IndexBuffer, Surface, VertexBuffer};
use rusttype::Scale;
use crate::worker::fonts::{Font, Offset, Origin};
use crate::worker::FrameContext;
use super::vertex::{TexturedUniform, TexturedVertex};

const OVERLAY_OFFSET_PIXELS: u16 = 10;
const RIGHTS_NOTATION_RATIO_IN_Y_AXIS: f32 = 0.16;
const WATERMARK_RATIO_IN_Y_AXIS: f32 = 0.12;

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface>(frame_context: &FrameContext<F, S>) {
    let dimension = frame_context.dimension();
    let aspect_ratio = frame_context.aspect_ratio();
    let facade = frame_context.facade;
    let resources = frame_context.resources;
    let draw_parameters = frame_context.draw_parameters;
    let rendering_context = frame_context.rendering_context;
    
    let rights_position = calculate_rights_notation_position(dimension, aspect_ratio);
    let watermark_position = calculate_watermark_position(dimension, aspect_ratio);

    let vertices = [
        TexturedVertex {
            position: rights_position[0],
            uv: [0.0, 0.5]
        },
        TexturedVertex {
            position: rights_position[1],
            uv: [1.0, 0.5]
        },
        TexturedVertex {
            position: rights_position[2],
            uv: [0.0, 0.75]
        },
        TexturedVertex {
            position: rights_position[3],
            uv: [1.0, 0.75]
        },
        TexturedVertex {
            position: watermark_position[0],
            uv: [0.0, 0.75]
        },
        TexturedVertex {
            position: watermark_position[1],
            uv: [1.0, 0.75]
        },
        TexturedVertex {
            position: watermark_position[2],
            uv: [0.0, 1.0]
        },
        TexturedVertex {
            position: watermark_position[3],
            uv: [1.0, 1.0]
        },
    ];
    let indices = [0_u32, 1, 2, 3, 3, 4, 4, 5, 6, 7];

    let vertex_buffer = VertexBuffer::dynamic(facade, &vertices).unwrap();
    let index_buffer =
        IndexBuffer::dynamic(facade, PrimitiveType::TriangleStrip, &indices).unwrap();

    resources
        .shader
        .textured
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            &vertex_buffer,
            &index_buffer,
            &TexturedUniform {
                texture_sampler: &resources.texture.overlay,
            },
            draw_parameters,
        )
        .unwrap();

    let time_text = rendering_context
        .time
        .with_timezone(&Japan)
        .format("%Y年%m月%d日 %H時%M分頃発生")
        .to_string();
    frame_context
        .font_manager
        .borrow_mut()
        .deref_mut()
        .draw_text(
            &time_text,
            Font::BizUDPGothicBold,
            [0.0f32, 0.0, 0.0, 0.63],
            Scale::uniform(30.0),
            Offset::new(Origin::RightDown, Origin::RightDown, -30, -30),
            frame_context.dimension(),
            resources,
            facade,
            frame_context.surface.borrow_mut().deref_mut(),
            draw_parameters,
        );
}

fn calculate_rights_notation_position(dimension: (u32, u32), aspect: f32) -> [[f32; 2]; 4] {
    let x_offset = (2.0 / dimension.0 as f32) * OVERLAY_OFFSET_PIXELS as f32;
    let y_offset = (2.0 / dimension.1 as f32) * OVERLAY_OFFSET_PIXELS as f32;
    [
        [-1.0 + x_offset, -1.0 + y_offset],
        [
            -1.0 + x_offset + RIGHTS_NOTATION_RATIO_IN_Y_AXIS * aspect * 2.0 * 4.0,
            -1.0 + y_offset,
        ],
        [
            -1.0 + x_offset,
            -1.0 + RIGHTS_NOTATION_RATIO_IN_Y_AXIS * 2.0 + y_offset,
        ],
        [
            -1.0 + x_offset + RIGHTS_NOTATION_RATIO_IN_Y_AXIS * aspect * 2.0 * 4.0,
            -1.0 + RIGHTS_NOTATION_RATIO_IN_Y_AXIS * 2.0 + y_offset,
        ],
    ]
}

fn calculate_watermark_position(dimension: (u32, u32), aspect: f32) -> [[f32; 2]; 4] {
    let x_offset = (2.0 / dimension.0 as f32) * OVERLAY_OFFSET_PIXELS as f32;
    let y_offset = (2.0 / dimension.1 as f32) * OVERLAY_OFFSET_PIXELS as f32;
    [
        [
            1.0 - x_offset - WATERMARK_RATIO_IN_Y_AXIS * aspect * 2.0 * 4.0,
            1.0 - WATERMARK_RATIO_IN_Y_AXIS * 2.0 - y_offset,
        ],
        [
            1.0 - x_offset,
            1.0 - WATERMARK_RATIO_IN_Y_AXIS * 2.0 - y_offset,
        ],
        [
            1.0 - x_offset - WATERMARK_RATIO_IN_Y_AXIS * aspect * 2.0 * 4.0,
            1.0 - y_offset,
        ],
        [1.0 - x_offset, 1.0 - y_offset],
    ]
}
