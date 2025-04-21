use crate::worker::resources::Resources;
use crate::worker::vertex::TextVertex;
use glium::backend::Facade;
use glium::index::PrimitiveType;
use glium::texture::{ClientFormat, MipmapsOption, RawImage2d, UncompressedFloatFormat};
use glium::uniforms::MagnifySamplerFilter;
use glium::{uniform, DrawParameters, IndexBuffer, Surface, Texture2d, VertexBuffer};
use rusttype::gpu_cache::{Cache, CacheBuilder};
use rusttype::{point, Point, Scale};
use std::borrow::Cow;
use std::cmp::{max, min};
use strum::{EnumIter, IntoEnumIterator};

const FONT_CACHE_SIZE: u32 = 512;

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Font {
    BizUDPGothicBold,
}

trait FontInfo {
    fn font_id(&self) -> usize;

    fn font_binary(&self) -> &'static [u8];
}

impl FontInfo for Font {
    fn font_id(&self) -> usize {
        *self as usize
    }

    fn font_binary(&self) -> &'static [u8] {
        match self {
            Font::BizUDPGothicBold => {
                include_bytes!("../../../assets/font/biz-udpgothic/BIZUDPGothic-Bold.ttf")
            }
        }
    }
}

pub struct FontManager<'a> {
    fonts: Vec<rusttype::Font<'a>>,
    font_cache: Cache<'a>,
    font_cache_texture: Texture2d,
}

impl FontManager<'_> {
    pub fn new<F: ?Sized + Facade>(facade: &F) -> Self {
        let fonts: Vec<_> = Font::iter()
            .map(|f| rusttype::Font::try_from_bytes(f.font_binary()).unwrap())
            .collect();
        let font_cache = CacheBuilder::default()
            .dimensions(FONT_CACHE_SIZE, FONT_CACHE_SIZE)
            .multithread(true)
            .build();
        let image = RawImage2d {
            data: Cow::Owned(vec![0u8; (FONT_CACHE_SIZE * FONT_CACHE_SIZE) as usize]),
            width: FONT_CACHE_SIZE,
            height: FONT_CACHE_SIZE,
            format: ClientFormat::U8,
        };
        let font_cache_texture = Texture2d::with_format(
            facade,
            image,
            UncompressedFloatFormat::U8,
            MipmapsOption::NoMipmap,
        )
        .unwrap();

        Self {
            fonts,
            font_cache,
            font_cache_texture,
        }
    }

    pub fn draw_text<F: ?Sized + Facade, S: ?Sized + Surface>(
        &mut self,
        text: &str,
        font: Font,
        color: [f32; 4],
        scale: Scale,
        offset: Offset,
        image_dimension: (u32, u32),
        resources: &Resources,
        facade: &F,
        surface: &mut S,
        draw_params: &DrawParameters,
    ) {
        let font_id = font.font_id();
        let font = &self.fonts[font_id];

        let glyph_offset = point(0.0, font.v_metrics(scale).ascent);
        let glyphs: Vec<_> = font.layout(text, scale, glyph_offset).collect();

        let (text_width, text_height) = {
            let mut min_x = i32::MAX;
            let mut max_x = i32::MIN;
            let mut min_y = i32::MAX;
            let mut max_y = i32::MIN;

            glyphs
                .iter()
                .filter_map(|glyph| glyph.pixel_bounding_box())
                .for_each(|rect| {
                    min_x = min(min_x, rect.min.x);
                    max_x = max(max_x, rect.max.x);
                    min_y = min(min_y, rect.min.y);
                    max_y = max(max_y, rect.max.y);
                });

            ((max_x - min_x) as u32, (max_y - min_y) as u32)
        };

        let glyph_offset = offset.glyph_offset(
            image_dimension,
            (text_width, text_height),
            font.v_metrics(scale).ascent,
        );
        let glyphs: Vec<_> = font.layout(text, scale, glyph_offset).collect();

        for glyph in &glyphs {
            self.font_cache.queue_glyph(font_id, glyph.clone())
        }
        self.font_cache
            .cache_queued(|rect, data| {
                self.font_cache_texture.main_level().write(
                    glium::Rect {
                        left: rect.min.x,
                        bottom: rect.min.y,
                        width: rect.width(),
                        height: rect.height(),
                    },
                    RawImage2d {
                        data: Cow::Borrowed(data),
                        width: rect.width(),
                        height: rect.height(),
                        format: ClientFormat::U8,
                    },
                );
            })
            .unwrap();

        let glyph_rects: Vec<_> = glyphs
            .iter()
            .filter_map(|glyph| self.font_cache.rect_for(font_id, glyph).ok().flatten())
            .collect();
        let vertices: Vec<TextVertex> = glyph_rects
            .into_iter()
            .flat_map(|(uv_rect, screen_rect)| {
                let min = (
                    screen_rect.min.x as f32 / image_dimension.0 as f32 * 2.0 - 1.0,
                    screen_rect.min.y as f32 / image_dimension.1 as f32 * -2.0 + 1.0,
                );
                let max = (
                    screen_rect.max.x as f32 / image_dimension.0 as f32 * 2.0 - 1.0,
                    screen_rect.max.y as f32 / image_dimension.1 as f32 * -2.0 + 1.0,
                );

                vec![
                    TextVertex::new((min.0, min.1), (uv_rect.min.x, uv_rect.min.y)),
                    TextVertex::new((max.0, min.1), (uv_rect.max.x, uv_rect.min.y)),
                    TextVertex::new((min.0, max.1), (uv_rect.min.x, uv_rect.max.y)),
                    TextVertex::new((max.0, max.1), (uv_rect.max.x, uv_rect.max.y)),
                ]
            })
            .collect();
        let indices: Vec<_> = (0..vertices.len() as u32 / 4)
            .map(|v| v * 4)
            .scan(false, |&mut first_time, v| {
                if first_time {
                    Some(vec![v, v + 1, v + 2, v + 3])
                } else {
                    Some(vec![v - 1, v, v, v + 1, v + 2, v + 3])
                }
            })
            .flatten()
            .collect();

        let vertex_buffer = VertexBuffer::dynamic(facade, &vertices).unwrap();
        let index_buffer =
            IndexBuffer::dynamic(facade, PrimitiveType::TriangleStrip, &indices).unwrap();
        let uniforms = uniform! {
            font_texture: self.font_cache_texture.sampled().magnify_filter(MagnifySamplerFilter::Nearest),
            color: color,
        };

        surface
            .draw(
                &vertex_buffer,
                &index_buffer,
                &resources.shader.text,
                &uniforms,
                draw_params,
            )
            .unwrap();
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Origin {
    LeftUp,
    LeftDown,
    RightUp,
    RightDown,
    Center,
}

#[derive(Copy, Clone, Debug)]
pub struct Offset {
    image_origin: Origin,
    text_origin: Origin,
    x: i32,
    y: i32,
}

impl Offset {
    pub fn new(image_origin: Origin, text_origin: Origin, x: i32, y: i32) -> Self {
        Self {
            image_origin,
            text_origin,
            x,
            y,
        }
    }

    fn glyph_offset(
        &self,
        image_dimension: (u32, u32),
        text_size: (u32, u32),
        ascent: f32,
    ) -> Point<f32> {
        let x = self.x as f32;
        let y = self.y as f32;

        let image_dimension = (image_dimension.0 as f32, image_dimension.1 as f32);
        let offset = match self.image_origin {
            Origin::LeftUp => (x, y),
            Origin::LeftDown => (x, y + image_dimension.1),
            Origin::RightUp => (x + image_dimension.0, y),
            Origin::RightDown => (x + image_dimension.0, y + image_dimension.1),
            Origin::Center => (x + image_dimension.0 / 2.0, y + image_dimension.1 / 2.0),
        };

        let text_size = (text_size.0 as f32, text_size.1 as f32);
        let offset = match self.text_origin {
            Origin::LeftUp => (offset.0, offset.1),
            Origin::LeftDown => (offset.0, offset.1 - text_size.1),
            Origin::RightUp => (offset.0 - text_size.0, offset.1),
            Origin::RightDown => (offset.0 - text_size.0, offset.1 - text_size.1),
            Origin::Center => (offset.0 - text_size.0 / 2.0, offset.1 - text_size.1 / 2.0),
        };

        point(offset.0, offset.1 + ascent)
    }
}

#[cfg(test)]
mod tests {
    use crate::worker::fonts::{Offset, Origin};
    use rusttype::point;

    #[test]
    fn test_offset() {
        let offset = Offset::new(Origin::LeftUp, Origin::LeftUp, 0, 0);
        assert_eq!(
            offset.glyph_offset((100, 100), (10, 10), 0.0),
            point(0.0, 0.0)
        );

        let offset = Offset::new(Origin::RightUp, Origin::RightUp, 0, 0);
        assert_eq!(
            offset.glyph_offset((100, 100), (10, 10), 0.0),
            point(90.0, 0.0)
        );

        let offset = Offset::new(Origin::LeftDown, Origin::LeftDown, 0, 0);
        assert_eq!(
            offset.glyph_offset((100, 100), (10, 10), 0.0),
            point(0.0, 90.0)
        );

        let offset = Offset::new(Origin::RightDown, Origin::RightDown, 0, 0);
        assert_eq!(
            offset.glyph_offset((100, 100), (10, 10), 0.0),
            point(90.0, 90.0)
        );

        let offset = Offset::new(Origin::Center, Origin::Center, 0, 0);
        assert_eq!(
            offset.glyph_offset((100, 100), (10, 10), 0.0),
            point(45.0, 45.0)
        );
    }
}
