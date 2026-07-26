use std::collections::{HashMap, HashSet};

use crate::TextMeasure;
use crate::Rect;

use super::layout::{LayoutEngine, LayoutPos};
use super::types::*;
use super::{MermaidDiagram, parse_mermaid};

/// Style configuration for diagram rendering
#[derive(Debug, Clone)]
pub struct DiagramStyle {
    pub node_fill: String,
    pub node_stroke: String,
    pub node_text: String,
    pub edge_stroke: String,
    pub edge_text: String,
    pub background: String,
    pub font_family: String,
    pub font_size: f32,
}

impl Default for DiagramStyle {
    fn default() -> Self {
        Self {
            node_fill: "#f5f5f5".to_string(),
            node_stroke: "#333333".to_string(),
            node_text: "#333333".to_string(),
            edge_stroke: "#333333".to_string(),
            edge_text: "#666666".to_string(),
            background: "transparent".to_string(),
            font_family: "sans-serif".to_string(),
            font_size: 13.0,
        }
    }
}

impl DiagramStyle {
    pub fn from_theme(text_color: &str, background: &str, code_bg: &str) -> Self {
        let diagram_fg = pick_higher_contrast(code_bg, text_color, background);
        let label_fg = mix_color(code_bg, &diagram_fg, 0.60);
        let edge_color = mix_color(code_bg, &diagram_fg, 0.30);
        let node_fill_color = mix_color(code_bg, &diagram_fg, 0.03);
        let node_stroke_color = mix_color(code_bg, &diagram_fg, 0.20);

        Self {
            node_fill: node_fill_color,
            node_stroke: node_stroke_color,
            node_text: diagram_fg,
            edge_stroke: edge_color,
            edge_text: label_fg,
            background: background.to_string(),
            font_family: "sans-serif".to_string(),
            font_size: 13.0,
        }
    }
}

fn parse_hex_rgb(value: &str) -> Option<(f32, f32, f32)> {
    let hex = value.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
    Some((r, g, b))
}

fn relative_luminance(color: (f32, f32, f32)) -> f32 {
    let linear = |v: f32| {
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };

    let (r, g, b) = color;
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

fn contrast_ratio(a: &str, b: &str) -> Option<f32> {
    let l1 = relative_luminance(parse_hex_rgb(a)?);
    let l2 = relative_luminance(parse_hex_rgb(b)?);
    let (hi, lo) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    Some((hi + 0.05) / (lo + 0.05))
}

/// Mix two hex colors: result = base * (1-t) + fg * t
fn mix_color(base: &str, fg: &str, t: f32) -> String {
    let (br, bg, bb) = parse_hex_rgb(base).unwrap_or((0.95, 0.95, 0.95));
    let (fr, fg_g, fb) = parse_hex_rgb(fg).unwrap_or((0.2, 0.2, 0.2));
    let r = (br * (1.0 - t) + fr * t).clamp(0.0, 1.0);
    let g = (bg * (1.0 - t) + fg_g * t).clamp(0.0, 1.0);
    let b = (bb * (1.0 - t) + fb * t).clamp(0.0, 1.0);
    format!(
        "#{:02x}{:02x}{:02x}",
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8
    )
}

fn pick_higher_contrast(base: &str, primary: &str, secondary: &str) -> String {
    let p = contrast_ratio(base, primary).unwrap_or(0.0);
    let s = contrast_ratio(base, secondary).unwrap_or(0.0);

    if s > p {
        secondary.to_string()
    } else {
        primary.to_string()
    }
}

/// Render any mermaid diagram to SVG
pub fn render_diagram<T: TextMeasure>(
    source: &str,
    style: &DiagramStyle,
    measure: &mut T,
) -> Result<(String, f32, f32), String> {
    let diagram = parse_mermaid(source)?;

    let result = match diagram {
        MermaidDiagram::Flowchart(fc) => {
            super::flowchart::render_flowchart(&fc, style, measure)?
        }
        MermaidDiagram::Sequence(seq) => render_sequence(&seq, style, measure)?,
        MermaidDiagram::ClassDiagram(cls) => render_class(&cls, style, measure)?,
        MermaidDiagram::StateDiagram(st) => render_state(&st, style, measure)?,
        MermaidDiagram::ErDiagram(er) => render_er(&er, style, measure)?,
        MermaidDiagram::Timeline(tl) => crate::timeline::render_timeline(&tl, style, measure)?,
        MermaidDiagram::Mindmap(mm) => crate::mindmap::render_mindmap(&mm, style, measure)?,
        MermaidDiagram::GitGraph(gg) => crate::gitgraph::render_gitgraph(&gg, style, style.font_size)?,
    };

    let source_lines = source.lines().filter(|l| !l.trim().is_empty()).count();
    if source_lines > 1 {
        let svg = &result.0;
        let has_content = svg.contains("<text")
            || svg.contains("<rect")
            || svg.contains("<circle")
            || svg.contains("<ellipse")
            || svg.contains("<path")
            || svg.contains("<polygon")
            || svg.contains("<line")
            || svg.contains("<polyline");
        if !has_content {
            eprintln!("Warning: Mermaid diagram produced no visual content; check syntax");
        }
    }

    Ok(result)
}

/// Escape XML special characters
pub fn escape_xml(s: &str) -> String {
    crate::xml::escape_xml(s)
}

// ============================================
// FLOWCHART RENDERING (moved to flowchart.rs)
// ============================================

// Note: render_flowchart is in flowchart.rs

// ============================================
// SEQUENCE DIAGRAM RENDERING
// ============================================

fn render_sequence(
    diagram: &SequenceDiagram,
    style: &DiagramStyle,
    measure: &mut impl TextMeasure,
) -> Result<(String, f32, f32), String> {
    if diagram.participants.is_empty() {
        return Ok(("<g></g>".to_string(), 100.0, 50.0));
    }

    let mut layout = LayoutEngine::new(measure, style.font_size);
    let (positions, bbox) = layout.layout_sequence(diagram);

    let mut svg = String::new();
    let padding = 20.0;

    let mut participant_centers: HashMap<&str, f32> = HashMap::new();
    for participant in &diagram.participants {
        if let Some(pos) = positions.get(&participant.id) {
            let (cx, _) = pos.center();
            participant_centers.insert(participant.id.as_str(), cx);
        }
    }

    let left_edge = participant_centers
        .values()
        .copied()
        .fold(f32::MAX, f32::min)
        .min(40.0);
    let right_edge = participant_centers
        .values()
        .copied()
        .fold(f32::MIN, f32::max)
        .max(120.0);

    for participant in &diagram.participants {
        if let Some(pos) = positions.get(&participant.id) {
            let display_name = participant.alias.as_ref().unwrap_or(&participant.id);
            let label = escape_xml(display_name);
            let text_x = participant_centers
                .get(participant.id.as_str())
                .copied()
                .unwrap_or(pos.x + pos.width / 2.0);

            svg.push_str(&format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="{}" stroke-width="1" rx="4" />"#,
                pos.x, pos.y, pos.width, pos.height, style.node_fill, style.node_stroke
            ));
            svg.push_str(&format!(
                r#"<text x="{:.2}" y="{:.2}" dy="0.35em" font-family="{}" font-size="{:.1}" font-weight="500" fill="{}" text-anchor="middle">{}</text>"#,
                text_x,
                pos.y + pos.height / 2.0,
                style.font_family,
                style.font_size,
                style.node_text,
                label
            ));
        }
    }

    let participant_bottom = diagram
        .participants
        .iter()
        .filter_map(|participant| positions.get(&participant.id))
        .map(|pos| pos.y + pos.height)
        .fold(bbox.y + 40.0, f32::max);
    let lifeline_start_y = participant_bottom + 8.0;

    let mut message_y = participant_bottom + 34.0;

    let mut activation_starts: HashMap<String, Vec<f32>> = HashMap::new();
    let elements_svg = render_sequence_elements(&mut RenderSequenceContext {
        elements: &diagram.elements,
        participant_centers: &participant_centers,
        style,
        measure,
        message_y: &mut message_y,
        block_depth: 0,
        left_edge,
        right_edge,
        activation_starts: &mut activation_starts,
    });

    let lifeline_end_y = (message_y + 6.0).max(lifeline_start_y + 24.0);
    for participant in &diagram.participants {
        if let Some(x) = participant_centers.get(participant.id.as_str()) {
            svg.push_str(&format!(
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75" stroke-dasharray="6,4" />"#,
                x, lifeline_start_y, x, lifeline_end_y, style.edge_stroke
            ));
        }
    }
    svg.push_str(&elements_svg);

    Ok((
        svg,
        bbox.right() + padding,
        bbox.bottom().max(message_y + 20.0) + padding,
    ))
}

struct RenderSequenceContext<'a, T: TextMeasure> {
    elements: &'a [SequenceElement],
    participant_centers: &'a HashMap<&'a str, f32>,
    style: &'a DiagramStyle,
    measure: &'a mut T,
    message_y: &'a mut f32,
    block_depth: usize,
    left_edge: f32,
    right_edge: f32,
    activation_starts: &'a mut HashMap<String, Vec<f32>>,
}

fn render_sequence_elements<T: TextMeasure>(
    ctx: &mut RenderSequenceContext<'_, T>,
) -> String {
    let elements = ctx.elements;
    let participant_centers = ctx.participant_centers;
    let style = ctx.style;
    let measure = &mut *ctx.measure;
    let message_y = &mut *ctx.message_y;
    let block_depth = ctx.block_depth;
    let left_edge = ctx.left_edge;
    let right_edge = ctx.right_edge;
    let activation_starts = &mut *ctx.activation_starts;

    let mut svg = String::new();
    for element in elements {
        match element {
            SequenceElement::Message(msg) => {
                if let (Some(x1), Some(x2)) = (
                    participant_centers.get(msg.from.as_str()),
                    participant_centers.get(msg.to.as_str()),
                ) {
                    if (x1 - x2).abs() < 0.5 {
                        // Self-message: draw a loop to the right
                        let cx = *x1;
                        let loop_w = 40.0;
                        let loop_h = 36.0;
                        let y_top = *message_y;
                        let y_bot = y_top + loop_h;

                        let dash = if msg.msg_type == MessageType::Dotted || msg.kind == MessageKind::Reply {
                            " stroke-dasharray=\"4,4\""
                        } else {
                            ""
                        };

                        svg.push_str(&format!(
                            r#"<polyline points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="none" stroke="{}" stroke-width="0.75"{} />"#,
                            cx, y_top,
                            cx + loop_w, y_top,
                            cx + loop_w, y_bot,
                            cx, y_bot,
                            style.edge_stroke, dash
                        ));

                        // Arrowhead pointing left at return point
                        svg.push_str(&format!(
                            r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" />"#,
                            cx, y_bot,
                            cx + 7.0, y_bot - 3.5,
                            cx + 7.0, y_bot + 3.5,
                            style.edge_stroke
                        ));

                        // Label with background pill for readability
                        if !msg.label.is_empty() {
                            let cleaned = crate::xml::sanitize_xml_text(&msg.label);
                            let label_font = style.font_size * 0.82;
                            let text_w = measure
                                .measure_text(&cleaned, label_font, false, false, false, None)
                                .0;
                            let pill_pad = 4.0;
                            let pill_w = text_w + pill_pad * 2.0;
                            let pill_h = label_font + pill_pad * 2.0;
                            let lx = cx + loop_w + 4.0 + text_w / 2.0;
                            let ly = y_top + loop_h / 2.0;
                            svg.push_str(&format!(
                                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="3" fill="{}" />"#,
                                lx - pill_w / 2.0,
                                ly - pill_h / 2.0,
                                pill_w,
                                pill_h,
                                style.node_fill
                            ));
                            svg.push_str(&format!(
                                r#"<text x="{:.2}" y="{:.2}" dy="0.35em" font-family="{}" font-size="{:.1}" fill="{}">{}</text>"#,
                                lx,
                                ly,
                                style.font_family,
                                label_font,
                                style.edge_text,
                                escape_xml(&cleaned)
                            ));
                        }

                        *message_y = y_bot + 20.0;
                    } else {
                        let is_right = x2 > x1;
                        let dash =
                            if msg.msg_type == MessageType::Dotted || msg.kind == MessageKind::Reply {
                                " stroke-dasharray=\"4,4\""
                            } else {
                                ""
                            };

                        svg.push_str(&format!(
                            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75"{} />"#,
                            x1, *message_y, x2, *message_y, style.edge_stroke, dash
                        ));

                        let arrow_dir = if is_right { -1.0 } else { 1.0 };
                        let arrow_x = *x2;
                        match msg.kind {
                            MessageKind::Async => {
                                let p1 = (arrow_x + arrow_dir * 7.0, *message_y - 3.5);
                                let p2 = (arrow_x + arrow_dir * 7.0, *message_y + 3.5);
                                svg.push_str(&format!(
                                    r#"<polyline points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="none" stroke="{}" stroke-width="0.75" />"#,
                                    p1.0, p1.1, arrow_x, *message_y, p2.0, p2.1, style.edge_stroke
                                ));
                            }
                            MessageKind::Sync => {
                                svg.push_str(&format!(
                                    r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" />"#,
                                    arrow_x,
                                    *message_y,
                                    arrow_x + arrow_dir * 7.0,
                                    *message_y - 3.5,
                                    arrow_x + arrow_dir * 7.0,
                                    *message_y + 3.5,
                                    style.edge_stroke
                                ));
                            }
                            MessageKind::Reply => {
                                let p1 = (arrow_x + arrow_dir * 7.0, *message_y - 3.5);
                                let p2 = (arrow_x + arrow_dir * 7.0, *message_y + 3.5);
                                svg.push_str(&format!(
                                    r#"<polyline points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="none" stroke="{}" stroke-width="0.75" />"#,
                                    p1.0, p1.1, arrow_x, *message_y, p2.0, p2.1, style.edge_stroke
                                ));
                            }
                        }

                        if !msg.label.is_empty() {
                            let cleaned = crate::xml::sanitize_xml_text(&msg.label);
                            let label_font = style.font_size * 0.82;
                            let text_w = measure
                                .measure_text(&cleaned, label_font, false, false, false, None)
                                .0;
                            let pill_pad = 6.0;
                            let pill_w = text_w + pill_pad * 2.0;
                            let pill_h = label_font + pill_pad * 2.0;
                            let label_x = (x1 + x2) / 2.0;
                            let label_y = *message_y - 10.0;

                            svg.push_str(&format!(
                                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="4" fill="{}" stroke="{}" stroke-width="0.5" />"#,
                                label_x - pill_w / 2.0,
                                label_y - pill_h / 2.0,
                                pill_w,
                                pill_h,
                                style.node_fill,
                                style.node_stroke
                            ));
                            svg.push_str(&format!(
                                r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="middle">{}</text>"#,
                                label_x,
                                label_y + 0.35,
                                style.font_family,
                                label_font,
                                style.edge_text,
                                escape_xml(&cleaned)
                            ));
                        }

                        *message_y += 50.0;
                    }
                }
            }
            SequenceElement::Activation(activation) => {
                if let Some(cx) = participant_centers.get(activation.participant.as_str()) {
                    activation_starts
                        .entry(activation.participant.clone())
                        .or_default()
                        .push(*message_y - 10.0);
                    svg.push_str(&format!(
                        r#"<rect x="{:.2}" y="{:.2}" width="8" height="16" fill="{}" stroke="{}" stroke-width="1" />"#,
                        cx - 4.0,
                        *message_y - 10.0,
                        style.node_fill,
                        style.node_stroke
                    ));
                }
                *message_y += 24.0;
            }
            SequenceElement::Deactivation(activation) => {
                if let Some(cx) = participant_centers.get(activation.participant.as_str())
                    && let Some(start) = activation_starts
                        .entry(activation.participant.clone())
                        .or_default()
                        .pop()
                    {
                        svg.push_str(&format!(
                            r#"<rect x="{:.2}" y="{:.2}" width="8" height="{:.2}" fill="{}" fill-opacity="0.35" stroke="{}" stroke-width="1" />"#,
                            cx - 4.0,
                            start,
                            (*message_y - start).max(16.0),
                            style.node_fill,
                            style.node_stroke
                        ));
                    }
                *message_y += 24.0;
            }
            SequenceElement::Note {
                participant,
                position,
                text,
            } => {
                if let Some(cx) = participant_centers.get(participant.as_str()) {
                    let cleaned = crate::xml::sanitize_xml_text(text);
                    let note_width = (measure
                        .measure_text(&cleaned, style.font_size * 0.8, false, false, false, None)
                        .0
                        + 20.0)
                        .clamp(80.0, 220.0);
                    let x = match position.as_str() {
                        "left" => cx - note_width - 12.0,
                        "right" => cx + 12.0,
                        _ => cx - note_width / 2.0,
                    };
                    let y = *message_y - 18.0;
                    svg.push_str(&format!(
                        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="28" rx="3" fill="{}" fill-opacity="0.25" stroke="{}" stroke-width="1" />"#,
                        x,
                        y,
                        note_width,
                        style.node_fill,
                        style.node_stroke
                    ));
                    svg.push_str(&format!(
                        r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}">{}</text>"#,
                        x + 8.0,
                        y + 18.0,
                        style.font_family,
                        style.font_size * 0.8,
                        style.node_text,
                        escape_xml(text)
                    ));
                }
                *message_y += 42.0;
            }
            SequenceElement::Block(block) => {
                *message_y += 12.0;
                let start_y = *message_y - 20.0;
                let inset = block_depth as f32 * 8.0;
                let block_left = left_edge - 36.0 + inset;
                let block_right = right_edge + 36.0 - inset;
                let block_kind = match block.block_type {
                    SequenceBlockType::Alt => "alt",
                    SequenceBlockType::Opt => "opt",
                    SequenceBlockType::Loop => "loop",
                    SequenceBlockType::Par => "par",
                    SequenceBlockType::Critical => "critical",
                };
                let title = if block.label.is_empty() {
                    block_kind.to_string()
                } else {
                    format!("{} {}", block_kind, block.label)
                };

                let title_font = style.font_size * 0.8;
                let cleaned_title = crate::xml::sanitize_xml_text(&title);
                let title_w = measure
                    .measure_text(&cleaned_title, title_font, false, true, false, None)
                    .0;
                let np_pad_x = 6.0;
                let np_pad_y = 3.0;
                let np_w = title_w + np_pad_x * 2.0;
                let np_h = title_font + np_pad_y * 2.0;
                let np_x = block_left + 10.0 - np_pad_x;
                let np_y = *message_y - title_font - np_pad_y + 2.0;
                svg.push_str(&format!(
                    r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="{}" stroke-width="0.75" />"#,
                    np_x, np_y, np_w, np_h,
                    style.node_fill, style.node_stroke
                ));
                svg.push_str(&format!(
                    r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" font-weight="bold">{}</text>"#,
                    block_left + 10.0,
                    *message_y,
                    style.font_family,
                    title_font,
                    style.node_text,
                    escape_xml(&title)
                ));
                *message_y += 22.0;

                svg.push_str(&render_sequence_elements(&mut RenderSequenceContext {
                    elements: &block.messages,
                    participant_centers,
                    style,
                    measure,
                    message_y,
                    block_depth: block_depth + 1,
                    left_edge,
                    right_edge,
                    activation_starts,
                }));

                for (label, branch_elements) in &block.else_branches {
                    let separator_y = *message_y + 2.0;
                    svg.push_str(&format!(
                        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1" stroke-dasharray="5,3" />"#,
                        block_left,
                        separator_y,
                        block_right,
                        separator_y,
                        style.edge_stroke
                    ));
                    if !label.is_empty() {
                        svg.push_str(&format!(
                            r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}">{}</text>"#,
                            block_left + 10.0,
                            separator_y - 12.0,
                            style.font_family,
                            style.font_size * 0.78,
                            style.node_text,
                            escape_xml(label)
                        ));
                    }
                    *message_y = separator_y + 30.0;
                    svg.push_str(&render_sequence_elements(&mut RenderSequenceContext {
                        elements: branch_elements,
                        participant_centers,
                        style,
                        measure,
                        message_y,
                        block_depth: block_depth + 1,
                        left_edge,
                        right_edge,
                        activation_starts,
                    }));
                }

                svg.push_str(&format!(
                    r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="none" stroke="{}" stroke-width="1" stroke-dasharray="5,3" />"#,
                    block_left,
                    start_y,
                    (block_right - block_left).max(24.0),
                    (*message_y - start_y + 16.0).max(28.0),
                    style.edge_stroke
                ));
                *message_y += 10.0;
            }
        }
    }
    svg
}

// ============================================
// CLASS DIAGRAM RENDERING
// ============================================

fn render_class(
    diagram: &ClassDiagram,
    style: &DiagramStyle,
    measure: &mut impl TextMeasure,
) -> Result<(String, f32, f32), String> {
    if diagram.classes.is_empty() {
        return Ok(("<g></g>".to_string(), 100.0, 50.0));
    }

    let mut layout = LayoutEngine::new(measure, style.font_size);
    let (positions, _edge_waypoints, bbox) = layout.layout_class(diagram);

    let mut svg = String::new();
    let padding = 20.0;

    // Draw classes
    for class in &diagram.classes {
        if let Some(pos) = positions.get(&class.name) {
            svg.push_str(&render_class_box(class, pos, style));
        }
    }

    // Draw relations
    for relation in &diagram.relations {
        let from_pos = positions.get(&relation.from);
        let to_pos = positions.get(&relation.to);

        if let (Some(from), Some(to)) = (from_pos, to_pos) {
            svg.push_str(&render_class_relation(relation, from, to, style, measure));
        }
    }

    let total_width = bbox.right() + padding;
    let total_height = bbox.bottom() + padding;

    Ok((svg, total_width, total_height))
}

fn render_class_box(class: &ClassDefinition, pos: &LayoutPos, style: &DiagramStyle) -> String {
    let mut svg = String::new();

    // Main box
    svg.push_str(&format!(
        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="{}" stroke-width="1" />"#,
        pos.x, pos.y, pos.width, pos.height,
        style.node_fill, style.node_stroke
    ));

    let mut y = pos.y + style.font_size + 8.0;

    // Class name
    let effective_stereotype = if class.is_interface {
        class
            .stereotype
            .clone()
            .or_else(|| Some("interface".to_string()))
    } else {
        class.stereotype.clone()
    };
    let name_text = if let Some(stereo) = effective_stereotype {
        format!(
            "&lt;&lt;{}&gt;&gt; {}",
            escape_xml(&stereo),
            escape_xml(&class.name)
        )
    } else {
        escape_xml(&class.name)
    };
    let name_style = if class.is_abstract || class.is_interface {
        " font-style=\"italic\""
    } else {
        ""
    };

    // Header band (subtle tinted background for class name area)
    let header_h = y + 6.0 - pos.y;
    let header_fill = mix_color(&style.node_fill, &style.node_text, 0.05);
    svg.push_str(&format!(
        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" />"#,
        pos.x + 0.5, pos.y + 0.5, pos.width - 1.0, header_h,
        header_fill
    ));

    svg.push_str(&format!(
        r#"<text x="{:.2}" y="{:.2}" dy="0.35em" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="middle" font-weight="bold"{}>{}</text>"#,
        pos.x + pos.width / 2.0,
        pos.y + header_h / 2.0,
        style.font_family,
        style.font_size,
        style.node_text,
        name_style,
        name_text
    ));

    // Divider line after name
    y += 6.0;
    svg.push_str(&format!(
        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75" />"#,
        pos.x,
        y,
        pos.x + pos.width,
        y,
        style.node_stroke
    ));

    // Attributes
    y += style.font_size + 4.0;
    for attr in &class.attributes {
        let vis = match attr.member.visibility {
            Visibility::Public => "+",
            Visibility::Private => "-",
            Visibility::Protected => "#",
            Visibility::Package => "~",
        };
        let attr_text = if let Some(ref t) = attr.type_annotation {
            format!("{} {}: {}", vis, attr.member.name, t)
        } else {
            format!("{} {}", vis, attr.member.name)
        };

        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" font-family="monospace" font-size="{:.1}" fill="{}"{}{}>{}</text>"#,
            pos.x + 8.0,
            y,
            style.font_size * 0.85,
            style.node_text,
            if attr.member.is_static {
                " text-decoration=\"underline\""
            } else {
                ""
            },
            if attr.member.is_abstract {
                " font-style=\"italic\""
            } else {
                ""
            },
            escape_xml(&attr_text)
        ));
        y += style.font_size * 0.9;
    }

    // Divider line before methods
    if !class.methods.is_empty() {
        y += 2.0;
        svg.push_str(&format!(
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1" />"#,
            pos.x,
            y,
            pos.x + pos.width,
            y,
            style.node_stroke
        ));
        y += style.font_size + 2.0;
    }

    // Methods
    for method in &class.methods {
        let vis = match method.member.visibility {
            Visibility::Public => "+",
            Visibility::Private => "-",
            Visibility::Protected => "#",
            Visibility::Package => "~",
        };

        let params: Vec<String> = method
            .parameters
            .iter()
            .map(|(name, t)| {
                if let Some(ty) = t {
                    format!("{}: {}", name, ty)
                } else {
                    name.clone()
                }
            })
            .collect();

        let method_text = if let Some(ref ret) = method.return_type {
            format!(
                "{} {}({}): {}",
                vis,
                method.member.name,
                params.join(", "),
                ret
            )
        } else {
            format!("{} {}({})", vis, method.member.name, params.join(", "))
        };

        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" font-family="monospace" font-size="{:.1}" fill="{}"{}{}>{}</text>"#,
            pos.x + 8.0,
            y,
            style.font_size * 0.85,
            style.node_text,
            if method.member.is_static {
                " text-decoration=\"underline\""
            } else {
                ""
            },
            if method.member.is_abstract {
                " font-style=\"italic\""
            } else {
                ""
            },
            escape_xml(&method_text)
        ));
        y += style.font_size * 0.9;
    }

    svg
}

fn render_class_relation(
    relation: &ClassRelation,
    from: &LayoutPos,
    to: &LayoutPos,
    style: &DiagramStyle,
    measure: &mut impl TextMeasure,
) -> String {
    let mut svg = String::new();

    let (from_cx, from_cy) = from.center();
    let (to_cx, to_cy) = to.center();
    let angle = (to_cy - from_cy).atan2(to_cx - from_cx);

    let (x1, y1) = rect_boundary_point(from, angle);
    let (x2, y2) = rect_boundary_point(to, angle + std::f32::consts::PI);

    let line_style = match relation.relation_type {
        ClassRelationType::Dependency | ClassRelationType::Realization => {
            " stroke-dasharray=\"6,3\""
        }
        _ => "",
    };

    svg.push_str(&format!(
        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75"{} />"#,
        x1, y1, x2, y2, style.edge_stroke, line_style
    ));

    let (from_marker, to_marker) = match relation.relation_type {
        ClassRelationType::Inheritance => (Some("hollow_triangle"), None),
        ClassRelationType::Composition => (Some("filled_diamond"), None),
        ClassRelationType::Aggregation => (Some("hollow_diamond"), None),
        ClassRelationType::Association => (None, Some("arrow")),
        ClassRelationType::Dependency => (Some("arrow"), None),
        ClassRelationType::Realization => (Some("hollow_triangle"), None),
    };

    if let Some(marker) = to_marker {
        let angle = (y2 - y1).atan2(x2 - x1);
        svg.push_str(&draw_marker(marker, x2, y2, angle, style));
    }

    if let Some(marker) = from_marker {
        let angle = (y1 - y2).atan2(x1 - x2);
        svg.push_str(&draw_marker(marker, x1, y1, angle, style));
    }

    let angle = (y2 - y1).atan2(x2 - x1);
    let unit_x = angle.cos();
    let unit_y = angle.sin();
    let normal_x = -unit_y;
    let normal_y = unit_x;

    if let Some(label) = &relation.label {
        let mx = (x1 + x2) / 2.0;
        let my = (y1 + y2) / 2.0;
        let cleaned = crate::xml::sanitize_xml_text(label);
        let label_font = style.font_size * 0.8;
        let label_offset = 18.0;
        let label_x = mx + normal_x * label_offset;
        let label_y = my + normal_y * label_offset;
        let text_w = measure
            .measure_text(&cleaned, label_font, false, false, false, None)
            .0;
        let pill_pad = 5.0;
        let pill_w = text_w + pill_pad * 2.0;
        let pill_h = label_font + pill_pad * 1.6;

        svg.push_str(&format!(
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="4" fill="{}" stroke="{}" stroke-width="0.5" />"#,
            label_x - pill_w / 2.0,
            label_y - pill_h / 2.0,
            pill_w,
            pill_h,
            style.node_fill,
            style.node_stroke
        ));

        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="middle">{}</text>"#,
            label_x,
            label_y + 0.35,
            style.font_family,
            label_font,
            style.edge_text,
            escape_xml(&cleaned)
        ));
    }

    if let Some(m) = &relation.multiplicity_from {
        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}">{}</text>"#,
            x1 + unit_x * 12.0 + normal_x * 8.0,
            y1 + unit_y * 12.0 + normal_y * 8.0,
            style.font_family,
            style.font_size * 0.75,
            style.edge_text,
            escape_xml(m)
        ));
    }

    if let Some(m) = &relation.multiplicity_to {
        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="end">{}</text>"#,
            x2 - unit_x * 12.0 + normal_x * 8.0,
            y2 - unit_y * 12.0 + normal_y * 8.0,
            style.font_family,
            style.font_size * 0.75,
            style.edge_text,
            escape_xml(m)
        ));
    }

    svg
}

fn draw_marker(marker_type: &str, x: f32, y: f32, angle: f32, style: &DiagramStyle) -> String {
    let cos = angle.cos();
    let sin = angle.sin();

    match marker_type {
        "arrow" => {
            let p1 = (x - cos * 12.0 + sin * 5.0, y - sin * 12.0 - cos * 5.0);
            let p2 = (x - cos * 12.0 - sin * 5.0, y - sin * 12.0 + cos * 5.0);
            format!(
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" />"#,
                x, y, p1.0, p1.1, p2.0, p2.1, style.edge_stroke
            )
        }
        "hollow_triangle" => {
            let p1 = (x - cos * 14.0 + sin * 7.0, y - sin * 14.0 - cos * 7.0);
            let p2 = (x - cos * 14.0 - sin * 7.0, y - sin * 14.0 + cos * 7.0);
            format!(
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" stroke="{}" stroke-width="1" />"#,
                x, y, p1.0, p1.1, p2.0, p2.1, style.node_fill, style.edge_stroke
            )
        }
        "filled_diamond" => {
            let p1 = (x - cos * 16.0 + sin * 6.0, y - sin * 16.0 - cos * 6.0);
            let p2 = (x - cos * 16.0 - sin * 6.0, y - sin * 16.0 + cos * 6.0);
            let back = (x - cos * 24.0, y - sin * 24.0);
            format!(
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" />"#,
                x, y, p1.0, p1.1, back.0, back.1, p2.0, p2.1, style.edge_stroke
            )
        }
        "hollow_diamond" => {
            let p1 = (x - cos * 16.0 + sin * 6.0, y - sin * 16.0 - cos * 6.0);
            let p2 = (x - cos * 16.0 - sin * 6.0, y - sin * 16.0 + cos * 6.0);
            let back = (x - cos * 24.0, y - sin * 24.0);
            format!(
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" stroke="{}" stroke-width="1" />"#,
                x, y, p1.0, p1.1, back.0, back.1, p2.0, p2.1, style.node_fill, style.edge_stroke
            )
        }
        _ => String::new(),
    }
}

// ============================================
// STATE DIAGRAM RENDERING
// ============================================

fn rect_from_pos(pos: &LayoutPos, pad: f32) -> Rect {
    Rect {
        x: pos.x - pad,
        y: pos.y - pad,
        w: pos.width + 2.0 * pad,
        h: pos.height + 2.0 * pad,
    }
}

fn vseg_hits_rect(x: f32, y1: f32, y2: f32, r: &Rect) -> bool {
    let (ya, yb) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
    x >= r.x && x <= r.x + r.w && yb >= r.y && ya <= r.y + r.h
}

fn hseg_hits_rect(y: f32, x1: f32, x2: f32, r: &Rect) -> bool {
    let (xa, xb) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    y >= r.y && y <= r.y + r.h && xb >= r.x && xa <= r.x + r.w
}

fn line_intersects_rect(x1: f32, y1: f32, x2: f32, y2: f32, r: &Rect) -> bool {
    // Cohen–Sutherland: check if segment (x1,y1)→(x2,y2) intersects axis-aligned rect r
    let left = r.x;
    let right = r.x + r.w;
    let top = r.y;
    let bottom = r.y + r.h;

    let code = |x: f32, y: f32| -> u8 {
        let mut c = 0u8;
        if x < left { c |= 1; }
        if x > right { c |= 2; }
        if y < top { c |= 4; }
        if y > bottom { c |= 8; }
        c
    };

    let mut c1 = code(x1, y1);
    let mut c2 = code(x2, y2);
    let mut ax = x1;
    let mut ay = y1;
    let mut bx = x2;
    let mut by = y2;

    for _ in 0..20 {
        if c1 == 0 || c2 == 0 { return true; } // one endpoint inside
        if c1 & c2 != 0 { return false; } // both on same outside
        let c = if c1 != 0 { c1 } else { c2 };
        let (nx, ny);
        if c & 8 != 0 { nx = ax + (bx - ax) * (bottom - ay) / (by - ay); ny = bottom; }
        else if c & 4 != 0 { nx = ax + (bx - ax) * (top - ay) / (by - ay); ny = top; }
        else if c & 2 != 0 { ny = ay + (by - ay) * (right - ax) / (bx - ax); nx = right; }
        else { ny = ay + (by - ay) * (left - ax) / (bx - ax); nx = left; }
        if c == c1 { ax = nx; ay = ny; c1 = code(ax, ay); }
        else { bx = nx; by = ny; c2 = code(bx, by); }
    }
    false
}

fn render_state(
    diagram: &StateDiagram,
    style: &DiagramStyle,
    measure: &mut impl TextMeasure,
) -> Result<(String, f32, f32), String> {
    if diagram.states.is_empty() {
        return Ok(("<g></g>".to_string(), 100.0, 50.0));
    }

    let mut layout = LayoutEngine::new(measure, style.font_size);
    let (positions, _edge_waypoints, bbox) = layout.layout_state(diagram);

    let mut svg = String::new();
    let padding = 20.0;

    let mut child_state_ids: HashSet<&str> = HashSet::new();
    for state in &diagram.states {
        for child in &state.children {
            if let StateElement::State(child_state) = child {
                child_state_ids.insert(child_state.id.as_str());
            }
        }
    }

    // Draw transitions first (behind states)
    let visible_transitions: Vec<&StateTransition> = diagram
        .transitions
        .iter()
        .filter(|transition| {
            !child_state_ids.contains(transition.from.as_str())
                && !child_state_ids.contains(transition.to.as_str())
        })
        .collect();

    let mut pair_totals: HashMap<(String, String), usize> = HashMap::new();
    for transition in &visible_transitions {
        *pair_totals
            .entry(state_pair_key(&transition.from, &transition.to))
            .or_insert(0) += 1;
    }

    let state_obstacles: Vec<Rect> = positions
        .values()
        .map(|p| rect_from_pos(p, 8.0))
        .collect();

    let mut transition_min_x = f32::MAX;
    let mut transition_max_x = f32::MIN;
    let mut transition_max_y = f32::MIN;

    let mut pair_seen: HashMap<(String, String), usize> = HashMap::new();
    let mut occupied_labels: Vec<Rect> = Vec::new();
    for transition in visible_transitions {
        let key = state_pair_key(&transition.from, &transition.to);
        let route_index = pair_seen.get(&key).copied().unwrap_or(0);
        pair_seen.insert(key.clone(), route_index + 1);
        let route_total = pair_totals.get(&key).copied().unwrap_or(1);

        let from_pos = positions.get(&transition.from);
        let to_pos = positions.get(&transition.to);

        if let (Some(from), Some(to)) = (from_pos, to_pos) {
            let (t_svg, ext) = render_state_transition(&mut StateTransitionContext {
                transition,
                from,
                to,
                style,
                measure,
                route_index,
                route_total,
                occupied_labels: &mut occupied_labels,
                obstacles: &state_obstacles,
            });
            svg.push_str(&t_svg);
            transition_min_x = transition_min_x.min(ext.x);
            transition_max_x = transition_max_x.max(ext.x + ext.w);
            transition_max_y = transition_max_y.max(ext.y + ext.h);
        }
    }

    // Draw states
    for state in &diagram.states {
        if child_state_ids.contains(state.id.as_str()) {
            continue;
        }

        if let Some(pos) = positions.get(&state.id) {
            svg.push_str(&render_state_node(
                state,
                &state.children,
                pos,
                style,
                &positions,
                measure,
            ));
        }
    }

    let mut total_width = bbox.right() + padding;
    let mut total_height = bbox.bottom() + padding;

    for state in &diagram.states {
        for child in &state.children {
            if let StateElement::Note { state: note_state, text } = child
                && !text.is_empty()
                    && let Some(pos) = positions.get(note_state.as_str()) {
                        let note_width = 180.0_f32;
                        let note_height = 26.0_f32;
                        let nx = pos.x + pos.width + 28.0;
                        let ny = pos.y + 4.0;
                        total_width = total_width.max(nx + note_width + padding);
                        total_height = total_height.max(ny + note_height + padding);
                    }
        }
    }

    // Expand to contain any transition routing that extends beyond the bbox
    if transition_max_x != f32::MIN {
        total_width = total_width.max(transition_max_x + padding);
    }
    if transition_max_y != f32::MIN {
        total_height = total_height.max(transition_max_y + padding);
    }

    // If transitions route into negative x territory, shift everything right
    if transition_min_x != f32::MAX && transition_min_x < 0.0 {
        let shift = -transition_min_x + padding;
        let shifted_svg = format!(r#"<g transform="translate({:.2},0)">{}</g>"#, shift, svg);
        total_width += shift;
        return Ok((shifted_svg, total_width, total_height));
    }

    Ok((svg, total_width, total_height))
}

fn render_state_node(
    state: &State,
    children: &[StateElement],
    pos: &LayoutPos,
    style: &DiagramStyle,
    positions: &HashMap<String, LayoutPos>,
    measure: &mut impl TextMeasure,
) -> String {
    let mut svg = String::new();

    if state.is_start {
        // Start state (filled circle)
        svg.push_str(&format!(
            r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}" />"#,
            pos.x + pos.width / 2.0,
            pos.y + pos.height / 2.0,
            pos.width / 2.0,
            style.node_stroke
        ));
    } else if state.is_end {
        // End state (circle with ring)
        let cx = pos.x + pos.width / 2.0;
        let cy = pos.y + pos.height / 2.0;
        svg.push_str(&format!(
            r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}" stroke="{}" stroke-width="2" />"#,
            cx,
            cy,
            pos.width / 2.0 - 3.0,
            style.node_stroke,
            style.node_stroke
        ));
        svg.push_str(&format!(
            r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="none" stroke="{}" stroke-width="2" />"#,
            cx, cy, pos.width / 2.0, style.node_stroke
        ));
    } else {
        svg.push_str(&format!(
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" fill="{}" stroke="{}" stroke-width="1" />"#,
            pos.x, pos.y, pos.width, pos.height,
            10.0, style.node_fill, style.node_stroke
        ));

        let text_x = pos.x + pos.width / 2.0;
        let text_y = if state.is_composite {
            pos.y + style.font_size + 8.0
        } else {
            pos.y + pos.height / 2.0 + style.font_size / 3.0
        };
        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="middle">{}</text>"#,
            text_x, text_y, style.font_family, style.font_size, style.node_text, escape_xml(&state.label)
        ));

        if state.is_composite {
            svg.push_str(&render_composite_state_contents(
                state, children, pos, style, positions, measure,
            ));
        } else {
            for child in children {
                if let StateElement::Note {
                    state: note_state,
                    text,
                } = child
                    && note_state == &state.id && !text.is_empty() {
                        svg.push_str(&render_state_note(note_state, text, pos, style, measure));
                    }
            }
        }
    }

    svg
}

fn calculate_child_state_size(
    state: &State,
    style: &DiagramStyle,
    measure: &mut impl TextMeasure,
) -> (f32, f32) {
    if state.is_start || state.is_end {
        return (24.0, 24.0);
    }

    let node_padding = 12.0;
    let label_w = measure
        .measure_text(&state.label, style.font_size, false, false, false, None)
        .0;
    let base_width = (label_w + node_padding * 2.0).max(120.0);
    let base_height = (style.font_size * 2.2).max(40.0);

    if !state.is_composite {
        return (base_width, base_height);
    }

    let child_states: Vec<&State> = state
        .children
        .iter()
        .filter_map(|child| match child {
            StateElement::State(s) if s.id != state.id => Some(s),
            _ => None,
        })
        .collect();

    if child_states.is_empty() {
        return (base_width, base_height);
    }

    let child_sizes: Vec<(f32, f32)> = child_states
        .iter()
        .map(|s| calculate_child_state_size(s, style, measure))
        .collect();

    let child_gap = 20.0;
    let inner_pad = 16.0;
    let header_h = style.font_size * 2.0 + 16.0;

    let max_child_w: f32 = child_sizes.iter().map(|(w, _)| *w).fold(0.0, f32::max);
    let total_child_h: f32 = child_sizes.iter().map(|(_, h)| *h).sum::<f32>()
        + child_gap * (child_sizes.len().saturating_sub(1)) as f32;

    let width = base_width.max(max_child_w + inner_pad * 2.0);
    let height = header_h + total_child_h + inner_pad * 2.0;

    (width, height)
}

fn render_composite_state_contents(
    state: &State,
    children: &[StateElement],
    parent_pos: &LayoutPos,
    style: &DiagramStyle,
    positions: &HashMap<String, LayoutPos>,
    measure: &mut impl TextMeasure,
) -> String {
    let mut svg = String::new();
    let mut child_states: Vec<&State> = Vec::new();
    let mut child_transitions: Vec<&StateTransition> = Vec::new();
    let mut child_notes: Vec<(&str, &str)> = Vec::new();

    for child in children {
        match child {
            StateElement::State(child_state) => child_states.push(child_state),
            StateElement::Transition(transition) => child_transitions.push(transition),
            StateElement::Note { state, text } if !text.is_empty() => {
                child_notes.push((state.as_str(), text.as_str()))
            }
            StateElement::Note { .. } => {}
        }
    }

    if child_states.is_empty() {
        return svg;
    }

    let child_state_nodes: Vec<&State> = child_states
        .into_iter()
        .filter(|child_state| child_state.id != state.id)
        .collect();

    if child_state_nodes.is_empty() {
        return svg;
    }

    let mut child_positions: HashMap<String, LayoutPos> = HashMap::new();
    let header_h = style.font_size * 2.0 + 16.0;
    let inner_pad = 16.0;
    let route_lane = 40.0; // extra space on each side for transition routing
    let child_gap = 20.0;
    let inner_top = parent_pos.y + header_h + inner_pad;
    let content_left = parent_pos.x + inner_pad + route_lane;
    let content_width = parent_pos.width - inner_pad * 2.0 - route_lane * 2.0;

    let mut y_cursor = inner_top;
    for child_state in &child_state_nodes {
        let (child_w, child_h) = calculate_child_state_size(child_state, style, measure);
        let child_width = content_width.max(child_w);
        let child_x = content_left;
        let child_y = y_cursor;

        child_positions.insert(
            child_state.id.clone(),
            LayoutPos::new(child_x, child_y, child_width, child_h),
        );

        svg.push_str(&render_state_node(
            child_state,
            &child_state.children,
            &LayoutPos::new(child_x, child_y, child_width, child_h),
            style,
            positions,
            measure,
        ));

        y_cursor += child_h + child_gap;
    }

    let mut pair_totals: HashMap<(String, String), usize> = HashMap::new();
    for transition in &child_transitions {
        *pair_totals
            .entry(state_pair_key(&transition.from, &transition.to))
            .or_insert(0) += 1;
    }

    let parent_rect = rect_from_pos(parent_pos, 0.0);
    let mut child_obstacles: Vec<Rect> = child_positions
        .values()
        .map(|p| rect_from_pos(p, 3.0))
        .collect();
    // Include global positions but exclude the parent composite (we're routing inside it)
    child_obstacles.extend(
        positions
            .values()
            .filter(|p| {
                let r = rect_from_pos(p, 0.0);
                !r.overlaps(&parent_rect) || r.w < parent_rect.w * 0.5
            })
            .map(|p| rect_from_pos(p, 3.0)),
    );

    let mut pair_seen: HashMap<(String, String), usize> = HashMap::new();
    let mut occupied_labels: Vec<Rect> = Vec::new();
    for transition in child_transitions {
        let key = state_pair_key(&transition.from, &transition.to);
        let route_index = pair_seen.get(&key).copied().unwrap_or(0);
        pair_seen.insert(key.clone(), route_index + 1);
        let route_total = pair_totals.get(&key).copied().unwrap_or(1);

        let from = child_positions
            .get(&transition.from)
            .or_else(|| positions.get(&transition.from));
        let to = child_positions
            .get(&transition.to)
            .or_else(|| positions.get(&transition.to));
        if let (Some(from_pos), Some(to_pos)) = (from, to) {
            let (t_svg, _ext) = render_state_transition(&mut StateTransitionContext {
                transition,
                from: from_pos,
                to: to_pos,
                style,
                measure,
                route_index,
                route_total,
                occupied_labels: &mut occupied_labels,
                obstacles: &child_obstacles,
            });
            svg.push_str(&t_svg);
        }
    }

    for (note_state, text) in child_notes {
        if let Some(target) = child_positions
            .get(note_state)
            .or_else(|| positions.get(note_state))
        {
            svg.push_str(&render_state_note(note_state, text, target, style, measure));
        }
    }

    svg
}

fn render_state_note(
    _state_id: &str,
    text: &str,
    state_pos: &LayoutPos,
    style: &DiagramStyle,
    measure: &mut impl TextMeasure,
) -> String {
    let cleaned = crate::xml::sanitize_xml_text(text);
    let note_width = (measure
        .measure_text(&cleaned, style.font_size * 0.8, false, false, false, None)
        .0
        + 16.0)
        .clamp(72.0, 180.0);
    let note_height = 26.0;
    let x = state_pos.x + state_pos.width + 28.0;
    let y = state_pos.y + 4.0;

    let y_mid = y + note_height / 2.0;
    let line_x1 = state_pos.x + state_pos.width;
    let line_x2 = x;

    format!(
        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1" stroke-dasharray="4,3" /><rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="3" fill="{}" fill-opacity="0.25" stroke="{}" stroke-width="1" /><text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}">{}</text>"#,
        line_x1,
        y_mid,
        line_x2,
        y_mid,
        style.edge_stroke,
        x,
        y,
        note_width,
        note_height,
        style.node_fill,
        style.node_stroke,
        x + 8.0,
        y + note_height * 0.65,
        style.font_family,
        style.font_size * 0.8,
        style.edge_text,
        escape_xml(text)
    )
}

struct StateTransitionContext<'a, T: TextMeasure> {
    transition: &'a StateTransition,
    from: &'a LayoutPos,
    to: &'a LayoutPos,
    style: &'a DiagramStyle,
    measure: &'a mut T,
    route_index: usize,
    route_total: usize,
    occupied_labels: &'a mut Vec<Rect>,
    obstacles: &'a [Rect],
}

fn render_state_transition<T: TextMeasure>(
    ctx: &mut StateTransitionContext<'_, T>,
) -> (String, Rect) {
    let transition = ctx.transition;
    let from = ctx.from;
    let to = ctx.to;
    let style = ctx.style;
    let measure = &mut *ctx.measure;
    let route_index = ctx.route_index;
    let route_total = ctx.route_total;
    let occupied_labels = &mut *ctx.occupied_labels;
    let obstacles = ctx.obstacles;

    let mut svg = String::new();
    let mut ext_min_x = f32::MAX;
    let mut ext_min_y = f32::MAX;
    let mut ext_max_x = f32::MIN;
    let mut ext_max_y = f32::MIN;

    macro_rules! track_point {
        ($x:expr, $y:expr) => {
            ext_min_x = ext_min_x.min($x);
            ext_min_y = ext_min_y.min($y);
            ext_max_x = ext_max_x.max($x);
            ext_max_y = ext_max_y.max($y);
        };
    }

    let (from_cx, from_cy) = from.center();
    let (to_cx, to_cy) = to.center();
    let center_angle = (to_cy - from_cy).atan2(to_cx - from_cx);

    let (px1, py1) = if from.width == from.height && from.width < 30.0 {
        (
            from_cx + center_angle.cos() * (from.width / 2.0),
            from_cy + center_angle.sin() * (from.width / 2.0),
        )
    } else {
        rect_boundary_point(from, center_angle)
    };

    let (px2, py2) = if to.width == to.height && to.width < 30.0 {
        (
            to_cx + (center_angle + std::f32::consts::PI).cos() * (to.width / 2.0),
            to_cy + (center_angle + std::f32::consts::PI).sin() * (to.width / 2.0),
        )
    } else {
        rect_boundary_point(to, center_angle + std::f32::consts::PI)
    };

    let route_hash = transition
        .from
        .bytes()
        .chain(transition.to.bytes())
        .fold(0_u32, |acc, value| {
            acc.wrapping_mul(31).wrapping_add(value as u32)
        });
    let lane = if route_total > 1 {
        route_index as f32 - (route_total as f32 - 1.0) / 2.0
    } else {
        0.0
    };
    let lane_offset = lane * 30.0;
    let global_lane = (route_hash % 5) as f32 - 2.0;
    let global_offset = global_lane * 6.0;
    let route_side = match transition.from.cmp(&transition.to) {
        std::cmp::Ordering::Less => 1.0,
        std::cmp::Ordering::Greater => -1.0,
        std::cmp::Ordering::Equal => {
            if route_hash % 2 == 0 {
                1.0
            } else {
                -1.0
            }
        }
    };
    let verticalish = (from_cx - to_cx).abs() < (from.width.min(to.width)) / 2.0 && (py2 - py1).abs() > 30.0;

    let label_anchor_x;
    let label_anchor_y;
    let arrow_angle;
    let mut arrow_x = px2;
    let mut arrow_y = py2;

    if verticalish {
        // Determine if from and to are adjacent (no boxes between them)
        let gap = (to.y - from.bottom()).max(from.y - to.bottom());
        let (top_y, bot_y) = if from_cy < to_cy {
            (from.bottom(), to.y)
        } else {
            (to.bottom(), from.y)
        };
        let mid_x = (from_cx + to_cx) / 2.0;
        let is_src_or_dst = |r: &Rect| -> bool {
            let rcx = r.x + r.w / 2.0;
            let rcy = r.y + r.h / 2.0;
            ((rcx - from_cx).abs() < 1.0 && (rcy - from_cy).abs() < 1.0)
                || ((rcx - to_cx).abs() < 1.0 && (rcy - to_cy).abs() < 1.0)
        };
        let has_obstacle_between = gap > 0.0 && obstacles.iter().any(|r| {
            if is_src_or_dst(r) {
                return false;
            }
            // Check if a straight vertical line from from→to would cross this obstacle
            let line_x = mid_x;
            line_x >= r.x && line_x <= r.x + r.w && r.y < bot_y && r.y + r.h > top_y
        });
        // Also check if the straight line crosses any obstacle using full intersection test
        let straight_crosses = obstacles.iter().any(|r| {
            if is_src_or_dst(r) {
                return false;
            }
            line_intersects_rect(px1, py1, px2, py2, r)
        });
        let adjacent = gap > 0.0 && gap < 60.0 && !has_obstacle_between && !straight_crosses;

        if adjacent {
            // Adjacent states: draw a straight vertical line
            let x = (from_cx + to_cx) / 2.0;
            let y1 = if from_cy < to_cy { from.bottom() } else { from.y };
            let y2 = if from_cy < to_cy { to.y } else { to.bottom() };
            svg.push_str(&format!(
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75" />"#,
                x, y1, x, y2, style.edge_stroke
            ));
            track_point!(x, y1);
            track_point!(x, y2);
            label_anchor_x = x;
            label_anchor_y = (y1 + y2) / 2.0;
            arrow_angle = if from_cy < to_cy {
                -std::f32::consts::FRAC_PI_2 // arrowhead points up (into top of target)
            } else {
                std::f32::consts::FRAC_PI_2
            };
            arrow_x = x;
            arrow_y = y2;
        } else {
            // Non-adjacent: use orthogonal routing (out → down → in)
            let max_half_width = (from.width / 2.0).max(to.width / 2.0);
            let exit_y = from_cy;
            let enter_y = to_cy;
            let is_endpoint = |r: &Rect| -> bool {
                let rcx = r.x + r.w / 2.0;
                let rcy = r.y + r.h / 2.0;
                ((rcx - from_cx).abs() < 1.0 && (rcy - from_cy).abs() < 1.0)
                    || ((rcx - to_cx).abs() < 1.0 && (rcy - to_cy).abs() < 1.0)
            };

            // Try both sides and pick the one that clears first (fewer steps)
            let step = 18.0;
            let max_steps = 30;

            let mut best_lane_x = from_cx + route_side * (max_half_width + 30.0);
            let mut best_step_count = max_steps;

            for &try_side in &[route_side, -route_side] {
                let base = from_cx + try_side * (max_half_width + 30.0 + lane.abs() * 14.0);
                let fex = if base >= from_cx {
                    from.x + from.width
                } else {
                    from.x
                };
                let tex = if base >= to_cx {
                    to.x + to.width
                } else {
                    to.x
                };
                for i in 0..max_steps {
                    let candidate = base + try_side * (i as f32) * step;
                    let clear = obstacles.iter().all(|r| {
                        if is_endpoint(r) {
                            return true;
                        }
                        let rr = r.expanded(6.0);
                        !hseg_hits_rect(exit_y, fex, candidate, &rr)
                            && !vseg_hits_rect(candidate, exit_y, enter_y, &rr)
                            && !hseg_hits_rect(enter_y, candidate, tex, &rr)
                    });
                    if clear && i < best_step_count {
                        best_lane_x = candidate;
                        best_step_count = i;
                        break;
                    }
                }
            }

            let lane_x = best_lane_x;
            let from_exit_x = if lane_x >= from_cx {
                from.x + from.width
            } else {
                from.x
            };
            let to_enter_x = if lane_x >= to_cx {
                to.x + to.width
            } else {
                to.x
            };

            svg.push_str(&format!(
                r#"<polyline points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="none" stroke="{}" stroke-width="0.75" />"#,
                from_exit_x, exit_y, lane_x, exit_y, lane_x, enter_y, to_enter_x, enter_y,
                style.edge_stroke
            ));
            track_point!(from_exit_x, exit_y);
            track_point!(lane_x, exit_y);
            track_point!(lane_x, enter_y);
            track_point!(to_enter_x, enter_y);
            label_anchor_x = lane_x;
            label_anchor_y = (exit_y + enter_y) / 2.0;
            arrow_angle = if to_enter_x > lane_x {
                0.0 // pointing right
            } else {
                std::f32::consts::PI // pointing left
            };
            arrow_x = to_enter_x;
            arrow_y = enter_y;
        }
    } else {
        // Check if a straight line would cross any obstacles
        let is_from_or_to = |r: &Rect| -> bool {
            let rcx = r.x + r.w / 2.0;
            let rcy = r.y + r.h / 2.0;
            ((rcx - from_cx).abs() < 1.0 && (rcy - from_cy).abs() < 1.0)
                || ((rcx - to_cx).abs() < 1.0 && (rcy - to_cy).abs() < 1.0)
        };
        let straight_blocked = obstacles.iter().any(|r| {
            if is_from_or_to(r) {
                return false;
            }
            line_intersects_rect(px1, py1, px2, py2, r)
        });

        if straight_blocked {
            // Obstacle-aware orthogonal routing for non-verticalish blocked paths.
            // We try multiple strategies and pick the first clear one:
            //   1. Simple Z-route (exit side → horizontal mid → enter side)
            //   2. U-route via vertical center exit (exit top/bottom → horizontal lane → enter top/bottom)
            //   3. Side-exit L-route (exit side → vertical lane → enter side) - like verticalish routing
            let is_endpoint = |r: &Rect| -> bool {
                let rcx = r.x + r.w / 2.0;
                let rcy = r.y + r.h / 2.0;
                ((rcx - from_cx).abs() < 1.0 && (rcy - from_cy).abs() < 1.0)
                    || ((rcx - to_cx).abs() < 1.0 && (rcy - to_cy).abs() < 1.0)
            };

            let step = 18.0;
            let max_steps = 30;

            // Strategy 1: Simple Z-route (exit from side, horizontal midline, enter from side)
            let going_right = to_cx > from_cx;
            let simple_from_exit_x = if going_right { from.x + from.width } else { from.x };
            let simple_to_enter_x = if going_right { to.x } else { to.x + to.width };
            let mid_y = (from_cy + to_cy) / 2.0;
            let simple_clear = obstacles.iter().all(|r| {
                if is_endpoint(r) {
                    return true;
                }
                let rr = r.expanded(6.0);
                !vseg_hits_rect(simple_from_exit_x, from_cy, mid_y, &rr)
                    && !hseg_hits_rect(mid_y, simple_from_exit_x, simple_to_enter_x, &rr)
                    && !vseg_hits_rect(simple_to_enter_x, mid_y, to_cy, &rr)
            });

            if simple_clear {
                // Use simpler Z-route since it's clear
                svg.push_str(&format!(
                    r#"<polyline points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="none" stroke="{}" stroke-width="0.75" />"#,
                    simple_from_exit_x, from_cy, simple_from_exit_x, mid_y, simple_to_enter_x, mid_y, simple_to_enter_x, to_cy,
                    style.edge_stroke
                ));
                track_point!(simple_from_exit_x, from_cy);
                track_point!(simple_from_exit_x, mid_y);
                track_point!(simple_to_enter_x, mid_y);
                track_point!(simple_to_enter_x, to_cy);
                label_anchor_x = (simple_from_exit_x + simple_to_enter_x) / 2.0;
                label_anchor_y = mid_y;
                arrow_angle = if to_cy > mid_y {
                    -std::f32::consts::FRAC_PI_2
                } else {
                    std::f32::consts::FRAC_PI_2
                };
                arrow_x = simple_to_enter_x;
                arrow_y = to_cy;
            } else {
                // Strategy 2: U-route via vertical center exit
                let going_down = to_cy > from_cy;
                let pref_side: f32 = if going_down { 1.0 } else { -1.0 };
                let mut best_u_lane_y = f32::NAN;
                let mut best_u_steps = max_steps;

                for &try_side in &[pref_side, -pref_side] {
                    let base = if try_side > 0.0 {
                        from.bottom().max(to.bottom()) + 30.0 + lane.abs() * 14.0
                    } else {
                        from.y.min(to.y) - 30.0 - lane.abs() * 14.0
                    };
                    let fey = if try_side > 0.0 { from.bottom() } else { from.y };
                    let tey = if try_side > 0.0 { to.bottom() } else { to.y };
                    for i in 0..max_steps {
                        let candidate = base + try_side * (i as f32) * step;
                        let clear = obstacles.iter().all(|r| {
                            if is_endpoint(r) {
                                return true;
                            }
                            let rr = r.expanded(6.0);
                            !vseg_hits_rect(from_cx, fey, candidate, &rr)
                                && !hseg_hits_rect(candidate, from_cx, to_cx, &rr)
                                && !vseg_hits_rect(to_cx, candidate, tey, &rr)
                        });
                        if clear && i < best_u_steps {
                            best_u_lane_y = candidate;
                            best_u_steps = i;
                            break;
                        }
                    }
                }

                // Strategy 3: Side-exit L-route (exit from side, vertical lane, enter from side)
                // Same approach as the verticalish non-adjacent routing.
                let max_half_width = (from.width / 2.0).max(to.width / 2.0);
                let exit_y = from_cy;
                let enter_y = to_cy;
                let mut best_side_lane_x = f32::NAN;
                let mut best_side_steps = max_steps;

                for &try_side in &[route_side, -route_side] {
                    let base_x = from_cx + try_side * (max_half_width + 30.0 + lane.abs() * 14.0);
                    let fex = if base_x >= from_cx { from.x + from.width } else { from.x };
                    let tex = if base_x >= to_cx { to.x + to.width } else { to.x };
                    for i in 0..max_steps {
                        let candidate = base_x + try_side * (i as f32) * step;
                        let clear = obstacles.iter().all(|r| {
                            if is_endpoint(r) {
                                return true;
                            }
                            let rr = r.expanded(6.0);
                            !hseg_hits_rect(exit_y, fex, candidate, &rr)
                                && !vseg_hits_rect(candidate, exit_y, enter_y, &rr)
                                && !hseg_hits_rect(enter_y, candidate, tex, &rr)
                        });
                        if clear && i < best_side_steps {
                            best_side_lane_x = candidate;
                            best_side_steps = i;
                            break;
                        }
                    }
                }

                // Pick the best strategy: prefer fewer steps (closer route)
                let use_u_route = !best_u_lane_y.is_nan()
                    && (best_side_lane_x.is_nan() || best_u_steps <= best_side_steps);

                if use_u_route {
                    let lane_y = best_u_lane_y;
                    let from_exit_y = if lane_y > from_cy { from.bottom() } else { from.y };
                    let to_enter_y = if lane_y > to_cy { to.bottom() } else { to.y };

                    svg.push_str(&format!(
                        r#"<polyline points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="none" stroke="{}" stroke-width="0.75" />"#,
                        from_cx, from_exit_y, from_cx, lane_y, to_cx, lane_y, to_cx, to_enter_y,
                        style.edge_stroke
                    ));
                    track_point!(from_cx, from_exit_y);
                    track_point!(from_cx, lane_y);
                    track_point!(to_cx, lane_y);
                    track_point!(to_cx, to_enter_y);
                    label_anchor_x = (from_cx + to_cx) / 2.0;
                    label_anchor_y = lane_y;
                    arrow_angle = if to_enter_y > lane_y {
                        -std::f32::consts::FRAC_PI_2
                    } else {
                        std::f32::consts::FRAC_PI_2
                    };
                    arrow_x = to_cx;
                    arrow_y = to_enter_y;
                } else if !best_side_lane_x.is_nan() {
                    let lane_x = best_side_lane_x;
                    let from_exit_x = if lane_x >= from_cx { from.x + from.width } else { from.x };
                    let to_enter_x = if lane_x >= to_cx { to.x + to.width } else { to.x };

                    svg.push_str(&format!(
                        r#"<polyline points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="none" stroke="{}" stroke-width="0.75" />"#,
                        from_exit_x, exit_y, lane_x, exit_y, lane_x, enter_y, to_enter_x, enter_y,
                        style.edge_stroke
                    ));
                    track_point!(from_exit_x, exit_y);
                    track_point!(lane_x, exit_y);
                    track_point!(lane_x, enter_y);
                    track_point!(to_enter_x, enter_y);
                    label_anchor_x = lane_x;
                    label_anchor_y = (exit_y + enter_y) / 2.0;
                    arrow_angle = if to_enter_x > lane_x { 0.0 } else { std::f32::consts::PI };
                    arrow_x = to_enter_x;
                    arrow_y = enter_y;
                } else {
                    // Fallback: draw the straight line anyway
                    svg.push_str(&format!(
                        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75" />"#,
                        px1, py1, px2, py2, style.edge_stroke
                    ));
                    track_point!(px1, py1);
                    track_point!(px2, py2);
                    arrow_angle = (py1 - py2).atan2(px1 - px2);
                    label_anchor_x = (px1 + px2) / 2.0;
                    label_anchor_y = (py1 + py2) / 2.0;
                }
            }
        } else {
            svg.push_str(&format!(
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75" />"#,
                px1, py1, px2, py2, style.edge_stroke
            ));
            track_point!(px1, py1);
            track_point!(px2, py2);
            arrow_angle = (py1 - py2).atan2(px1 - px2);

            // Jitter label anchor along the edge to reduce label-label collisions.
            let dx = px2 - px1;
            let dy = py2 - py1;
            let jitter = ((route_hash % 7) as f32 - 3.0) * 0.07;
            let t = (0.5 + jitter).clamp(0.25, 0.75);
            label_anchor_x = px1 + dx * t;
            label_anchor_y = py1 + dy * t;
        }
    }

    // Arrow
    let ax = arrow_x;
    let ay = arrow_y;
    let p1 = (
        ax + arrow_angle.cos() * 10.0 - arrow_angle.sin() * 5.0,
        ay + arrow_angle.sin() * 10.0 + arrow_angle.cos() * 5.0,
    );
    let p2 = (
        ax + arrow_angle.cos() * 10.0 + arrow_angle.sin() * 5.0,
        ay + arrow_angle.sin() * 10.0 - arrow_angle.cos() * 5.0,
    );

    svg.push_str(&format!(
        r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" />"#,
        ax, ay, p1.0, p1.1, p2.0, p2.1, style.edge_stroke
    ));

    // Label
    if let Some(ref label) = transition.label {
        let cleaned = crate::xml::sanitize_xml_text(label);
        let label_width = measure
            .measure_text(&cleaned, style.font_size * 0.85, false, false, false, None)
            .0
            + 8.0;
        let label_height = style.font_size * 0.8 + 6.0;
        let dx = px2 - px1;
        let dy = py2 - py1;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let tx = dx / len;
        let ty = dy / len;
        let perp_x = -ty;
        let perp_y = tx;
        let tangent_offset = lane_offset + global_offset;

        let rect_for = |lx: f32, ly: f32| Rect {
            x: lx - label_width / 2.0,
            y: ly - label_height + 2.0,
            w: label_width,
            h: label_height,
        };

        let score = |r: &Rect| -> f32 {
            let mut s = 0.0;
            if r.x < 0.0 || r.y < 0.0 {
                s += 1000.0;
            }
            for o in obstacles {
                if r.overlaps(o) {
                    s += 140.0;
                }
            }
            for o in occupied_labels.iter() {
                if r.overlaps(o) {
                    s += 220.0;
                }
            }
            s
        };

        // Candidate search: try different anchor t and perpendicular distances on both sides.
        let t_candidates: [f32; 5] = [0.38, 0.46, 0.5, 0.54, 0.62];
        let dist_candidates: [f32; 6] = [24.0, 34.0, 44.0, 54.0, 66.0, 78.0];
        let side_candidates: [f32; 2] = [route_side, -route_side];

        // Always include the previous heuristic as a candidate.
        let movement_weight = 2.0;
        let base_dist = 28.0 + lane.abs() * 10.0 + global_lane.abs() * 4.0;
        let base_x = label_anchor_x
            + perp_x * base_dist * route_side
            + tx * tangent_offset;
        let base_y = label_anchor_y
            + perp_y * base_dist * route_side
            + ty * tangent_offset;
        let base_rect = rect_for(base_x, base_y);
        let base_score = score(&base_rect);
        let mut best_x = base_x;
        let mut best_y = base_y;
        let mut best_rect = base_rect;
        let mut best_move =
            ((base_x - label_anchor_x).powi(2) + (base_y - label_anchor_y).powi(2)).sqrt();
        let mut best_cost = base_score + best_move * movement_weight;

        for &t in &t_candidates {
            let ax = px1 + dx * t;
            let ay = py1 + dy * t;
            for &side in &side_candidates {
                for &dist in &dist_candidates {
                    let lx = ax + perp_x * dist * side + tx * tangent_offset;
                    let ly = ay + perp_y * dist * side + ty * tangent_offset;
                    let r = rect_for(lx, ly);
                    let sc = score(&r);
                    let mv = ((lx - label_anchor_x).powi(2) + (ly - label_anchor_y).powi(2)).sqrt();
                    let cost = sc + mv * movement_weight;
                    if cost < best_cost || ((cost - best_cost).abs() < f32::EPSILON && mv < best_move) {
                        best_cost = cost;
                        best_move = mv;
                        best_x = lx;
                        best_y = ly;
                        best_rect = r;
                    }
                }
            }
        }

        // For vertical-ish routes (polyline), also try shifting sideways.
        if verticalish {
            for &side in &side_candidates {
                for &dist in &dist_candidates {
                    let lx = label_anchor_x + side * dist;
                    let ly = label_anchor_y + lane_offset * 0.5;
                    let r = rect_for(lx, ly);
                    let sc = score(&r);
                    let mv = ((lx - label_anchor_x).powi(2) + (ly - label_anchor_y).powi(2)).sqrt();
                    let cost = sc + mv * movement_weight;
                    if cost < best_cost || ((cost - best_cost).abs() < f32::EPSILON && mv < best_move) {
                        best_cost = cost;
                        best_move = mv;
                        best_x = lx;
                        best_y = ly;
                        best_rect = r;
                    }
                }
            }
        }

        occupied_labels.push(best_rect);
        track_point!(best_rect.x, best_rect.y);
        track_point!(best_rect.x + best_rect.w, best_rect.y + best_rect.h);

        svg.push_str(&format!(
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="3" fill="{}" stroke="{}" stroke-width="0.5" />"#,
            best_rect.x,
            best_rect.y,
            best_rect.w,
            best_rect.h,
            style.node_fill,
            style.node_stroke
        ));

        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="middle">{}</text>"#,
            best_x,
            best_y,
            style.font_family,
            style.font_size * 0.85,
            style.edge_text,
            escape_xml(&cleaned)
        ));
    }

    let extent = Rect {
        x: ext_min_x.min(0.0),
        y: ext_min_y.min(0.0),
        w: (ext_max_x - ext_min_x.min(0.0)).max(0.0),
        h: (ext_max_y - ext_min_y.min(0.0)).max(0.0),
    };
    (svg, extent)
}

fn state_pair_key(from: &str, to: &str) -> (String, String) {
    if from <= to {
        (from.to_string(), to.to_string())
    } else {
        (to.to_string(), from.to_string())
    }
}

// ============================================
// ER DIAGRAM RENDERING
// ============================================

fn render_er(
    diagram: &ErDiagram,
    style: &DiagramStyle,
    measure: &mut impl TextMeasure,
) -> Result<(String, f32, f32), String> {
    if diagram.entities.is_empty() {
        return Ok(("<g></g>".to_string(), 100.0, 50.0));
    }

    let mut layout = LayoutEngine::new(measure, style.font_size);
    let (positions, _edge_waypoints, bbox) = layout.layout_er(diagram);

    let mut svg = String::new();
    let padding = 20.0;
    let mut occupied_labels: Vec<Rect> = Vec::new();

    // Draw relationships first
    for relation in &diagram.relationships {
        let from_pos = positions.get(&relation.from);
        let to_pos = positions.get(&relation.to);

        if let (Some(from), Some(to)) = (from_pos, to_pos) {
            svg.push_str(&render_er_relationship(
                relation,
                from,
                to,
                style,
                measure,
                &mut occupied_labels,
            ));
        }
    }

    // Draw entities
    for entity in &diagram.entities {
        if let Some(pos) = positions.get(&entity.name) {
            svg.push_str(&render_er_entity(entity, pos, style));
        }
    }

    let total_width = bbox.right() + padding;
    let total_height = bbox.bottom() + padding;

    Ok((svg, total_width, total_height))
}

fn render_er_entity(entity: &ErEntity, pos: &LayoutPos, style: &DiagramStyle) -> String {
    let mut svg = String::new();

    // Entity box
    svg.push_str(&format!(
        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="{}" stroke-width="1" />"#,
        pos.x, pos.y, pos.width, pos.height,
        style.node_fill, style.node_stroke
    ));

    let mut y = pos.y + style.font_size + 6.0;

    // Entity name (bold)
    svg.push_str(&format!(
        r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="middle" font-weight="bold">{}</text>"#,
        pos.x + pos.width / 2.0, y, style.font_family, style.font_size, style.node_text, escape_xml(&entity.name)
    ));

    // Divider line between name and attributes
    if !entity.attributes.is_empty() {
        y += 4.0;
        svg.push_str(&format!(
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75" />"#,
            pos.x, y, pos.x + pos.width, y, style.node_stroke
        ));
        y += style.font_size * 0.5;
    }

    // Attributes
    y += style.font_size;
    for attr in &entity.attributes {
        let marker = if attr.is_key { "*" } else { "" };
        let attr_name = if attr.is_composite {
            format!("[{}]", attr.name)
        } else {
            attr.name.clone()
        };
        let attr_text = format!("{}{}", marker, attr_name);

        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}">{}</text>"#,
            pos.x + 8.0,
            y,
            style.font_family,
            style.font_size * 0.9,
            style.node_text,
            escape_xml(&attr_text)
        ));
        y += style.font_size * 1.3;
    }

    svg
}

fn render_er_relationship(
    relation: &ErRelationship,
    from: &LayoutPos,
    to: &LayoutPos,
    style: &DiagramStyle,
    measure: &mut impl TextMeasure,
    occupied_labels: &mut Vec<Rect>,
) -> String {
    let mut svg = String::new();

    let (from_cx, from_cy) = from.center();
    let (to_cx, to_cy) = to.center();
    let angle = (to_cy - from_cy).atan2(to_cx - from_cx);
    let (x1, y1) = rect_boundary_point(from, angle);
    let (x2, y2) = rect_boundary_point(to, angle + std::f32::consts::PI);

    // Line
    svg.push_str(&format!(
        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75" />"#,
        x1, y1, x2, y2, style.edge_stroke
    ));

    svg.push_str(&render_er_cardinality_marker(
        x1,
        y1,
        angle,
        &relation.from_cardinality,
        style,
    ));
    svg.push_str(&render_er_cardinality_marker(
        x2,
        y2,
        angle + std::f32::consts::PI,
        &relation.to_cardinality,
        style,
    ));

    if let Some(label) = &relation.label {
        let cleaned = crate::xml::sanitize_xml_text(label);
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let nx = -dy / len;
        let ny = dx / len;
        let mx = (x1 + x2) / 2.0;
        let my = (y1 + y2) / 2.0;
        let label_w = measure
            .measure_text(&cleaned, style.font_size * 0.8, false, false, false, None)
            .0
            + 8.0;
        let label_h = style.font_size * 0.75 + 6.0;

        let pad = 3.0;
        let from_x = from.x - pad;
        let from_y = from.y - pad;
        let from_w = from.width + pad * 2.0;
        let from_h = from.height + pad * 2.0;
        let to_x = to.x - pad;
        let to_y = to.y - pad;
        let to_w = to.width + pad * 2.0;
        let to_h = to.height + pad * 2.0;

        // Choose the normal direction that avoids overlapping endpoints.
        let rect_for = |lx: f32, ly: f32| Rect {
            x: lx - label_w / 2.0,
            y: ly - label_h + 2.0,
            w: label_w,
            h: label_h,
        };

        let mut best_dir = (nx, ny);
        let mut best_penalty = i32::MAX;
        for (cx, cy) in [(nx, ny), (-nx, -ny)] {
            let off = 22.0;
            let lx = mx + cx * off;
            let ly = my + cy * off;
            let r = rect_for(lx, ly);
            let mut penalty = 0;
            if r.overlaps(&Rect {
                x: from_x,
                y: from_y,
                w: from_w,
                h: from_h,
            }) {
                penalty += 1;
            }
            if r.overlaps(&Rect {
                x: to_x,
                y: to_y,
                w: to_w,
                h: to_h,
            }) {
                penalty += 1;
            }
            if penalty < best_penalty {
                best_penalty = penalty;
                best_dir = (cx, cy);
            }
            if penalty == 0 {
                break;
            }
        }

        let from_r = Rect {
            x: from_x,
            y: from_y,
            w: from_w,
            h: from_h,
        };
        let to_r = Rect {
            x: to_x,
            y: to_y,
            w: to_w,
            h: to_h,
        };

        let mut label_x = mx;
        let mut label_y = my;
        let mut best_rect = rect_for(label_x, label_y);
        let mut best_score = i32::MAX;

        let tx = dx / len;
        let ty = dy / len;
        for off in [22.0_f32, 32.0, 44.0, 56.0, 68.0, 80.0] {
            for tangent in [-28.0_f32, -14.0, 0.0, 14.0, 28.0] {
                for (sx, sy) in [(best_dir.0, best_dir.1), (-best_dir.0, -best_dir.1)] {
                    let lx = mx + sx * off + tx * tangent;
                    let ly = my + sy * off + ty * tangent;
                    let r = rect_for(lx, ly);
                    let mut sc = 0;
                    if r.overlaps(&from_r) {
                        sc += 500;
                    }
                    if r.overlaps(&to_r) {
                        sc += 500;
                    }
                    for o in occupied_labels.iter() {
                        if r.overlaps(o) {
                            sc += 800;
                        }
                    }
                    if sc < best_score {
                        best_score = sc;
                        label_x = lx;
                        label_y = ly;
                        best_rect = r;
                        if best_score == 0 {
                            break;
                        }
                    }
                }
                if best_score == 0 {
                    break;
                }
            }
            if best_score == 0 {
                break;
            }
        }

        occupied_labels.push(best_rect);

        svg.push_str(&format!(
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="3" fill="{}" stroke="{}" stroke-width="0.5" />"#,
            best_rect.x,
            best_rect.y,
            best_rect.w,
            best_rect.h,
            style.node_fill,
            style.node_stroke
        ));

        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="middle">{}</text>"#,
            label_x,
            label_y,
            style.font_family,
            style.font_size * 0.8,
            style.edge_text,
            escape_xml(&cleaned)
        ));
    }

    svg
}

fn rect_boundary_point(rect: &LayoutPos, angle: f32) -> (f32, f32) {
    let (cx, cy) = rect.center();
    let dx = angle.cos();
    let dy = angle.sin();
    let half_w = rect.width / 2.0;
    let half_h = rect.height / 2.0;

    let tx = if dx.abs() > 1e-5 {
        half_w / dx.abs()
    } else {
        f32::INFINITY
    };
    let ty = if dy.abs() > 1e-5 {
        half_h / dy.abs()
    } else {
        f32::INFINITY
    };
    let t = tx.min(ty);

    (cx + dx * t, cy + dy * t)
}

fn render_er_cardinality_marker(
    x: f32,
    y: f32,
    angle: f32,
    cardinality: &ErCardinality,
    style: &DiagramStyle,
) -> String {
    let ux = angle.cos();
    let uy = angle.sin();
    let nx = -uy;
    let ny = ux;

    let mut marker = String::new();

    let draw_one = |dist: f32| {
        format!(
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75" />"#,
            x + ux * dist + nx * 6.0,
            y + uy * dist + ny * 6.0,
            x + ux * dist - nx * 6.0,
            y + uy * dist - ny * 6.0,
            style.edge_stroke
        )
    };

    let draw_zero = |dist: f32| {
        format!(
            r#"<circle cx="{:.2}" cy="{:.2}" r="4.50" fill="{}" stroke="{}" stroke-width="1.2" />"#,
            x + ux * dist,
            y + uy * dist,
            style.background,
            style.edge_stroke
        )
    };

    let draw_many = |dist: f32| {
        let cx = x + ux * dist;
        let cy = y + uy * dist;
        format!(
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75" /><line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75" /><line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75" />"#,
            cx,
            cy,
            cx + ux * 8.0 + nx * 8.0,
            cy + uy * 8.0 + ny * 8.0,
            style.edge_stroke,
            cx,
            cy,
            cx + ux * 10.0,
            cy + uy * 10.0,
            style.edge_stroke,
            cx,
            cy,
            cx + ux * 8.0 - nx * 8.0,
            cy + uy * 8.0 - ny * 8.0,
            style.edge_stroke
        )
    };

    match cardinality {
        ErCardinality::ExactlyOne => {
            marker.push_str(&draw_one(8.0));
            marker.push_str(&draw_one(14.0));
        }
        ErCardinality::ZeroOrOne => {
            marker.push_str(&draw_zero(8.0));
            marker.push_str(&draw_one(16.0));
        }
        ErCardinality::ZeroOrMore => {
            marker.push_str(&draw_zero(8.0));
            marker.push_str(&draw_many(16.0));
        }
        ErCardinality::OneOrMore => {
            marker.push_str(&draw_one(8.0));
            marker.push_str(&draw_many(16.0));
        }
    }

    marker
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockMeasure;

    impl TextMeasure for MockMeasure {
        fn measure_text(
            &mut self,
            text: &str,
            font_size: f32,
            _is_code: bool,
            _is_bold: bool,
            _is_italic: bool,
            _max_width: Option<f32>,
        ) -> (f32, f32) {
            (text.chars().count() as f32 * font_size * 0.6, font_size)
        }
    }

    #[test]
    fn render_class_includes_relations_and_labels() {
        let src = r#"classDiagram
  class User {
    +String id
  }
  class Session {
    +String token
  }
  class AuditLog {
    +record(event: String): void
  }
  User --> Session : creates
  User ..> AuditLog : writes
"#;
        let style = DiagramStyle::default();
        let mut measure = MockMeasure;

        let (svg, _w, _h) = render_diagram(src, &style, &mut measure).unwrap();
        assert!(svg.contains("creates"));
        assert!(svg.contains("writes"));
    }

    #[test]
    fn from_theme_produces_color_hierarchy() {
        let style = DiagramStyle::from_theme("#586e75", "#fdf6e3", "#073642");
        // edge_text should be a 60% mix of code_bg toward fg (not raw fg)
        assert_ne!(style.edge_text, style.node_text);
        // node_fill should be very close to code_bg (3% fg mix)
        assert_ne!(style.node_fill, "#073642");
        // node_stroke should be lighter than node_text (20% vs 100% fg)
        assert_ne!(style.node_stroke, style.node_text);
    }

    #[test]
    fn inheritance_marker_is_simple_triangle() {
        let src = r#"classDiagram
  class A
  class B
  A <|-- B
"#;
        let style = DiagramStyle::default();
        let mut measure = MockMeasure;
        let (svg, _w, _h) = render_diagram(src, &style, &mut measure).unwrap();

        let marker = "<polygon points=\"";
        let idx = svg.find(marker).expect("expected class marker polygon");
        let rest = &svg[idx + marker.len()..];
        let end = rest.find('"').expect("expected points terminator");
        let points = &rest[..end];
        let pair_count = points.split_whitespace().count();
        assert_eq!(pair_count, 3, "inheritance triangle should have exactly 3 points");
    }

    #[test]
    fn test_unknown_diagram_type_renders_without_panic() {
        let style = DiagramStyle::default();
        let mut measure = MockMeasure;
        let result = render_diagram("pie title Pets\n  \"Dogs\" : 50", &style, &mut measure);
        assert!(result.is_ok(), "Unknown diagram type should render without error");
    }

    #[test]
    fn test_syntax_error_renders_without_panic() {
        let style = DiagramStyle::default();
        let mut measure = MockMeasure;
        let result =
            render_diagram("flowchart LR\n  ??? invalid syntax ???", &style, &mut measure);
        assert!(result.is_ok(), "Syntax errors should not cause panic");
    }
}
