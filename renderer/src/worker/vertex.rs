use glium::uniforms::{AsUniformValue, Sampler, UniformValue, Uniforms};
use glium::{implement_vertex, Texture2d};

#[derive(Debug)]
pub struct BorderLineUniform {
    pub dimension: [f32; 2],
    pub offset: [f32; 2],
    pub zoom: f32,
    pub line_width: f32,
    pub color: [f32; 3],
}

impl Uniforms for BorderLineUniform {
    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, mut visitor: F) {
        visitor("dimension", self.dimension.as_uniform_value());
        visitor("offset", self.offset.as_uniform_value());
        visitor("zoom", self.zoom.as_uniform_value());
        visitor("line_width", self.line_width.as_uniform_value());
        visitor("color", self.color.as_uniform_value());
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EpicenterVertex {
    pub position: [f32; 2],
}
implement_vertex!(EpicenterVertex, position);

#[derive(Debug)]
pub struct EpicenterUniform<'a> {
    pub aspect_ratio: f32,
    pub offset: [f32; 2],
    pub zoom: f32,
    pub icon_ratio_in_y_axis: f32,
    pub texture_sampler: &'a Texture2d,
}

impl Uniforms for EpicenterUniform<'_> {
    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, mut visitor: F) {
        visitor("aspect_ratio", self.aspect_ratio.as_uniform_value());
        visitor("offset", self.offset.as_uniform_value());
        visitor("zoom", self.zoom.as_uniform_value());
        visitor(
            "icon_ratio_in_y_axis",
            self.icon_ratio_in_y_axis.as_uniform_value(),
        );
        visitor("texture_sampler", self.texture_sampler.as_uniform_value());
    }
}

#[derive(Copy, Clone, Debug)]
pub struct IntensityIconVertex {
    pub position: [f32; 2],
    pub uv_offset: [f32; 2],
}
implement_vertex!(IntensityIconVertex, position, uv_offset);

#[derive(Debug)]
pub struct IntensityIconUniform<'a> {
    pub aspect_ratio: f32,
    pub offset: [f32; 2],
    pub zoom: f32,
    pub icon_ratio_in_y_axis: f32,
    pub texture_sampler: &'a Texture2d,
}

impl Uniforms for IntensityIconUniform<'_> {
    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, mut visitor: F) {
        visitor("aspect_ratio", self.aspect_ratio.as_uniform_value());
        visitor("offset", self.offset.as_uniform_value());
        visitor("zoom", self.zoom.as_uniform_value());
        visitor(
            "icon_ratio_in_y_axis",
            self.icon_ratio_in_y_axis.as_uniform_value(),
        );
        visitor("texture_sampler", self.texture_sampler.as_uniform_value());
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MapVertex {
    pub position: [f32; 2],
}
implement_vertex!(MapVertex, position);

#[derive(Debug)]
pub struct MapUniform {
    pub aspect_ratio: f32,
    pub offset: [f32; 2],
    pub zoom: f32,
    pub color: [f32; 3],
}

impl Uniforms for MapUniform {
    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, mut visitor: F) {
        visitor("aspect_ratio", self.aspect_ratio.as_uniform_value());
        visitor("offset", self.offset.as_uniform_value());
        visitor("zoom", self.zoom.as_uniform_value());
        visitor("color", self.color.as_uniform_value());
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TextVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}
implement_vertex!(TextVertex, position, uv);

#[derive(Debug)]
pub struct TextUniform<'a> {
    pub font_texture: &'a Sampler<'a, Texture2d>,
    pub color: [f32; 4],
}

impl Uniforms for TextUniform<'_> {
    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, mut visitor: F) {
        visitor("font_texture", self.font_texture.as_uniform_value());
        visitor("color", self.color.as_uniform_value());
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TexturedVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}
implement_vertex!(TexturedVertex, position, uv);

#[derive(Debug)]
pub struct TexturedUniform<'a> {
    pub texture_sampler: &'a Texture2d,
}

impl Uniforms for TexturedUniform<'_> {
    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, mut visitor: F) {
        visitor("texture_sampler", self.texture_sampler.as_uniform_value());
    }
}
