use glium::uniforms::{AsUniformValue, Sampler, UniformBlock, UniformBuffer, UniformValue, Uniforms};
use glium::{implement_uniform_block, implement_vertex, Texture2d};
use glium::backend::Facade;
use glium::texture::Texture1d;

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
pub struct TsunamiVertex {
    pub position: [f32; 2],
    pub code: u16,
}
implement_vertex!(TsunamiVertex, position, code);

#[derive(Debug)]
pub struct TsunamiUniform {
    pub dimension: [f32; 2],
    pub offset: [f32; 2],
    pub zoom: f32,
    pub colors: UniformBuffer<TsunamiLineColors>,
    pub levels: Texture1d,
    pub line_width: f32,
}

impl TsunamiUniform {
    pub fn new<F: ?Sized + Facade>(
        facade: &F,
        dimension: [f32; 2],
        offset: [f32; 2],
        zoom: f32,
        colors: TsunamiLineColors,
        levels: Texture1d,
        line_width: f32,
    ) -> Self {
        let colors = UniformBuffer::dynamic(facade, colors).unwrap();

        Self {
            dimension,
            offset,
            zoom,
            colors,
            levels,
            line_width,
        }
    }
}

impl Uniforms for TsunamiUniform {
    fn visit_values<'a, Fn: FnMut(&str, UniformValue<'a>)>(&'a self, mut visitor: Fn) {
        visitor("dimension", self.dimension.as_uniform_value());
        visitor("offset", self.offset.as_uniform_value());
        visitor("zoom", self.zoom.as_uniform_value());
        let colors = UniformValue::Block(self.colors.as_slice_any(), |block| TsunamiLineColors::matches(&block.layout, 0));
        visitor("colors", colors);
        visitor("levels", self.levels.as_uniform_value());
        visitor("line_width", self.line_width.as_uniform_value());
        // meow
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TsunamiLineColors {
    pub forecast: [f32; 3],
    pub advisory: [f32; 3],
    pub warning: [f32; 3],
    pub major_warning: [f32; 3],
}
implement_uniform_block!(TsunamiLineColors, forecast, advisory, warning, major_warning);

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
