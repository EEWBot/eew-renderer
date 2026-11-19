use ab_glyph::{point, Font as _, FontRef, GlyphId, OutlinedGlyph, Point, ScaleFont};
use std::cmp::{max, min};
use std::collections::HashMap;
use std::ops::Range;
use strum::{EnumIter, IntoEnumIterator};

pub const GLYPH_ATLAS_SIZE: u32 = 512;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, EnumIter)]
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

const SUBPIXEL_STEPS: i32 = 4;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    font: Font,
    id: GlyphId,
    size_bits: u32,
    subpixel: [u8; 2],
}

impl GlyphKey {
    fn size(&self) -> f32 {
        f32::from_bits(self.size_bits)
    }

    fn subpixel_offset(&self) -> Point {
        let step = SUBPIXEL_STEPS as f32;
        point(
            self.subpixel[0] as f32 / step,
            self.subpixel[1] as f32 / step,
        )
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PlacedGlyph {
    pub key: GlyphKey,
    pub origin: [i32; 2],
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
        font_kind: Font,
        size: f32,
        offset: Offset,
        image_dimension: (u32, u32),
    ) -> Vec<PlacedGlyph> {
        let font = self.fonts[font_kind.font_id()].as_scaled(size);
        let ascent = font.ascent();
        let descent = font.descent();
        let line_gap = font.line_gap();

        let layout_line = |line: &str, origin: Point| -> Vec<(GlyphId, Point)> {
            let mut caret = origin.x;
            let mut previous: Option<GlyphId> = None;
            let mut glyphs = Vec::new();

            for c in line.chars() {
                let id = font.glyph_id(c);
                if let Some(previous) = previous {
                    caret += font.kern(previous, id);
                }

                glyphs.push((id, point(caret, origin.y)));
                caret += font.h_advance(id);
                previous = Some(id);
            }

            glyphs
        };

        let lines: Vec<&str> = text.lines().collect();

        let text_width = if !offset.text_origin.uses_width() {
            0
        } else {
            lines
                .iter()
                .map(|line| {
                    let mut min_x = i32::MAX;
                    let mut max_x = i32::MIN;

                    for (id, position) in layout_line(line, point(0.0, ascent)) {
                        let glyph = id.with_scale_and_position(size, position);
                        let Some(outlined) = font.outline_glyph(glyph) else {
                            continue;
                        };
                        let bounds = outlined.px_bounds();
                        min_x = min(min_x, bounds.min.x as i32);
                        max_x = max(max_x, bounds.max.x as i32);
                    }

                    max_x - min_x
                })
                .max()
                .unwrap_or(0)
        };

        let line_height = ascent - descent;

        let place = |id: GlyphId, position: Point| -> PlacedGlyph {
            let split = |v: f32| {
                let steps = (v * SUBPIXEL_STEPS as f32).round() as i32;
                (
                    steps.div_euclid(SUBPIXEL_STEPS),
                    steps.rem_euclid(SUBPIXEL_STEPS) as u8,
                )
            };
            let (x, sub_x) = split(position.x);
            let (y, sub_y) = split(position.y);

            PlacedGlyph {
                key: GlyphKey {
                    font: font_kind,
                    id,
                    size_bits: size.to_bits(),
                    subpixel: [sub_x, sub_y],
                },
                origin: [x, y],
            }
        };

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
            .map(|(id, position)| place(id, position))
            .collect()
    }

    fn outline(&self, key: &GlyphKey) -> Option<OutlinedGlyph> {
        let font = &self.fonts[key.font.font_id()];
        font.outline_glyph(
            key.id
                .with_scale_and_position(key.size(), key.subpixel_offset()),
        )
    }
}

pub struct GlyphCache {
    pixels: Vec<u8>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    entries: HashMap<GlyphKey, Option<CachedGlyph>>,
    dirty: Option<Range<u32>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CachedGlyph {
    pub rect: AtlasRect,
    pub offset: [i32; 2],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AtlasFull;

impl GlyphCache {
    const PADDING: u32 = 1;

    pub fn new() -> Self {
        Self {
            pixels: vec![0; (GLYPH_ATLAS_SIZE * GLYPH_ATLAS_SIZE * 4) as usize],
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            entries: HashMap::new(),
            dirty: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get_or_insert(
        &mut self,
        fonts: &Fonts,
        key: GlyphKey,
    ) -> Result<Option<CachedGlyph>, AtlasFull> {
        if let Some(entry) = self.entries.get(&key) {
            return Ok(*entry);
        }

        let entry = match fonts.outline(&key) {
            Some(glyph) => self.insert(&glyph)?,
            None => None,
        };
        self.entries.insert(key, entry);

        Ok(entry)
    }

    fn insert(&mut self, glyph: &OutlinedGlyph) -> Result<Option<CachedGlyph>, AtlasFull> {
        let bounds = glyph.px_bounds();
        let width = bounds.width() as u32;
        let height = bounds.height() as u32;

        if width == 0 || height == 0 {
            return Ok(None);
        }

        if width > GLYPH_ATLAS_SIZE {
            return Err(AtlasFull);
        }

        let (mut x, mut y, mut row_height) = (self.cursor_x, self.cursor_y, self.row_height);
        if x + width > GLYPH_ATLAS_SIZE {
            x = 0;
            y += row_height + Self::PADDING;
            row_height = 0;
        }

        if y + height > GLYPH_ATLAS_SIZE {
            return Err(AtlasFull);
        }

        let rect = AtlasRect {
            x,
            y,
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

        self.cursor_x = x + width + Self::PADDING;
        self.cursor_y = y;
        self.row_height = max(row_height, height);
        self.mark_dirty(rect.y..rect.y + rect.height);

        Ok(Some(CachedGlyph {
            rect,
            offset: [bounds.min.x as i32, bounds.min.y as i32],
        }))
    }

    pub fn clear(&mut self) {
        let used = self.cursor_y + self.row_height;
        self.pixels[..(used * GLYPH_ATLAS_SIZE * 4) as usize].fill(0);
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.row_height = 0;
        self.entries.clear();
        if used > 0 {
            self.mark_dirty(0..used);
        }
    }

    fn mark_dirty(&mut self, rows: Range<u32>) {
        self.dirty = Some(match self.dirty.take() {
            Some(dirty) => min(dirty.start, rows.start)..max(dirty.end, rows.end),
            None => rows,
        });
    }

    pub fn take_dirty(&mut self) -> Option<(u32, u32, &[u8])> {
        let rows = self.dirty.take()?;
        let row_bytes = (GLYPH_ATLAS_SIZE * 4) as usize;

        Some((
            rows.start,
            rows.len() as u32,
            &self.pixels[rows.start as usize * row_bytes..rows.end as usize * row_bytes],
        ))
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

impl Origin {
    fn uses_width(self) -> bool {
        match self {
            Origin::LeftUp | Origin::LeftDown => false,
            Origin::RightUp | Origin::RightDown | Origin::Center | Origin::CenterDown => true,
        }
    }
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

    fn bounds(cache: &mut GlyphCache, fonts: &Fonts, glyph: &PlacedGlyph) -> Option<[i32; 4]> {
        let cached = cache.get_or_insert(fonts, glyph.key).unwrap()?;
        let x = glyph.origin[0] + cached.offset[0];
        let y = glyph.origin[1] + cached.offset[1];
        Some([
            x,
            y,
            x + cached.rect.width as i32,
            y + cached.rect.height as i32,
        ])
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

        let mut cache = GlyphCache::new();
        let all: Vec<_> = glyphs
            .iter()
            .filter_map(|g| bounds(&mut cache, &fonts, g))
            .collect();
        let max_x = all.iter().map(|b| b[2]).max().unwrap();
        let max_y = all.iter().map(|b| b[3]).max().unwrap();

        assert!((1000..=1005).contains(&max_x), "max_x = {max_x}");
        assert!((735..=748).contains(&max_y), "max_y = {max_y}");
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

        let mut cache = GlyphCache::new();
        let rects: Vec<_> = glyphs
            .iter()
            .filter_map(|g| cache.get_or_insert(&fonts, g.key).unwrap())
            .map(|c| c.rect)
            .collect();

        assert_eq!(rects.len(), 5);
        for pair in rects.windows(2) {
            assert!(pair[0].x + pair[0].width < pair[1].x || pair[0].y != pair[1].y);
        }

        let (y, rows, pixels) = cache.take_dirty().unwrap();
        assert_eq!(y, 0);
        assert!(rows > 0);
        assert!(pixels.contains(&255));
    }
}
