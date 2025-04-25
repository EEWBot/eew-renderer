#version 410

uniform sampler2D font_texture;
uniform vec4 color;

layout (location = 0) in vec2 uv_vsh_out;

out vec4 fragment_color;

void main() {
    fragment_color = color * vec4(1.0, 1.0, 1.0, texture(font_texture, uv_vsh_out).r);
}
