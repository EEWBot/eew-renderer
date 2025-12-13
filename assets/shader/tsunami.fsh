#version 410

in vec4 color_gsh_out;

out vec4 fragment_color;

void main() {
//    fragment_color = color_gsh_out;
    fragment_color = vec4(1.0, 0.0, 0.0, 1.0);
}
