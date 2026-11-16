// 全パイプライン共通の描画単位のパラメータ。
//
// クリップ座標は `(position + offset) * scale` で求める。
// 地図はメルカトル座標とカメラの値を、画面に直接置く図形は offset = 0, scale = 1 を使う。
struct Draw {
    offset: vec2<f32>,
    scale: vec2<f32>,
    // ビューポートの大きさ (px)
    viewport: vec2<f32>,
    // 線: x = 線幅 (px) / スプライト: クリップ座標での半分の大きさ
    param: vec2<f32>,
    // 塗り・線: ストレートアルファの色 / テクスチャ: プリマルチプライド済みの乗算色
    color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> draw: Draw;

@group(1) @binding(0) var color_texture: texture_2d<f32>;
@group(1) @binding(1) var color_sampler: sampler;

// 津波予報区の内部コード → 発表レベル (0: なし, 1: 予報, 2: 注意報, 3: 警報, 4: 大津波警報) と、
// レベルごとの線の色。
struct Tsunami {
    colors: array<vec4<f32>, 5>,
    levels: array<vec4<u32>, 32>,
}

@group(1) @binding(2) var<uniform> tsunami: Tsunami;

const PI: f32 = 3.14159265358979323846;

// 線の端につける半円の分割数
const CAP_SLICES: u32 = 4u;

struct ColorVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct TextureVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

fn project(position: vec2<f32>) -> vec2<f32> {
    return (position + draw.offset) * draw.scale;
}

fn premultiply(color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(color.rgb * color.a, color.a);
}

// 線分 1 本を「長方形 + 両端の半円」として描くときの頂点位置。
// 頂点 0..6 が長方形、以降が a 側・b 側の半円 (扇形の三角形リスト)。
// 太さは画面上の px で一定にするため、px 空間で押し出す。
fn segment_vertex(vertex_index: u32, segment: vec4<f32>) -> vec4<f32> {
    let half_viewport = draw.viewport * 0.5;
    let a = project(segment.xy) * half_viewport;
    let b = project(segment.zw) * half_viewport;

    let delta = b - a;
    let len = length(delta);
    var dir = vec2<f32>(1.0, 0.0);
    if len > 1e-6 {
        dir = delta / len;
    }
    let normal = vec2<f32>(-dir.y, dir.x);
    let radius = draw.param.x * 0.5;

    var p: vec2<f32>;
    if vertex_index < 6u {
        var along = array<f32, 6>(0.0, 1.0, 0.0, 0.0, 1.0, 1.0);
        var side = array<f32, 6>(-1.0, -1.0, 1.0, 1.0, -1.0, 1.0);
        p = mix(a, b, along[vertex_index]) + normal * side[vertex_index] * radius;
    } else {
        let k = vertex_index - 6u;
        let cap = k / (3u * CAP_SLICES);
        let slice = (k % (3u * CAP_SLICES)) / 3u;
        let corner = k % 3u;

        var center = a;
        var outward = -dir;
        if cap == 1u {
            center = b;
            outward = dir;
        }

        if corner == 0u {
            p = center;
        } else {
            let step = f32(slice + corner - 1u);
            let theta = (step / f32(CAP_SLICES) - 0.5) * PI;
            p = center + radius * (cos(theta) * outward + sin(theta) * normal);
        }
    }

    return vec4<f32>(p / half_viewport, 0.0, 1.0);
}

// ---- 単色の塗り (地図のポリゴン、画面上の図形) ----

@vertex
fn vs_fill(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(project(position), 0.0, 1.0);
}

@fragment
fn fs_fill() -> @location(0) vec4<f32> {
    return premultiply(draw.color);
}

// ---- 境界線 ----

@vertex
fn vs_line(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) segment: vec4<f32>,
) -> @builtin(position) vec4<f32> {
    return segment_vertex(vertex_index, segment);
}

// ---- 津波予報区の線 ----

@vertex
fn vs_tsunami_line(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) segment: vec4<f32>,
    @location(1) code: u32,
) -> ColorVertex {
    let level = tsunami.levels[code / 4u][code % 4u];

    var out: ColorVertex;
    out.color = tsunami.colors[level];
    if level != u32(draw.param.y) {
        // 今回描くレベルではない予報区 (発表のない予報区を含む) は面積 0 にして描かない
        out.position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else {
        out.position = segment_vertex(vertex_index, segment);
    }
    return out;
}

@fragment
fn fs_vertex_color(in: ColorVertex) -> @location(0) vec4<f32> {
    return premultiply(in.color);
}

// ---- スプライト (震度アイコン、震央) ----

@vertex
fn vs_sprite(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) center: vec2<f32>,
    // (左, 上, 右, 下)
    @location(1) uv_rect: vec4<f32>,
) -> TextureVertex {
    var xs = array<f32, 6>(-1.0, 1.0, -1.0, -1.0, 1.0, 1.0);
    var ys = array<f32, 6>(-1.0, -1.0, 1.0, 1.0, -1.0, 1.0);
    let corner = vec2<f32>(xs[vertex_index], ys[vertex_index]);

    var out: TextureVertex;
    out.position = vec4<f32>(project(center) + corner * draw.param, 0.0, 1.0);
    out.uv = mix(uv_rect.xy, uv_rect.zw, vec2<f32>(corner.x * 0.5 + 0.5, 0.5 - corner.y * 0.5));
    return out;
}

// ---- テクスチャ付きの図形 (オーバーレイ、文字) ----

@vertex
fn vs_textured(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>) -> TextureVertex {
    var out: TextureVertex;
    out.position = vec4<f32>(project(position), 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_textured(in: TextureVertex) -> @location(0) vec4<f32> {
    return textureSample(color_texture, color_sampler, in.uv) * draw.color;
}
