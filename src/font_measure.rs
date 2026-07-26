//! Font-based text measurement using `ab_glyph`.
//! Enabled via the `font-measure` feature flag.

use crate::TextMeasure;

#[cfg(feature = "font-measure")]
use ab_glyph::Font;

/// Accurate text measurement using a loaded font and `ab_glyph`.
#[cfg(feature = "font-measure")]
pub struct FontMeasure {
    font: ab_glyph::FontRef<'static>,
    /// System fonts loaded for fallback (CJK, symbols, etc.)
    fallback_fonts: Vec<ab_glyph::FontRef<'static>>,
    default_font_size: f32,
    font_data: Vec<Vec<u8>>,
}

#[cfg(feature = "font-measure")]
impl FontMeasure {
    /// Create a FontMeasure from in-memory font data.
    /// `font_data` takes ownership so the font bytes live as long as the measure.
    pub fn new(font_data: Vec<u8>) -> Result<Self, ab_glyph::InvalidFont> {
        // Safety: font_data is owned by Self and lives as long as the struct
        let font_ref: &[u8] = unsafe { std::mem::transmute(font_data.as_slice()) };
        let font = ab_glyph::FontRef::try_from_slice(font_ref)?;
        Ok(Self {
            font: unsafe { std::mem::transmute(font) },
            fallback_fonts: Vec::new(),
            default_font_size: 13.0,
            font_data: vec![font_data],
        })
    }

    /// Add a fallback font for characters not present in the primary font.
    pub fn with_fallback(mut self, font_data: Vec<u8>) -> Result<Self, ab_glyph::InvalidFont> {
        let font_ref: &[u8] = unsafe { std::mem::transmute(font_data.as_slice()) };
        let font = ab_glyph::FontRef::try_from_slice(font_ref)?;
        self.fallback_fonts.push(unsafe { std::mem::transmute(font) });
        self.font_data.push(font_data);
        Ok(self)
    }

    /// Set the default font size (used when `font_size` is 0).
    pub fn with_default_size(mut self, size: f32) -> Self {
        self.default_font_size = size;
        self
    }

    /// Find the best font for a given codepoint.
    fn font_for(&self, codepoint: u32) -> &ab_glyph::FontRef<'static> {
        if self.font.glyph_id(char::from_u32(codepoint).unwrap_or(' ')).0 != 0 {
            return &self.font;
        }
        for fallback in &self.fallback_fonts {
            if fallback.glyph_id(char::from_u32(codepoint).unwrap_or(' ')).0 != 0 {
                return fallback;
            }
        }
        &self.font // fallback to primary even if glyph missing
    }

    /// Try to find a system font file by family name.
    pub fn find_system_font() -> Option<Vec<u8>> {
        Self::find_font_in_dirs(&["/usr/share/fonts", "/System/Library/Fonts", "C:\\Windows\\Fonts"])
    }

    pub fn find_cjk_font() -> Option<Vec<u8>> {
        let cjk_candidates = [
            "NotoSansCJK", "NotoSansSC", "NotoSansJP", "WenQuanYi",
            "SimSun", "MS-Mincho", "AppleGothic", "PingFang",
        ];
        Self::find_font_in_dirs(&["/usr/share/fonts", "/System/Library/Fonts", "C:\\Windows\\Fonts"])
            .or_else(|| {
                // Try searching by known CJK font names
                for dir in &["/usr/share/fonts", "/System/Library/Fonts"] {
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            let name = entry.file_name().to_string_lossy().to_lowercase();
                            for cjk in &cjk_candidates {
                                if name.contains(&cjk.to_lowercase()) {
                                    if let Ok(data) = std::fs::read(entry.path()) {
                                        return Some(data);
                                    }
                                }
                            }
                        }
                    }
                }
                None
            })
    }

    fn find_font_in_dirs(dirs: &[&str]) -> Option<Vec<u8>> {
        let candidates = [
            "DejaVuSans.ttf", "LiberationSans-Regular.ttf",
            "Arial.ttf", "Helvetica.ttc",
            "NotoSans-Regular.ttf", "Roboto-Regular.ttf",
        ];
        for dir in dirs {
            for name in &candidates {
                let path = std::path::Path::new(dir).join(name);
                if let Ok(data) = std::fs::read(&path) {
                    return Some(data);
                }
            }
        }
        None
    }

    fn glyph_advance(&self, font: &ab_glyph::FontRef<'static>, codepoint: u32, scale: ab_glyph::PxScale) -> f32 {
        let ch = char::from_u32(codepoint).unwrap_or(' ');
        let glyph_id = font.glyph_id(ch);
        font.h_advance_unscaled(glyph_id) * scale.x / font.height_unscaled()
    }

    /// Measure a single character's advance width in pixels.
    pub fn char_width(&self, ch: char, font_size: f32) -> f32 {
        let scale = ab_glyph::PxScale::from(font_size);
        let font = self.font_for(ch as u32);
        self.glyph_advance(font, ch as u32, scale)
    }
}

#[cfg(feature = "font-measure")]
impl TextMeasure for FontMeasure {
    fn measure_text(
        &mut self,
        text: &str,
        font_size: f32,
        _is_code: bool,
        _is_bold: bool,
        _is_italic: bool,
        _max_width: Option<f32>,
    ) -> (f32, f32) {
        let fs = if font_size > 0.0 { font_size } else { self.default_font_size };
        let scale = ab_glyph::PxScale::from(fs);

        let mut total_width = 0.0f32;
        for ch in text.chars() {
            let font = self.font_for(ch as u32);
            total_width += self.glyph_advance(font, ch as u32, scale);
        }

        let height = fs * 1.2;
        (total_width.max(1.0), height.max(fs * 0.1))
    }
}
