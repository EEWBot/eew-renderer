#version 430

in vec2 position;
in vec2 uv;

layout (location = 0) out vec2 uv_out;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    uv_out = uv;
}
