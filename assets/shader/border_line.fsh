#version 430

uniform vec3 color;

in float alpha;

out vec4 fragColor;

void main() {
    fragColor = vec4(color, step(1.0, alpha));
}
