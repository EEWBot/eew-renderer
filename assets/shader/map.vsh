#version 410

uniform float aspect_ratio;
uniform vec2 offset;
uniform float zoom;

in vec2 position;

const float PI = 3.14159265358979323846264338327950288;
const float e = 0.081819191042815791; // https://ja.wikipedia.org/wiki/GRS80

vec2 to_mercator(vec2 coord) {
    vec2 radianized = radians(coord);
    float x = radianized.x / PI;
    float y = (atanh(sin(radianized.y)) - e * atanh(e * sin(radianized.y))) / PI;
    return vec2(x, y);
}

void main() {
    vec2 map_coordinate = (to_mercator(position) + offset) * zoom;
    vec2 display_coordinate = vec2(map_coordinate.x, map_coordinate.y / aspect_ratio);
    gl_Position = vec4(display_coordinate, 0.0, 1.0);
}
