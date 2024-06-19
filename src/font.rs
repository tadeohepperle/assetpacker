use std::collections::HashMap;

use glam::{vec2, Vec2};
use guillotiere::size2;
use image::GenericImage;
use sdfer::{Image2d, Unorm8};
use serde::{Deserialize, Serialize};

use crate::{gather::FontAsset, pack::next_pow2_number};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdfFont {
    font_size: usize,
    line_metrics: LineMetrics,
    name: String,
    glyphs: HashMap<char, Glyph>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct LineMetrics {
    /// The highest point that any glyph in the font extends to above the baseline. Typically
    /// positive.
    pub ascent: f32,
    /// The lowest point that any glyph in the font extends to below the baseline. Typically
    /// negative.
    pub descent: f32,
    /// The gap to leave between the descent of one line and the ascent of the next. This is of
    /// course only a guideline given by the font's designers.
    pub line_gap: f32,
    /// A precalculated value for the height or width of the line depending on if the font is laid
    /// out horizontally or vertically. It's calculated by: ascent - descent + line_gap.
    pub new_line_size: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Glyph {
    pub xmin: f32,
    pub ymin: f32,
    pub width: f32,
    pub height: f32,
    pub advance: f32,
    pub is_white_space: bool,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
}

pub fn font_to_sdf_font(font_asset: &FontAsset) -> (SdfFont, image::GrayImage) {
    let font_size: usize = 64;
    let pad: usize = 16;

    let font: fontdue::Font = fontdue::Font::from_bytes(&*font_asset.bytes, Default::default())
        .expect("data must be valid ttf");
    let mut glyphs: HashMap<char, Glyph> = HashMap::new();

    let atlas_size = next_pow2_number((font_size + 2 * pad) * 8); // this gives us space for at least 256 glyphs, which should be enough in most cases
    let mut atlas_allocator =
        guillotiere::AtlasAllocator::new(size2(atlas_size as i32, atlas_size as i32));
    let mut atlas_image = image::GrayImage::new(atlas_size as u32, atlas_size as u32);

    const ALPHABET: &str =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,!:;/?|(){}[]!+-_=* \n\t'\"><~`";
    for ch in ALPHABET.chars() {
        let (metrics, img) = font.rasterize(ch, font_size as f32);
        let glyph = if ch.is_whitespace() {
            Glyph {
                xmin: metrics.bounds.xmin,
                ymin: metrics.bounds.ymin,
                width: metrics.bounds.width,
                height: metrics.bounds.height,
                advance: metrics.advance_width,
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ZERO,
                is_white_space: true,
            }
        } else {
            let gray = image::GrayImage::from_raw(metrics.width as u32, metrics.height as u32, img)
                .unwrap();
            let mut gray_for_sdfer: Image2d<Unorm8> = From::from(gray.clone());

            let (generated_sdf, _) = sdfer::esdt::glyph_to_sdf(
                &mut gray_for_sdfer,
                sdfer::esdt::Params {
                    pad: pad as usize,
                    radius: pad as f32,
                    cutoff: 0.5,
                    solidify: true,
                    preprocess: true,
                },
                None,
            );
            let sdf = image::GrayImage::from(generated_sdf);
            let (w, h) = sdf.dimensions();
            let allocation = atlas_allocator
                .allocate(size2(w as i32, h as i32))
                .expect("allocation failed");
            let uv_min = vec2(
                allocation.rectangle.min.x as f32,
                allocation.rectangle.min.y as f32,
            ) / atlas_size as f32;
            let uv_max = vec2(
                allocation.rectangle.min.x as f32 + w as f32,
                allocation.rectangle.min.y as f32 + h as f32,
            ) / atlas_size as f32;

            atlas_image
                .copy_from(
                    &sdf,
                    allocation.rectangle.min.x as u32,
                    allocation.rectangle.min.y as u32,
                )
                .expect("copy from sdf_glyph image to atlas_image failed");

            Glyph {
                xmin: metrics.bounds.xmin - pad as f32,
                ymin: metrics.bounds.ymin - pad as f32,
                width: metrics.bounds.width + (2 * pad) as f32,
                height: metrics.bounds.height + (2 * pad) as f32,
                advance: metrics.advance_width,
                uv_min,
                uv_max,
                is_white_space: false,
            }
        };
        glyphs.insert(ch, glyph);
    }

    let lm = font.horizontal_line_metrics(font_size as f32).unwrap();
    let line_metrics = LineMetrics {
        ascent: lm.ascent,
        descent: lm.descent,
        line_gap: lm.line_gap,
        new_line_size: lm.new_line_size,
    };
    let sdf_font = SdfFont {
        font_size,
        line_metrics,
        name: font_asset.entry.asset_path.ident().to_string(),
        glyphs,
    };
    (sdf_font, atlas_image)
}
