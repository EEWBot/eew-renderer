#version 410

uniform sampler2D texture_sampler;

layout (location = 0) in vec2 uv_gsh_out;

out vec4 fragment_color;

void main() {
    fragment_color = texture(texture_sampler, uv_gsh_out);
}
