#version 410

uniform vec2 dimension;
uniform float line_width;

layout(lines) in;
in int vertex_id_vsh_out[];
in vec4 color_vsh_out[];

layout(triangle_strip, max_vertices = 8) out;
out vec4 color_gsh_out;

const float PI = 3.14159265358979323846264338327950288;

float atan2(vec2 v) {
    return v.x == 0.0 ? sign(v.y) * PI / 2 : atan(v.y, v.x);
}

void main() {
    if (sign(vertex_id_vsh_out[0]) + sign(vertex_id_vsh_out[1]) < 2) {
        // 片方あるいは両方の点のインデックスが0なので描画しない
        return;
    }

    vec4 edge1 = gl_in[0].gl_Position;
    vec4 edge2 = gl_in[1].gl_Position;
    float angle = atan2(edge1.xy - edge2.xy) - 0.5 * PI;
    float radius = 2.0 / dimension.x * line_width / 2.0;
    float aspect_ratio = dimension.y / dimension.x;

    gl_Position = vec4(
        edge1.x + radius * cos(angle + PI),
        edge1.y + radius * sin(angle + PI) / aspect_ratio,
        edge1.z,
        edge1.w
    );
    color_gsh_out = color_vsh_out[0];
    EmitVertex();

    gl_Position = vec4(
        edge2.x + radius * cos(angle + 1.0 * PI),
        edge2.y + radius * sin(angle + 1.0 * PI) / aspect_ratio,
        edge2.z,
        edge2.w
    );
    color_gsh_out = color_vsh_out[0];
    EmitVertex();

    gl_Position = vec4(
        edge1.x + radius * cos(angle + 2.0 / 3.0 * PI),
        edge1.y + radius * sin(angle + 2.0 / 3.0 * PI) / aspect_ratio,
        edge1.z,
        edge1.w
    );
    color_gsh_out = color_vsh_out[0];
    EmitVertex();

    gl_Position = vec4(
        edge2.x + radius * cos(angle + 4.0 / 3.0 * PI),
        edge2.y + radius * sin(angle + 4.0 / 3.0 * PI) / aspect_ratio,
        edge2.z,
        edge2.w
    );
    color_gsh_out = color_vsh_out[0];
    EmitVertex();

    gl_Position = vec4(
        edge1.x + radius * cos(angle + 1.0 / 3.0 * PI),
        edge1.y + radius * sin(angle + 1.0 / 3.0 * PI) / aspect_ratio,
        edge1.z,
        edge1.w
    );
    color_gsh_out = color_vsh_out[0];
    EmitVertex();

    gl_Position = vec4(
        edge2.x + radius * cos(angle + 5.0 / 3.0 * PI),
        edge2.y + radius * sin(angle + 5.0 / 3.0 * PI) / aspect_ratio,
        edge2.z,
        edge2.w
    );
    color_gsh_out = color_vsh_out[0];
    EmitVertex();

    gl_Position = vec4(
        edge1.x + radius * cos(angle),
        edge1.y + radius * sin(angle) / aspect_ratio,
        edge1.z,
        edge1.w
    );
    color_gsh_out = color_vsh_out[0];
    EmitVertex();

    gl_Position = vec4(
        edge2.x + radius * cos(angle),
        edge2.y + radius * sin(angle) / aspect_ratio,
        edge2.z,
        edge2.w
    );
    color_gsh_out = color_vsh_out[0];
    EmitVertex();
}
