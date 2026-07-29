use crate::frame_context::MapLayerConfig;
use crate::worker::vertex::{BorderLineUniform, MapUniform};
use crate::worker::FrameContext;
use glium::backend::Facade;
use glium::Surface;
use renderer_types::InsetRegion;
use std::ops::DerefMut;

pub fn draw<F: ?Sized + Facade, S: ?Sized + Surface>(
    frame_context: &FrameContext<F, S>,
    layers: MapLayerConfig,
    regions: &[InsetRegion],
) {
    let theme = frame_context.theme;
    let params = frame_context.draw_parameters;
    let resources = frame_context.resources;
    let scale = frame_context.scale;
    let aspect_ratio = frame_context.image_size.aspect_ratio();
    let offset = frame_context.offset.into();
    let image_size: [f32; 2] = frame_context.image_size.to_f32().into();

    if layers.world {
        resources
            .shader
            .map
            .draw(
                frame_context.surface.borrow_mut().deref_mut(),
                &resources.world.vertex,
                &resources.world.index,
                &MapUniform {
                    aspect_ratio,
                    offset,
                    zoom: scale,
                    color: theme.ground_color,
                },
                params,
            )
            .unwrap();
    }

    for region in regions {
        resources
            .shader
            .map
            .draw(
                frame_context.surface.borrow_mut().deref_mut(),
                &resources.buffer.map_vertex,
                resources.buffer.get_map_by_region(*region),
                &MapUniform {
                    aspect_ratio,
                    offset,
                    zoom: scale,
                    color: theme.ground_color,
                },
                params,
            )
            .unwrap();
    }

    resources
        .shader
        .map
        .draw(
            frame_context.surface.borrow_mut().deref_mut(),
            &resources.lake.vertex,
            &resources.lake.index,
            &MapUniform {
                aspect_ratio,
                offset,
                zoom: scale,
                color: [
                    theme.clear_color[0],
                    theme.clear_color[1],
                    theme.clear_color[2],
                ],
            },
            params,
        )
        .unwrap();

    if layers.saibunkuiki_line {
        resources
            .shader
            .border_line
            .draw(
                frame_context.surface.borrow_mut().deref_mut(),
                &resources.buffer.map_vertex,
                resources.buffer.get_area_line_by_scale(scale).unwrap(),
                &BorderLineUniform {
                    dimension: image_size,
                    offset,
                    zoom: scale,
                    line_width: theme.area_border_width,
                    color: theme.area_border_color,
                },
                params,
            )
            .unwrap();
    }

    // 県境線は本土のみに存在するため、Main を含む場合のみ描画
    if regions.contains(&InsetRegion::Main) {
        resources
            .shader
            .border_line
            .draw(
                frame_context.surface.borrow_mut().deref_mut(),
                &resources.buffer.map_vertex,
                resources.buffer.get_pref_line_by_scale(scale).unwrap(),
                &BorderLineUniform {
                    dimension: image_size,
                    offset,
                    zoom: scale,
                    line_width: theme.prefectural_border_width,
                    color: theme.prefectural_border_color,
                },
                params,
            )
            .unwrap();
    }
}
