//! Timeline diagram: parser, layout, and SVG renderer.
//!
//! Syntax:
//! ```mermaid
//! timeline
//!     title History
//!     2023 : Founded
//!     2024 : Series A
//!          : Launched product
//! ```

use std::collections::HashMap;

use crate::layout::{BBox, LayoutPos};
use crate::TextMeasure;

// ── Types ──

#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub period: String,
    pub events: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Timeline {
    pub title: Option<String>,
    pub events: Vec<TimelineEvent>,
}

// ── Parser ──

pub fn parse_timeline(input: &str) -> Result<Timeline, String> {
    let mut title = None;
    let mut events: Vec<TimelineEvent> = Vec::new();
    let mut current_period: Option<String> = None;
    let mut current_events: Vec<String> = Vec::new();

    for line in input.lines().skip(1) {
        // skip "timeline" header line
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if let Some(t) = line.strip_prefix("title ") {
            title = Some(t.trim().to_string());
            continue;
        }

        // Period line: "YYYY : Description" or "Label : Description"
        if let Some(colon_pos) = line.find(':') {
            let period_part = line[..colon_pos].trim();
            let event_part = line[colon_pos + 1..].trim();

            if !period_part.is_empty() {
                // Flush previous period
                if let Some(period) = current_period.take() {
                    events.push(TimelineEvent {
                        period,
                        events: std::mem::take(&mut current_events),
                    });
                }
                // Start new period
                current_period = Some(period_part.to_string());
                if !event_part.is_empty() {
                    current_events.push(event_part.to_string());
                }
            } else {
                // ": event" — continuation under current period
                if !event_part.is_empty() {
                    current_events.push(event_part.to_string());
                }
            }
        }
    }

    // Flush last period
    if let Some(period) = current_period {
        events.push(TimelineEvent {
            period,
            events: current_events,
        });
    }

    if events.is_empty() {
        return Err("timeline: no events found".to_string());
    }

    Ok(Timeline { title, events })
}

// ── Layout ──

pub fn layout_timeline(timeline: &Timeline, measure: &mut impl TextMeasure, font_size: f32) -> (HashMap<String, LayoutPos>, BBox) {
    let mut positions = HashMap::new();
    let n = timeline.events.len();
    if n == 0 {
        return (positions, BBox::default());
    }

    let period_font = font_size;
    let event_font = font_size * 0.85;
    let marker_r = 6.0;
    let pad_x = 20.0;
    let pad_y = 12.0;

    // Measure period labels and event labels
    let period_widths: Vec<f32> = timeline.events.iter()
        .map(|e| measure_text(measure, &e.period, period_font, true))
        .collect();

    let mut max_events_per_period = 0usize;
    let mut event_size_map: Vec<Vec<(f32, f32)>> = Vec::new();
    for event in &timeline.events {
        let sizes: Vec<(f32, f32)> = event.events.iter()
            .map(|ev| {
                let w = measure_text(measure, ev, event_font, false);
                (w, event_font * 1.3)
            })
            .collect();
        max_events_per_period = max_events_per_period.max(sizes.len());
        event_size_map.push(sizes);
    }

    let marker_y = 60.0; // y position of the horizontal timeline axis
    let event_start_y = marker_y + marker_r + pad_y;
    let node_height = 40.0;

    // Position periods along horizontal axis
    let mut x = 40.0;
    let event_spacing = 80.0;

    for i in 0..n {
        let p_w = period_widths[i];
        let node_w = (p_w + pad_x * 2.0).max(80.0);

        // Period marker on the axis
        positions.insert(format!("tl-marker-{i}"), LayoutPos::new(x, marker_y - marker_r, marker_r * 2.0, marker_r * 2.0));

        // Period label above marker
        let label_x = x - node_w / 2.0;
        positions.insert(format!("tl-period-{i}"), LayoutPos::new(label_x, marker_y - marker_r - node_height - 4.0, node_w, node_height));

        // Event labels below marker
        let mut ey = event_start_y;
        for j in 0..event_size_map[i].len() {
            let (ew, eh) = event_size_map[i][j];
            let event_node_w = (ew + pad_x * 2.0).max(60.0);
            positions.insert(format!("tl-event-{i}-{j}"), LayoutPos::new(x - event_node_w / 2.0, ey, event_node_w, eh));
            ey += eh + 4.0;
        }

        x += node_w.max(period_widths[i] + pad_x * 2.0) + event_spacing;
    }

    let mut min_x = f32::MAX; let mut min_y = f32::MAX;
    let mut max_x = f32::MIN; let mut max_y = f32::MIN;
    for pos in positions.values() {
        min_x = min_x.min(pos.x); min_y = min_y.min(pos.y);
        max_x = max_x.max(pos.x + pos.width); max_y = max_y.max(pos.y + pos.height);
    }
    let bbox = BBox::new(min_x, min_y, max_x - min_x + 40.0, max_y - min_y + 40.0);
    (positions, bbox)
}

// ── Render ──

pub fn render_timeline(timeline: &Timeline, style: &crate::render::DiagramStyle, measure: &mut impl TextMeasure) -> Result<(String, f32, f32), String> {
    let (positions, bbox) = layout_timeline(timeline, measure, style.font_size);

    let mut svg = String::new();
    let n = timeline.events.len();

    // Title
    if let Some(ref title) = timeline.title {
        let _tw = measure_text(measure, title, style.font_size * 1.2, true);
        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            bbox.x + bbox.width / 2.0, 28.0, style.font_family, style.font_size * 1.2, style.node_text, crate::render::escape_xml(title)
        ));
    }

    // Horizontal axis line
    let axis_y = positions.get("tl-marker-0").map(|p| p.y + p.height / 2.0).unwrap_or(60.0);
    let left_x = positions.values().map(|p| p.x).fold(f32::MAX, f32::min);
    let right_x = positions.values().map(|p| p.x + p.width).fold(f32::MIN, f32::max);
    svg.push_str(&format!(
        r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.5" />"#,
        left_x, axis_y, right_x, axis_y, style.edge_stroke
    ));

    // Period markers and labels
    for i in 0..n {
        // Marker circle
        if let Some(pos) = positions.get(&format!("tl-marker-{i}")) {
            let cx = pos.x + pos.width / 2.0;
            let cy = pos.y + pos.height / 2.0;
            svg.push_str(&format!(
                r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}" stroke="{}" stroke-width="1.5" />"#,
                cx, cy, 6.0, style.node_fill, style.node_stroke
            ));
            // Vertical connector line
            svg.push_str(&format!(
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="0.75" />"#,
                cx, cy - 10.0, cx, cy + 14.0, style.edge_stroke
            ));
        }

        // Period label
        if let Some(pos) = positions.get(&format!("tl-period-{i}")) {
            let rx = 4.0;
            svg.push_str(&format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" fill="{}" stroke="{}" stroke-width="1" />"#,
                pos.x, pos.y, pos.width, pos.height, rx, style.node_fill, style.node_stroke
            ));
            svg.push_str(&format!(
                r#"<text x="{:.2}" y="{:.2}" dy="0.35em" font-family="{}" font-size="{:.1}" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
                pos.x + pos.width / 2.0, pos.y + pos.height / 2.0, style.font_family, style.font_size, style.node_text,
                crate::render::escape_xml(&timeline.events[i].period)
            ));
        }

        // Event labels
        for j in 0..timeline.events[i].events.len() {
            if let Some(pos) = positions.get(&format!("tl-event-{i}-{j}")) {
                svg.push_str(&format!(
                    r#"<text x="{:.2}" y="{:.2}" dy="0.35em" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="middle">{}</text>"#,
                    pos.x + pos.width / 2.0, pos.y + pos.height / 2.0, style.font_family, style.font_size * 0.85, style.edge_text,
                    crate::render::escape_xml(&timeline.events[i].events[j])
                ));
            }
        }
    }

    let total_w = bbox.width + 40.0;
    let total_h = bbox.height + 20.0;
    Ok((svg, total_w, total_h))
}

fn measure_text(measure: &mut impl TextMeasure, text: &str, font_size: f32, bold: bool) -> f32 {
    measure.measure_text(text, font_size, false, bold, false, None).0
}
