use crate::frame_context::MapLayerConfig;
use crate::worker::frame::Frame;
use renderer_types::InsetRegion;

pub fn draw(frame: &mut Frame, layers: MapLayerConfig, region: Option<InsetRegion>) {
    let theme = frame.theme;
    let resources = frame.resources;
    let zoom = frame.view.zoom;

    if layers.world {
        frame.fill_mesh(
            &resources.world,
            0..resources.world.index_count,
            theme.ground_color,
        );
    }

    frame.fill_mesh(
        &resources.map,
        resources.map_range(region),
        theme.ground_color,
    );

    // lakeと県境線は本土のみに存在するため、Main以外のinset描画時はskip
    let draws_mainland = region.is_none_or(|region| region == InsetRegion::Main);

    if draws_mainland {
        frame.fill_mesh(
            &resources.lake,
            0..resources.lake.index_count,
            [
                theme.clear_color[0],
                theme.clear_color[1],
                theme.clear_color[2],
            ],
        );
    }

    if layers.saibunkuiki_line {
        frame.border_lines(
            resources.area_line_range(zoom),
            theme.area_border_width,
            theme.area_border_color,
        );
    }

    if draws_mainland {
        frame.border_lines(
            resources.pref_line_range(zoom),
            theme.prefectural_border_width,
            theme.prefectural_border_color,
        );
    }
}
