use ab_glyph::{point, Font as _, FontRef, GlyphId, OutlinedGlyph, Point, ScaleFont};
use std::cmp::{max, min};
use strum::{EnumIter, IntoEnumIterator};

pub const GLYPH_ATLAS_SIZE: u32 = 512;

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

pub struct Fonts {
    fonts: Vec<FontRef<'static>>,
}

impl Fonts {
    pub fn new() -> Self {
        let fonts = Font::iter()
            .map(|f| FontRef::try_from_slice(f.font_binary()).unwrap())
            .collect();

        Self { fonts }
    }

    pub fn layout(
        &self,
        text: &str,
        font: Font,
        size: f32,
        offset: Offset,
        image_dimension: (u32, u32),
    ) -> Vec<OutlinedGlyph> {
        let font = self.fonts[font.font_id()].as_scaled(size);
        let ascent = font.ascent();
        let descent = font.descent();
        let line_gap = font.line_gap();

        let layout_line = |line: &str, origin: Point| -> Vec<OutlinedGlyph> {
            let mut caret = origin.x;
            let mut previous: Option<GlyphId> = None;
            let mut glyphs = Vec::new();

            for c in line.chars() {
                let id = font.glyph_id(c);
                if let Some(previous) = previous {
                    caret += font.kern(previous, id);
                }

                let glyph = id.with_scale_and_position(size, point(caret, origin.y));
                caret += font.h_advance(id);
                previous = Some(id);

                glyphs.extend(font.outline_glyph(glyph));
            }

            glyphs
        };

        let lines: Vec<&str> = text.lines().collect();

        let text_width = lines
            .iter()
            .map(|line| {
                let mut min_x = i32::MAX;
                let mut max_x = i32::MIN;

                for glyph in layout_line(line, point(0.0, ascent)) {
                    let bounds = glyph.px_bounds();
                    min_x = min(min_x, bounds.min.x as i32);
                    max_x = max(max_x, bounds.max.x as i32);
                }

                max_x - min_x
            })
            .max()
            .unwrap_or(0);

        let line_height = ascent - descent;

        lines
            .iter()
            .enumerate()
            .flat_map(|(i, line)| {
                let origin = offset.glyph_offset(
                    image_dimension,
                    (text_width as f32, line_height),
                    ascent + (line_height + line_gap) * (i as f32 - lines.len() as f32 + 1.0),
                );

                layout_line(line, origin)
            })
            .collect()
    }
}

pub struct GlyphAtlas {
    pixels: Vec<u8>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AtlasFull;

impl GlyphAtlas {
    const PADDING: u32 = 1;

    pub fn new() -> Self {
        Self {
            pixels: vec![0; (GLYPH_ATLAS_SIZE * GLYPH_ATLAS_SIZE * 4) as usize],
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
        }
    }

    pub fn insert(&mut self, glyph: &OutlinedGlyph) -> Result<Option<AtlasRect>, AtlasFull> {
        let bounds = glyph.px_bounds();
        let width = bounds.width() as u32;
        let height = bounds.height() as u32;

        if width == 0 || height == 0 {
            return Ok(None);
        }

        if width > GLYPH_ATLAS_SIZE {
            return Err(AtlasFull);
        }

        if self.cursor_x + width > GLYPH_ATLAS_SIZE {
            self.cursor_x = 0;
            self.cursor_y += self.row_height + Self::PADDING;
            self.row_height = 0;
        }

        if self.cursor_y + height > GLYPH_ATLAS_SIZE {
            return Err(AtlasFull);
        }

        let rect = AtlasRect {
            x: self.cursor_x,
            y: self.cursor_y,
            width,
            height,
        };

        glyph.draw(|x, y, coverage| {
            if x >= width || y >= height {
                return;
            }

            let value = (coverage.clamp(0.0, 1.0) * 255.0).round() as u8;
            let index = (((rect.y + y) * GLYPH_ATLAS_SIZE + rect.x + x) * 4) as usize;
            self.pixels[index..index + 4].fill(value);
        });

        self.cursor_x += width + Self::PADDING;
        self.row_height = max(self.row_height, height);

        Ok(Some(rect))
    }

    pub fn used(&self) -> Option<(u32, &[u8])> {
        let rows = self.cursor_y + self.row_height;
        if rows == 0 {
            return None;
        }

        Some((rows, &self.pixels[..(rows * GLYPH_ATLAS_SIZE * 4) as usize]))
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum Origin {
    LeftUp,
    LeftDown,
    RightUp,
    RightDown,
    Center,
    CenterDown,
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
        text_size: (f32, f32),
        ascent: f32,
    ) -> Point {
        let x = self.x as f32;
        let y = self.y as f32;

        let image_dimension = (image_dimension.0 as f32, image_dimension.1 as f32);
        let offset = match self.image_origin {
            Origin::LeftUp => (x, y),
            Origin::LeftDown => (x, y + image_dimension.1),
            Origin::RightUp => (x + image_dimension.0, y),
            Origin::RightDown => (x + image_dimension.0, y + image_dimension.1),
            Origin::Center => (x + image_dimension.0 / 2.0, y + image_dimension.1 / 2.0),
            Origin::CenterDown => (x + image_dimension.0 / 2.0, y + image_dimension.1),
        };

        let text_size = (text_size.0, text_size.1);
        let offset = match self.text_origin {
            Origin::LeftUp => (offset.0, offset.1),
            Origin::LeftDown => (offset.0, offset.1 - text_size.1),
            Origin::RightUp => (offset.0 - text_size.0, offset.1),
            Origin::RightDown => (offset.0 - text_size.0, offset.1 - text_size.1),
            Origin::Center => (offset.0 - text_size.0 / 2.0, offset.1 - text_size.1 / 2.0),
            Origin::CenterDown => (offset.0 - text_size.0 / 2.0, offset.1 - text_size.1),
        };

        point(offset.0, offset.1 + ascent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset() {
        let offset = Offset::new(Origin::LeftUp, Origin::LeftUp, 0, 0);
        assert_eq!(
            offset.glyph_offset((100, 100), (10.0, 10.0), 0.0),
            point(0.0, 0.0)
        );

        let offset = Offset::new(Origin::RightUp, Origin::RightUp, 0, 0);
        assert_eq!(
            offset.glyph_offset((100, 100), (10.0, 10.0), 0.0),
            point(90.0, 0.0)
        );

        let offset = Offset::new(Origin::LeftDown, Origin::LeftDown, 0, 0);
        assert_eq!(
            offset.glyph_offset((100, 100), (10.0, 10.0), 0.0),
            point(0.0, 90.0)
        );

        let offset = Offset::new(Origin::RightDown, Origin::RightDown, 0, 0);
        assert_eq!(
            offset.glyph_offset((100, 100), (10.0, 10.0), 0.0),
            point(90.0, 90.0)
        );

        let offset = Offset::new(Origin::Center, Origin::Center, 0, 0);
        assert_eq!(
            offset.glyph_offset((100, 100), (10.0, 10.0), 0.0),
            point(45.0, 45.0)
        );
    }

    #[test]
    fn layout_places_text_at_the_anchor() {
        let fonts = Fonts::new();
        let glyphs = fonts.layout(
            "2024年01月01日",
            Font::BizUDPGothicBold,
            20.0,
            Offset::new(Origin::RightDown, Origin::RightDown, -20, -20),
            (1024, 768),
        );

        assert_eq!(glyphs.len(), 11);

        let max_x = glyphs
            .iter()
            .map(|g| g.px_bounds().max.x)
            .fold(f32::MIN, f32::max);
        let max_y = glyphs
            .iter()
            .map(|g| g.px_bounds().max.y)
            .fold(f32::MIN, f32::max);

        assert!((1000.0..=1005.0).contains(&max_x), "max_x = {max_x}");
        assert!((735.0..=748.0).contains(&max_y), "max_y = {max_y}");
    }

    #[test]
    fn atlas_packs_glyphs_without_overlap() {
        let fonts = Fonts::new();
        let glyphs = fonts.layout(
            "津波注意報",
            Font::BizUDPGothicBold,
            22.0,
            Offset::new(Origin::LeftUp, Origin::LeftUp, 0, 0),
            (1024, 768),
        );

        let mut atlas = GlyphAtlas::new();
        let rects: Vec<_> = glyphs
            .iter()
            .filter_map(|g| atlas.insert(g).unwrap())
            .collect();

        assert_eq!(rects.len(), 5);
        for pair in rects.windows(2) {
            assert!(pair[0].x + pair[0].width < pair[1].x || pair[0].y != pair[1].y);
        }

        let (rows, pixels) = atlas.used().unwrap();
        assert!(rows > 0);
        assert!(pixels.contains(&255));
    }
}
