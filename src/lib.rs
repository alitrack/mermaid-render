// mermaid-rs — Pure Rust Mermaid diagram renderer
// Extracted from MarkieCli (https://github.com/lsj5031/MarkieCli, MIT)
//
// Usage:
//   let diagram = mermaid_rs::parse_mermaid("graph TD\n  A-->B")?;
//   let svg = mermaid_rs::render_diagram(&diagram, &Default::default(), &mut mermaid_rs::EstimatedMeasure)?;

mod flowchart;
mod layout;
mod parser;
mod render;
mod types;
mod xml;

pub use parser::{parse_mermaid, MermaidDiagram};
pub use render::{render_diagram, DiagramStyle};

/// Simple rectangle for geometry calculations in diagram layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn with_padding(&self, padding: f32) -> Self {
        Self {
            x: self.x - padding,
            y: self.y - padding,
            w: self.w + padding * 2.0,
            h: self.h + padding * 2.0,
        }
    }

    pub fn expanded(&self, pad: f32) -> Self {
        self.with_padding(pad)
    }

    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }
}

/// Trait for text measurement. Implement this if you need accurate text sizing
/// (e.g. using a font shaping library). For basic usage, use [`EstimatedMeasure`].
pub trait TextMeasure {
    fn measure_text(
        &mut self,
        text: &str,
        font_size: f32,
        is_code: bool,
        is_bold: bool,
        is_italic: bool,
        max_width: Option<f32>,
    ) -> (f32, f32);
}

/// Rough text measurement based on character count.
/// Good enough for most diagram layouts; use a font-based implementation for
/// pixel-accurate rendering.
#[derive(Default)]
pub struct EstimatedMeasure;

impl TextMeasure for EstimatedMeasure {
    fn measure_text(
        &mut self,
        text: &str,
        font_size: f32,
        _is_code: bool,
        _is_bold: bool,
        _is_italic: bool,
        _max_width: Option<f32>,
    ) -> (f32, f32) {
        let char_width = font_size * 0.6;
        let width = text.len() as f32 * char_width;
        let height = font_size * 1.4;
        (width, height)
    }
}
