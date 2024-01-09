#version 430

uniform float aspect_ratio;
uniform float icon_ratio_in_y_axis;

layout(points) in;
layout (location = 0) in vec2 uv_offset[];

layout(triangle_strip, max_vertices = 4) out;
layout (location = 0) out vec2 uv_out;

void main() {
    vec4 position = gl_in[0].gl_Position.xyzw;

    gl_Position = vec4(
            -icon_ratio_in_y_axis * aspect_ratio + position.x,
            -icon_ratio_in_y_axis + position.y,
            position.z,
            position[3]
    );
    uv_out = vec2(
            1.0f / 64.0f * 1.0 + uv_offset[0].x,
            1.0f / 64.0f * 43.0f + uv_offset[0].y
    );
    EmitVertex();

    gl_Position = vec4(
        icon_ratio_in_y_axis * aspect_ratio + position.x,
        -icon_ratio_in_y_axis + position.y,
        position.z,
        position[3]
    );
    uv_out = vec2(
        1.0f / 64.0f * 21.0 + uv_offset[0].x,
        1.0f / 64.0f * 43.0f + uv_offset[0].y
    );
    EmitVertex();

    gl_Position = vec4(
        -icon_ratio_in_y_axis * aspect_ratio + position.x,
        icon_ratio_in_y_axis + position.y,
        position.z,
        position[3]
    );
    uv_out = vec2(
        1.0f / 64.0f * 1.0 + uv_offset[0].x,
        1.0f / 64.0f * 63.0f + uv_offset[0].y
    );
    EmitVertex();

    gl_Position = vec4(
        icon_ratio_in_y_axis * aspect_ratio + position.x,
        icon_ratio_in_y_axis + position.y,
        position.z,
        position[3]
    );
    uv_out = vec2(
        1.0f / 64.0f * 21.0 + uv_offset[0].x,
        1.0f / 64.0f * 63.0f + uv_offset[0].y
    );
    EmitVertex();

    EndPrimitive();
}
