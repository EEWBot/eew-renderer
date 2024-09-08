use array_const_fn_init::array_const_fn_init;
use enum_map::EnumMap;
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::{implement_vertex, uniform, DrawParameters, Surface, VertexBuffer};

use crate::intensity::震度;

use renderer_types::*;

const ICON_RATIO_IN_Y_AXIS: f32 = 0.05;

#[derive(Copy, Clone)]
struct PerInstanceData {
    position: [f32; 2],
    uv_offset: [f32; 2],
}

implement_vertex!(PerInstanceData, position, uv_offset);

const fn 震度_to_uv_offset_fn(震度_i: usize) -> [f32; 2] {
    use const_soft_float::soft_f32::SoftF32;

    let virtual_texture_size = SoftF32(64.0);
    let icon_size_in_virtual_texture_size = SoftF32(21.0);

    let uv_normalized_icon_size = icon_size_in_virtual_texture_size.div(virtual_texture_size);

    let col_i = SoftF32((震度_i % 3) as f32);
    let offset_x = uv_normalized_icon_size.mul(col_i);

    let row_i = SoftF32((震度_i / 3) as f32);
    let offset_y = SoftF32(1.0).sub(uv_normalized_icon_size.mul(row_i));

    [offset_x.to_f32(), offset_y.to_f32()]
}

const 震度_TO_UV_OFFSET: [[f32; 2]; 9] = array_const_fn_init![震度_to_uv_offset_fn; 9];

pub fn draw_all<F: ?Sized + Facade, S: ?Sized + Surface>(
    objects: &EnumMap<震度, Vec<codes::Area>>,
    offset: Vertex<Screen>,
    aspect_ratio: f32,
    scale: f32,
    facade: &F,
    surface: &mut S,
    resources: &crate::resources::Resources,
    params: &DrawParameters,
) {
    let per_icon_data: Vec<_> = objects
        .iter()
        .flat_map(|(震度, area_codes)| {
            let uv_offset = &震度_TO_UV_OFFSET[震度 as usize];

            area_codes.iter().filter_map(|code| {
                let nearest_station_coord = renderer_assets::QueryInterface::query_rendering_center_by_area(*code)?;

                Some(
                    PerInstanceData {
                        position: nearest_station_coord.to_slice(),
                        uv_offset: uv_offset.to_owned(),
                    }
                )
            })
        })
        .collect();

    let per_icon_data = VertexBuffer::dynamic(facade, &per_icon_data).unwrap();

    surface
        .draw(
            &per_icon_data,
            NoIndices(PrimitiveType::Points),
            &resources.shader.intensity_icon,
            &uniform! {
                aspect_ratio: aspect_ratio,
                offset: offset.to_slice(),
                zoom: scale,
                icon_ratio_in_y_axis: ICON_RATIO_IN_Y_AXIS,
                texture_sampler: &resources.texture.intensity,
            },
            params,
        )
        .unwrap();
}
