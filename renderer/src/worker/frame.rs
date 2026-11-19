use super::camera::Camera;
use super::fonts::{Font, Fonts, GlyphAtlas, Offset, GLYPH_ATLAS_SIZE};
use super::inset::Viewport;
use super::pipelines::{
    DrawUniform, Pipelines, PositionVertex, SpriteInstance, TexturedVertex, SEGMENT_VERTEX_COUNT,
};
use super::resources::{Mesh, Resources};
use super::theme::Theme;
use crate::model::RenderingError;
use renderer_types::Size;
use std::ops::Range;

/// 描画単位のパラメータ1つ分の領域。uniformバッファのオフセットのアライメント以上にする。
pub const UNIFORM_STRIDE: usize = 256;
/// 1フレームで発行できる描画の数
pub const UNIFORM_CAPACITY: usize = UNIFORM_STRIDE * 256;
/// 1フレームで使える動的な頂点・インスタンスデータの量
pub const VERTEX_CAPACITY: usize = 512 * 1024;

pub struct DynamicBuffers {
    pub uniforms: wgpu::Buffer,
    pub draw_bind_group: wgpu::BindGroup,
    pub vertices: wgpu::Buffer,
    pub tsunami: wgpu::Buffer,
    pub tsunami_bind_group: wgpu::BindGroup,
    pub glyph_texture: wgpu::Texture,
    pub glyph_bind_group: wgpu::BindGroup,
}

/// 地図をどこに・どの縮尺で描くか。
#[derive(Copy, Clone, Debug)]
pub struct View {
    offset: [f32; 2],
    /// カメラの縮尺。LODの選択に使う。
    pub zoom: f32,
    /// 描画先 (ビューポート) の大きさ
    pub size: Size<u32>,
}

impl View {
    pub fn from_camera(camera: &Camera) -> Self {
        Self {
            offset: camera.offset.into(),
            zoom: camera.scale,
            size: camera.image_size,
        }
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.size.aspect_ratio()
    }

    fn size_f32(&self) -> [f32; 2] {
        self.size.to_f32().into()
    }

    /// メルカトル座標 → クリップ座標の倍率
    fn map_scale(&self) -> [f32; 2] {
        [self.zoom, self.zoom / self.aspect_ratio()]
    }
}

pub struct Frame<'a> {
    pub resources: &'a Resources,
    pub theme: &'a Theme,
    pub view: View,

    pipelines: &'a Pipelines,
    dynamic: &'a DynamicBuffers,
    fonts: &'a Fonts,
    pass: wgpu::RenderPass<'a>,
    target_size: Size<u32>,

    staging: Staging,
    glyphs: GlyphAtlas,
}

#[derive(Default)]
struct Staging {
    uniforms: Vec<u8>,
    vertices: Vec<u8>,
    error: Option<RenderingError>,
}

impl Staging {
    fn fail(&mut self, error: RenderingError) {
        self.error.get_or_insert(error);
    }

    /// 描画1回分のパラメータを積み、uniformバッファ上のオフセットを返す。
    fn push_uniform(&mut self, uniform: &DrawUniform) -> Option<u32> {
        if self.error.is_some() {
            return None;
        }

        let at = self.uniforms.len();
        if at + UNIFORM_STRIDE > UNIFORM_CAPACITY {
            self.fail(RenderingError::TooManyDraws);
            return None;
        }

        self.uniforms.extend_from_slice(bytemuck::bytes_of(uniform));
        self.uniforms.resize(at + UNIFORM_STRIDE, 0);

        Some(at as u32)
    }

    /// 頂点・インスタンスデータを積み、頂点バッファ上の範囲を返す。
    fn push_vertices<T: bytemuck::Pod>(&mut self, data: &[T]) -> Option<Range<u64>> {
        if self.error.is_some() {
            return None;
        }

        let bytes: &[u8] = bytemuck::cast_slice(data);
        let start = self.vertices.len();
        if start + bytes.len() > VERTEX_CAPACITY {
            self.fail(RenderingError::TooMuchGeometry);
            return None;
        }

        self.vertices.extend_from_slice(bytes);

        Some(start as u64..self.vertices.len() as u64)
    }
}

pub struct FrameData {
    pub uniforms: Vec<u8>,
    pub vertices: Vec<u8>,
    pub glyphs: GlyphAtlas,
}

impl<'a> Frame<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        resources: &'a Resources,
        pipelines: &'a Pipelines,
        dynamic: &'a DynamicBuffers,
        fonts: &'a Fonts,
        theme: &'a Theme,
        pass: wgpu::RenderPass<'a>,
        view: View,
    ) -> Self {
        let mut frame = Self {
            resources,
            theme,
            view,
            pipelines,
            dynamic,
            fonts,
            pass,
            target_size: view.size,
            staging: Staging::default(),
            glyphs: GlyphAtlas::new(),
        };

        frame.set_view(view, None);
        frame.set_stencil_reference(0);
        frame
    }

    pub fn finish(self) -> Result<FrameData, RenderingError> {
        if let Some(error) = self.staging.error {
            return Err(error);
        }

        Ok(FrameData {
            uniforms: self.staging.uniforms,
            vertices: self.staging.vertices,
            glyphs: self.glyphs,
        })
    }

    /// 以降の描画に使うカメラと描画先の矩形を切り替える。`None` なら画像全体。
    pub fn set_view(&mut self, view: View, viewport: Option<Viewport>) {
        self.view = view;

        let (x, y, width, height) = match viewport {
            // ビューポートの定義は左下原点、GPUは左上原点
            Some(v) => (
                v.left,
                self.target_size.y() - v.bottom - v.height,
                v.width,
                v.height,
            ),
            None => (0, 0, self.target_size.x(), self.target_size.y()),
        };

        self.pass
            .set_viewport(x as f32, y as f32, width as f32, height as f32, 0.0, 1.0);
        // ビューポートだけでは外側への描画が切り落とされる保証がない
        self.pass.set_scissor_rect(x, y, width, height);
    }

    /// 以降の描画は、ステンシルがこの値の画素にだけ行われる。
    pub fn set_stencil_reference(&mut self, reference: u32) {
        self.pass.set_stencil_reference(reference);
    }

    /// 地図のポリゴンを単色で塗る。
    pub fn fill_mesh(&mut self, mesh: &'a Mesh, indices: Range<u32>, color: [f32; 3]) {
        if indices.is_empty() {
            return;
        }

        if !self.bind_draw(
            self.view.offset,
            self.view.map_scale(),
            [0.0; 2],
            opaque(color),
        ) {
            return;
        }
        self.pass.set_pipeline(&self.pipelines.fill);
        self.pass.set_vertex_buffer(0, mesh.vertices.slice(..));
        self.pass
            .set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        self.pass.draw_indexed(indices, 0, 0..1);
    }

    /// ビューポート内のクリップ座標で与えた三角形リストを単色で塗る。
    pub fn fill_shape(&mut self, triangles: &[[f32; 2]], color: [f32; 4]) {
        self.draw_shape(triangles, color, false);
    }

    /// ビューポート内のクリップ座標で与えた三角形リストの範囲のステンシルを、
    /// 現在の参照値で上書きする。色は書かない。
    pub fn write_stencil(&mut self, triangles: &[[f32; 2]]) {
        self.draw_shape(triangles, [0.0; 4], true);
    }

    fn draw_shape(&mut self, triangles: &[[f32; 2]], color: [f32; 4], stencil_only: bool) {
        if triangles.is_empty() {
            return;
        }

        let vertices: Vec<_> = triangles
            .iter()
            .map(|&position| PositionVertex { position })
            .collect();
        let Some(range) = self.staging.push_vertices(&vertices) else {
            return;
        };

        if !self.bind_draw([0.0; 2], [1.0; 2], [0.0; 2], color) {
            return;
        }
        self.pass.set_pipeline(if stencil_only {
            &self.pipelines.mask
        } else {
            &self.pipelines.fill
        });
        self.pass
            .set_vertex_buffer(0, self.dynamic.vertices.slice(range));
        self.pass.draw(0..vertices.len() as u32, 0..1);
    }

    /// 境界線を画面上で一定の太さ (px) で描く。
    pub fn border_lines(&mut self, segments: Range<u32>, width: f32, color: [f32; 3]) {
        if segments.is_empty() {
            return;
        }

        if !self.bind_draw(
            self.view.offset,
            self.view.map_scale(),
            [width, 0.0],
            opaque(color),
        ) {
            return;
        }
        self.pass.set_pipeline(&self.pipelines.line);
        self.pass
            .set_vertex_buffer(0, self.resources.border_segments.slice(..));
        self.pass.draw(0..SEGMENT_VERTEX_COUNT, segments);
    }

    /// 津波予報区の線のうち、発表レベルが `level` (1以上) の予報区の線分範囲 `segments` を、
    /// そのレベルの色で描く。
    pub fn tsunami_lines(&mut self, segments: &[Range<u32>], width: f32, level: u32) {
        if segments.iter().all(Range::is_empty) {
            return;
        }

        if !self.bind_draw(
            self.view.offset,
            self.view.map_scale(),
            [width, level as f32],
            [0.0; 4],
        ) {
            return;
        }
        self.pass.set_pipeline(&self.pipelines.tsunami_line);
        self.pass
            .set_bind_group(1, &self.dynamic.tsunami_bind_group, &[]);
        self.pass
            .set_vertex_buffer(0, self.resources.tsunami_segments.slice(..));
        for range in segments {
            if !range.is_empty() {
                self.pass.draw(0..SEGMENT_VERTEX_COUNT, range.clone());
            }
        }
    }

    /// 地図上の位置を中心に、画面上で一定の大きさのスプライトを描く。
    /// `half_size` はクリップ座標での半分の大きさ。
    pub fn sprites(
        &mut self,
        texture: &'a wgpu::BindGroup,
        instances: &[SpriteInstance],
        half_size: [f32; 2],
    ) {
        if instances.is_empty() {
            return;
        }

        let Some(range) = self.staging.push_vertices(instances) else {
            return;
        };

        if !self.bind_draw(self.view.offset, self.view.map_scale(), half_size, [1.0; 4]) {
            return;
        }
        self.pass.set_pipeline(&self.pipelines.sprite);
        self.pass.set_bind_group(1, texture, &[]);
        self.pass
            .set_vertex_buffer(0, self.dynamic.vertices.slice(range));
        self.pass.draw(0..6, 0..instances.len() as u32);
    }

    /// ビューポート内のクリップ座標で与えた三角形リストにテクスチャを貼る。
    /// `tint` はプリマルチプライド済みの乗算色。
    pub fn textured(
        &mut self,
        texture: &'a wgpu::BindGroup,
        vertices: &[TexturedVertex],
        tint: [f32; 4],
    ) {
        if vertices.is_empty() {
            return;
        }

        let Some(range) = self.staging.push_vertices(vertices) else {
            return;
        };

        if !self.bind_draw([0.0; 2], [1.0; 2], [0.0; 2], tint) {
            return;
        }
        self.pass.set_pipeline(&self.pipelines.textured);
        self.pass.set_bind_group(1, texture, &[]);
        self.pass
            .set_vertex_buffer(0, self.dynamic.vertices.slice(range));
        self.pass.draw(0..vertices.len() as u32, 0..1);
    }

    /// 現在のビューポートに文字列を描く。`color` はストレートアルファ。
    pub fn text(&mut self, text: &str, font: Font, color: [f32; 4], size: f32, offset: Offset) {
        let dimension: (u32, u32) = self.view.size.into();
        let glyphs = self.fonts.layout(text, font, size, offset, dimension);

        let to_clip = |x: f32, y: f32| {
            [
                x / dimension.0 as f32 * 2.0 - 1.0,
                y / dimension.1 as f32 * -2.0 + 1.0,
            ]
        };
        let atlas_size = GLYPH_ATLAS_SIZE as f32;

        let mut vertices = Vec::with_capacity(glyphs.len() * 6);
        for glyph in &glyphs {
            let rect = match self.glyphs.insert(glyph) {
                Ok(Some(rect)) => rect,
                Ok(None) => continue,
                Err(_) => {
                    self.staging.fail(RenderingError::TooMuchText);
                    return;
                }
            };
            let bounds = glyph.px_bounds();

            let (left, top) = (rect.x as f32 / atlas_size, rect.y as f32 / atlas_size);
            let (right, bottom) = (
                (rect.x + rect.width) as f32 / atlas_size,
                (rect.y + rect.height) as f32 / atlas_size,
            );

            let corner = |x: f32, y: f32, u: f32, v: f32| TexturedVertex {
                position: to_clip(x, y),
                uv: [u, v],
            };
            let left_top = corner(bounds.min.x, bounds.min.y, left, top);
            let right_top = corner(bounds.max.x, bounds.min.y, right, top);
            let left_bottom = corner(bounds.min.x, bounds.max.y, left, bottom);
            let right_bottom = corner(bounds.max.x, bounds.max.y, right, bottom);

            vertices.extend([
                left_top,
                right_top,
                left_bottom,
                left_bottom,
                right_top,
                right_bottom,
            ]);
        }

        let tint = [
            color[0] * color[3],
            color[1] * color[3],
            color[2] * color[3],
            color[3],
        ];
        self.textured(&self.dynamic.glyph_bind_group, &vertices, tint);
    }

    #[must_use]
    fn bind_draw(
        &mut self,
        offset: [f32; 2],
        scale: [f32; 2],
        param: [f32; 2],
        color: [f32; 4],
    ) -> bool {
        let Some(at) = self.staging.push_uniform(&DrawUniform {
            offset,
            scale,
            viewport: self.view.size_f32(),
            param,
            color,
        }) else {
            return false;
        };

        self.pass
            .set_bind_group(0, &self.dynamic.draw_bind_group, &[at]);

        true
    }
}

fn opaque(color: [f32; 3]) -> [f32; 4] {
    [color[0], color[1], color[2], 1.0]
}
