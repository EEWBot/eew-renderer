#version 460

uniform vec2 dimension;
uniform vec2 offset;
uniform float zoom;
uniform vec3 forecast_color;
uniform vec3 advisory_color;
uniform vec3 warning_color;
uniform vec3 major_warning_color;

/**
 * 0 -> 発令なし
 * 1 -> 津波予報(若干の海面変動)
 * 2 -> 津波注意報
 * 3 -> 津波警報
 * 4 -> 大津波警報
**/
layout (r8ui) readonly uniform uimage1D levels;

in vec2 position;
in int code;

out int vertex_id_vsh_out;
out vec4 color_vsh_out;

const float PI = 3.14159265358979323846264338327950288;
const float e = 0.081819191042815791; // https://ja.wikipedia.org/wiki/GRS80

vec2 to_mercator(vec2 coord) {
    vec2 radianized = radians(coord);
    float x = radianized.x / PI;
    float y = (atanh(sin(radianized.y)) - e * atanh(e * sin(radianized.y))) / PI;
    return vec2(x, y);
}

void main() {
    uint level = imageLoad(levels, code).r;

    float aspect_ratio = dimension.y / dimension.x;
    vec2 map_coordinate = (to_mercator(position) + offset) * zoom;
    vec2 display_coordinate = vec2(map_coordinate.x, map_coordinate.y / aspect_ratio);
    vec2 no_forecast_cull = display_coordinate * sign(level);
    gl_Position = vec4(no_forecast_cull, 0.0, 1.0);
    vertex_id_vsh_out = gl_VertexID;

    if (level == 1) {
        color_vsh_out = vec4(forecast_color, 1.0);
    } else if (level == 2) {
        color_vsh_out = vec4(advisory_color, 1.0);
    } else if (level == 3) {
        color_vsh_out = vec4(warning_color, 1.0);
    } else if (level == 4) {
        color_vsh_out = vec4(major_warning_color, 1.0);
    } else {
        color_vsh_out = vec4(0.0, 0.0, 0.0, 0.0);
    }
}
