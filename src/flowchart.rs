use std::collections::HashMap;

use crate::TextMeasure;

use crate::Rect;
use super::layout::{BBox, LayoutEngine, LayoutPos};
use super::render::{DiagramStyle, escape_xml};
use super::types::{ArrowType, EdgeStyle, FlowDirection, Flowchart, NodeShape, NodeStyle};

/// Build a map from node_id → effective NodeStyle (classDef + inline styles merged).
fn build_node_style_map(flowchart: &Flowchart) -> std::collections::HashMap<String, NodeStyle> {
    use std::collections::HashMap;
    let mut map: HashMap<String, NodeStyle> = HashMap::new();
    for (node_id, style) in &flowchart.node_styles {
        map.insert(node_id.clone(), style.clone());
    }
    for class_def in &flowchart.class_defs {
        for node in &flowchart.nodes {
            if node.class_names.contains(&class_def.name) {
                let entry = map.entry(node.id.clone()).or_default();
                if class_def.style.fill.is_some() { entry.fill = class_def.style.fill.clone(); }
                if class_def.style.stroke.is_some() { entry.stroke = class_def.style.stroke.clone(); }
                if class_def.style.stroke_width.is_some() { entry.stroke_width = class_def.style.stroke_width.clone(); }
                if class_def.style.color.is_some() { entry.color = class_def.style.color.clone(); }
            }
        }
    }
    map
}

fn build_link_style_map(flowchart: &Flowchart) -> std::collections::HashMap<usize, (Option<String>, Option<String>)> {
    flowchart.link_styles.iter()
        .map(|ls| (ls.index, (ls.stroke.clone(), ls.stroke_width.clone())))
        .collect()
}

/// Render a flowchart to SVG
pub fn render_flowchart(
    flowchart: &Flowchart,
    style: &DiagramStyle,
    measure: &mut impl TextMeasure,
) -> Result<(String, f32, f32), String> {
    if flowchart.nodes.is_empty() {
        return Ok(("<g></g>".to_string(), 100.0, 50.0));
    }

    let mut layout = LayoutEngine::new(measure, style.font_size);
    let (positions, edge_waypoints, bbox) = layout.layout_flowchart(flowchart);

    let mut svg = String::new();
    let padding = 20.0;

    let node_map: HashMap<&str, &super::types::FlowchartNode> =
        flowchart.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Draw subgraph boxes first (background layer)
    for subgraph in &flowchart.subgraphs {
        svg.push_str(&render_subgraph_box(subgraph, &positions, style));
    }

    let node_style_map = build_node_style_map(flowchart);
    let link_style_map = build_link_style_map(flowchart);

    // Draw edges (behind nodes but on top of subgraph boxes)
    for (edge_idx, edge) in flowchart.edges.iter().enumerate() {
        let from_pos = positions.get(&edge.from);
        let to_pos = positions.get(&edge.to);
        let from_node = node_map.get(edge.from.as_str());
        let to_node = node_map.get(edge.to.as_str());

        if let (Some(from), Some(to), Some(fn_), Some(tn)) =
            (from_pos, to_pos, from_node, to_node)
        {
            let waypoints = edge_waypoints
                .get(&(edge.from.clone(), edge.to.clone()))
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            svg.push_str(&render_edge(&mut RenderEdgeContext {
                edge,
                from_node: fn_,
                from,
                to_node: tn,
                to,
                style,
                direction: &flowchart.direction,
                waypoints,
                measure,
                link_stroke: link_style_map.get(&edge_idx).and_then(|s| s.0.as_deref()),
                link_stroke_width: link_style_map.get(&edge_idx).and_then(|s| s.1.as_deref()),
            }));
        }
    }

    // Draw nodes on top
    for node in &flowchart.nodes {
        if let Some(pos) = positions.get(&node.id) {
            let ns = node_style_map.get(&node.id);
            svg.push_str(&render_node(&node.label, &node.shape, pos, style, ns));
        }
    }

    // Draw subgraph titles last (on top of everything) with collision avoidance
    let mut used_title_rects: Vec<Rect> = Vec::new();
    for subgraph in &flowchart.subgraphs {
        svg.push_str(&render_subgraph_title(
            subgraph,
            &positions,
            style,
            &mut used_title_rects,
        ));
    }

    let total_width = bbox.right() + padding;
    let total_height = bbox.bottom() + padding;

    Ok((svg, total_width, total_height))
}

fn render_node(label: &str, shape: &NodeShape, pos: &LayoutPos, style: &DiagramStyle, node_style: Option<&NodeStyle>) -> String {
    let mut svg = String::new();
    let label = label.replace("<br/>", "\n").replace("<br>", "\n").replace("<br />", "\n");
    let escaped_label = escape_xml(&label);

    let fill = node_style.and_then(|ns| ns.fill.as_deref()).unwrap_or(&style.node_fill);
    let stroke = node_style.and_then(|ns| ns.stroke.as_deref()).unwrap_or(&style.node_stroke);
    let stroke_w = node_style.and_then(|ns| ns.stroke_width.as_deref()).unwrap_or("1");
    let text_color = node_style.and_then(|ns| ns.color.as_deref()).unwrap_or(&style.node_text);

    match shape {
        NodeShape::Rect => {
            svg.push_str(&format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x, pos.y, pos.width, pos.height,
                fill, stroke
            ));
        }
        NodeShape::RoundedRect => {
            let rx = 6.0_f32.min(pos.height / 4.0);
            svg.push_str(&format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x, pos.y, pos.width, pos.height, rx,
                fill, stroke
            ));
        }
        NodeShape::Stadium => {
            let rx = pos.height / 2.0;
            svg.push_str(&format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x, pos.y, pos.width, pos.height, rx,
                fill, stroke
            ));
        }
        NodeShape::Subroutine => {
            // Rect with vertical lines at ends
            svg.push_str(&format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x, pos.y, pos.width, pos.height,
                fill, stroke
            ));
            let line_offset = 6.0;
            svg.push_str(&format!(
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x + line_offset, pos.y, pos.x + line_offset, pos.y + pos.height, stroke
            ));
            svg.push_str(&format!(
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x + pos.width - line_offset, pos.y, pos.x + pos.width - line_offset, pos.y + pos.height, stroke
            ));
        }
        NodeShape::Cylinder => {
            let cap_height = 12.0;
            let rx = pos.width / 2.0;
            let bottom_y = pos.y + pos.height - cap_height;
            // Body: left side, bottom arc, right side (filled, no top/bottom strokes)
            svg.push_str(&format!(
                r#"<path d="M {:.2} {:.2} L {:.2} {:.2} A {:.2} {:.2} 0 0 0 {:.2} {:.2} L {:.2} {:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x, pos.y + cap_height,
                pos.x, bottom_y,
                rx, cap_height,
                pos.x + pos.width, bottom_y,
                pos.x + pos.width, pos.y + cap_height,
                fill, stroke
            ));
            // Top ellipse (full, drawn on top of body)
            svg.push_str(&format!(
                r#"<ellipse cx="{:.2}" cy="{:.2}" rx="{:.2}" ry="{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x + pos.width / 2.0, pos.y + cap_height,
                rx, cap_height,
                fill, stroke
            ));
        }
        NodeShape::Circle => {
            let radius = pos.width.min(pos.height) / 2.0;
            let cx = pos.x + pos.width / 2.0;
            let cy = pos.y + pos.height / 2.0;
            svg.push_str(&format!(
                r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                cx, cy, radius, fill, stroke
            ));
        }
        NodeShape::DoubleCircle => {
            let radius = pos.width.min(pos.height) / 2.0 - 4.0;
            let cx = pos.x + pos.width / 2.0;
            let cy = pos.y + pos.height / 2.0;
            svg.push_str(&format!(
                r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                cx, cy, radius + 4.0, fill, stroke
            ));
            svg.push_str(&format!(
                r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                cx, cy, radius, fill, stroke
            ));
        }
        NodeShape::Rhombus => {
            let cx = pos.x + pos.width / 2.0;
            let cy = pos.y + pos.height / 2.0;
            svg.push_str(&format!(
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                cx, pos.y,
                pos.x + pos.width, cy,
                cx, pos.y + pos.height,
                pos.x, cy,
                fill, stroke
            ));
        }
        NodeShape::Hexagon => {
            let offset = 15.0_f32.min(pos.width / 4.0);
            svg.push_str(&format!(
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x + offset, pos.y,
                pos.x + pos.width - offset, pos.y,
                pos.x + pos.width, pos.y + pos.height / 2.0,
                pos.x + pos.width - offset, pos.y + pos.height,
                pos.x + offset, pos.y + pos.height,
                pos.x, pos.y + pos.height / 2.0,
                fill, stroke
            ));
        }
        NodeShape::Parallelogram => {
            let offset = 20.0_f32.min(pos.width / 3.0);
            svg.push_str(&format!(
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x + offset, pos.y,
                pos.x + pos.width, pos.y,
                pos.x + pos.width - offset, pos.y + pos.height,
                pos.x, pos.y + pos.height,
                fill, stroke
            ));
        }
        NodeShape::ParallelogramAlt => {
            let offset = 20.0_f32.min(pos.width / 3.0);
            svg.push_str(&format!(
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x, pos.y,
                pos.x + pos.width - offset, pos.y,
                pos.x + pos.width, pos.y + pos.height,
                pos.x + offset, pos.y + pos.height,
                fill, stroke
            ));
        }
        NodeShape::Trapezoid => {
            let offset = 15.0_f32.min(pos.width / 4.0);
            svg.push_str(&format!(
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x + offset, pos.y,
                pos.x + pos.width - offset, pos.y,
                pos.x + pos.width, pos.y + pos.height,
                pos.x, pos.y + pos.height,
                fill, stroke
            ));
        }
        NodeShape::TrapezoidAlt => {
            let offset = 15.0_f32.min(pos.width / 4.0);
            svg.push_str(&format!(
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" stroke="{}" stroke-width="{stroke_w}" />"#,
                pos.x, pos.y,
                pos.x + pos.width, pos.y,
                pos.x + pos.width - offset, pos.y + pos.height,
                pos.x + offset, pos.y + pos.height,
                fill, stroke
            ));
        }
    }

    // Draw label
    let text_x = pos.x + pos.width / 2.0;
    let text_y = pos.y + pos.height / 2.0;

    // Handle multi-line labels
    let lines: Vec<&str> = escaped_label.lines().collect();
    let line_height = style.font_size * 1.2;
    let total_height = line_height * lines.len() as f32;
    let start_y = text_y - (total_height / 2.0) + line_height / 2.0;

    for (i, line) in lines.iter().enumerate() {
        let y = start_y + i as f32 * line_height;
        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" dy="0.35em" font-family="{}" font-size="{:.1}" font-weight="500" fill="{}" text-anchor="middle">{}</text>"#,
            text_x, y, style.font_family, style.font_size, text_color, line
        ));
    }

    svg
}

/// Clip a point from center of a node to its shape boundary.
fn clip_to_shape(
    node: &super::types::FlowchartNode,
    pos: &LayoutPos,
    target_x: f32,
    target_y: f32,
) -> (f32, f32) {
    let cx = pos.x + pos.width / 2.0;
    let cy = pos.y + pos.height / 2.0;
    let dx = target_x - cx;
    let dy = target_y - cy;
    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return (cx, cy);
    }

    match node.shape {
        NodeShape::Circle | NodeShape::DoubleCircle => {
            let r = pos.width.min(pos.height) / 2.0;
            let dist = (dx * dx + dy * dy).sqrt();
            (cx + dx / dist * r, cy + dy / dist * r)
        }
        NodeShape::Rhombus => {
            let hw = pos.width / 2.0;
            let hh = pos.height / 2.0;
            let t = 1.0 / (dx.abs() / hw + dy.abs() / hh);
            (cx + dx * t, cy + dy * t)
        }
        _ => {
            let hw = pos.width / 2.0;
            let hh = pos.height / 2.0;
            let scale_x = if dx.abs() > 0.001 { hw / dx.abs() } else { f32::MAX };
            let scale_y = if dy.abs() > 0.001 { hh / dy.abs() } else { f32::MAX };
            let scale = scale_x.min(scale_y);
            (cx + dx * scale, cy + dy * scale)
        }
    }
}

struct RenderEdgeContext<'a, T: TextMeasure> {
    edge: &'a super::types::FlowchartEdge,
    from_node: &'a super::types::FlowchartNode,
    from: &'a LayoutPos,
    to_node: &'a super::types::FlowchartNode,
    to: &'a LayoutPos,
    style: &'a DiagramStyle,
    direction: &'a FlowDirection,
    waypoints: &'a [(f32, f32)],
    measure: &'a mut T,
    link_stroke: Option<&'a str>,
    link_stroke_width: Option<&'a str>,
}

fn render_edge<T: TextMeasure>(
    ctx: &mut RenderEdgeContext<'_, T>,
) -> String {
    let edge = ctx.edge;
    let from_node = ctx.from_node;
    let from = ctx.from;
    let to_node = ctx.to_node;
    let to = ctx.to;
    let style = ctx.style;
    let direction = ctx.direction;
    let waypoints = ctx.waypoints;
    let measure = &mut *ctx.measure;
    let link_stroke = ctx.link_stroke;
    let link_stroke_width = ctx.link_stroke_width;

    let edge_stroke_override = link_stroke.unwrap_or(&style.edge_stroke);
    let _edge_width_override = link_stroke_width.unwrap_or("0.75");

    let mut svg = String::new();
    let vertical = matches!(direction, FlowDirection::TopDown | FlowDirection::BottomUp);

    let (dash_attr, stroke_width) = match edge.style {
        EdgeStyle::Solid => ("", 0.75),
        EdgeStyle::Dotted => (" stroke-dasharray=\"4,4\"", 0.75),
        EdgeStyle::Thick => ("", 1.5),
    };

    let from_cx = from.x + from.width / 2.0;
    let from_cy = from.y + from.height / 2.0;
    let to_cx = to.x + to.width / 2.0;
    let to_cy = to.y + to.height / 2.0;

    // Self-loop: render a curved loopback path.
    // Adjust loop direction based on the graph flow direction so the loop
    // doesn't overlap content in the primary flow axis.
    if edge.from == edge.to {
        let loop_radius = 20.0;

        let (exit_x, exit_y, enter_x, enter_y, cp1_x, cp1_y, cp2_x, cp2_y) = match direction {
            FlowDirection::LeftRight | FlowDirection::RightLeft => {
                // Horizontal graph: loop upward
                let exit_x = from_cx;
                let exit_y = from.y;
                let enter_x = from_cx;
                let enter_y = from.y;
                let cp1_x = exit_x + loop_radius;
                let cp1_y = exit_y - loop_radius * 1.5;
                let cp2_x = enter_x - loop_radius;
                let cp2_y = enter_y - loop_radius * 1.5;
                (exit_x, exit_y, enter_x, enter_y, cp1_x, cp1_y, cp2_x, cp2_y)
            }
            // TopDown, BottomUp: loop to the right
            _ => {
                let exit_x = from.x + from.width;
                let exit_y = from_cy;
                let enter_x = from_cx;
                let enter_y = from.y;
                let cp1_x = exit_x + loop_radius;
                let cp1_y = exit_y - loop_radius;
                let cp2_x = enter_x + loop_radius;
                let cp2_y = enter_y - loop_radius;
                (exit_x, exit_y, enter_x, enter_y, cp1_x, cp1_y, cp2_x, cp2_y)
            }
        };

        svg.push_str(&format!(
            r#"<path d="M {:.2},{:.2} C {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="none" stroke="{}" stroke-width="{:.2}"{} />"#,
            exit_x, exit_y,
            cp1_x, cp1_y,
            cp2_x, cp2_y,
            enter_x, enter_y,
            edge_stroke_override, stroke_width, dash_attr
        ));

        // Arrowhead pointing down into the top of the node
        if edge.arrow_head != ArrowType::None {
            svg.push_str(&render_arrow_head(
                enter_x,
                enter_y,
                -std::f32::consts::FRAC_PI_2,
                &edge.arrow_head,
                style,
            ));
        }

        // Edge label
        if let Some(ref label) = edge.label {
            let escaped = escape_xml(label);
            let (label_w, _) = measure.measure_text(
                label,
                style.font_size * 0.82,
                false,
                false,
                false,
                None,
            );
            let label_x = exit_x + loop_radius - label_w / 2.0;
            let label_y = from.y - loop_radius;
            svg.push_str(&format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="4" fill="{}" stroke="{}" stroke-width="0.5" />"#,
                label_x - 2.0,
                label_y - style.font_size * 0.7,
                label_w + 4.0,
                style.font_size * 1.1,
                style.background,
                style.node_stroke
            ));
            svg.push_str(&format!(
                r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="start">{}</text>"#,
                label_x, label_y, style.font_family, style.font_size * 0.82, style.edge_text, escaped
            ));
        }

        return svg;
    }

    // Determine exit and entry ports based on flow direction and relative position
    let (exit_x, exit_y, enter_x, enter_y) = if vertical {
        let going_down = from_cy <= to_cy;
        let ey = if going_down { from.bottom() } else { from.y };
        let ny = if going_down { to.y } else { to.bottom() };
        // Handle same-rank (same y) case: route horizontally
        if (from_cy - to_cy).abs() < 1.0 {
            let going_right = from_cx < to_cx;
            let ex = if going_right { from.right() } else { from.x };
            let nx = if going_right { to.x } else { to.right() };
            (ex, from_cy, nx, to_cy)
        } else {
            (from_cx, ey, to_cx, ny)
        }
    } else {
        let going_right = from_cx <= to_cx;
        let ex = if going_right { from.right() } else { from.x };
        let nx = if going_right { to.x } else { to.right() };
        // Handle same-column case: route vertically
        if (from_cx - to_cx).abs() < 1.0 {
            let going_down = from_cy < to_cy;
            let ey = if going_down { from.bottom() } else { from.y };
            let ny = if going_down { to.y } else { to.bottom() };
            (from_cx, ey, to_cx, ny)
        } else {
            (ex, from_cy, nx, to_cy)
        }
    };

    // Clip exit/enter points to actual shape boundaries
    let (exit_x, exit_y) = clip_to_shape(from_node, from, exit_x, exit_y);
    let (enter_x, enter_y) = clip_to_shape(to_node, to, enter_x, enter_y);

    // Build polyline points through waypoints
    let mut all_x = vec![exit_x];
    let mut all_y = vec![exit_y];
    for &(wx, wy) in waypoints {
        all_x.push(wx);
        all_y.push(wy);
    }
    all_x.push(enter_x);
    all_y.push(enter_y);

    let mut points: Vec<(f32, f32)> = Vec::new();
    points.push((all_x[0], all_y[0]));

    for i in 0..all_x.len() - 1 {
        let x1 = all_x[i];
        let y1 = all_y[i];
        let x2 = all_x[i + 1];
        let y2 = all_y[i + 1];

        if vertical || ((from_cy - to_cy).abs() < 1.0 && waypoints.is_empty()) {
            // Vertical primary axis (or horizontal same-rank): dogleg with vertical-first
            if (x1 - x2).abs() > 0.5 {
                let mid_y = (y1 + y2) / 2.0;
                points.push((x1, mid_y));
                points.push((x2, mid_y));
            }
        } else {
            // Horizontal primary axis: dogleg with horizontal-first
            if (y1 - y2).abs() > 0.5 {
                let mid_x = (x1 + x2) / 2.0;
                points.push((mid_x, y1));
                points.push((mid_x, y2));
            }
        }
        points.push((x2, y2));
    }

    // Render polyline
    let points_str: String = points
        .iter()
        .map(|(x, y)| format!("{:.2},{:.2}", x, y))
        .collect::<Vec<_>>()
        .join(" ");
    svg.push_str(&format!(
        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="{:.2}" stroke-linecap="round" stroke-linejoin="round"{} />"#,
        points_str, edge_stroke_override, stroke_width, dash_attr
    ));

    // Arrow head aligned to final segment
    let head_angle = if points.len() >= 2 {
        let last = points[points.len() - 1];
        let prev = points[points.len() - 2];
        (last.1 - prev.1).atan2(last.0 - prev.0)
    } else {
        (enter_y - exit_y).atan2(enter_x - exit_x)
    };

    if edge.arrow_head != ArrowType::None {
        svg.push_str(&render_arrow_head(enter_x, enter_y, head_angle, &edge.arrow_head, style));
    }

    // Arrow tail
    if edge.arrow_tail != ArrowType::None {
        let tail_angle = if points.len() >= 2 {
            let first = points[0];
            let second = points[1];
            (first.1 - second.1).atan2(first.0 - second.0)
        } else {
            head_angle + std::f32::consts::PI
        };
        svg.push_str(&render_arrow_head(exit_x, exit_y, tail_angle, &edge.arrow_tail, style));
    }

    // Label on middle segment
    let mid = points.len() / 2;
    let label_x = if points.len() >= 2 {
        (points[mid.saturating_sub(1)].0 + points[mid.min(points.len() - 1)].0) / 2.0
    } else {
        (exit_x + enter_x) / 2.0
    };
    let label_y = if points.len() >= 2 {
        (points[mid.saturating_sub(1)].1 + points[mid.min(points.len() - 1)].1) / 2.0
    } else {
        (exit_y + enter_y) / 2.0
    };

    if let Some(ref label) = edge.label {
        let cleaned = crate::xml::sanitize_xml_text(label);
        let label_font_size = style.font_size * 0.85;
        let text_w = measure
            .measure_text(&cleaned, label_font_size, false, false, false, None)
            .0;
        let pill_pad = 8.0;
        let pill_w = (text_w + pill_pad * 2.0).max(label_font_size * 2.5);
        let pill_h = label_font_size + pill_pad * 2.0;

        svg.push_str(&format!(
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="4" fill="{}" stroke="{}" stroke-width="0.5" />"#,
            label_x - pill_w / 2.0,
            label_y - pill_h / 2.0,
            pill_w,
            pill_h,
            style.background,
            style.node_stroke
        ));

        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" dy="0.35em" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="middle">{}</text>"#,
            label_x,
            label_y,
            style.font_family,
            label_font_size,
            style.edge_text,
            escape_xml(&cleaned)
        ));
    }

    svg
}

fn render_arrow_head(
    x: f32,
    y: f32,
    angle: f32,
    arrow_type: &ArrowType,
    style: &DiagramStyle,
) -> String {
    let cos = angle.cos();
    let sin = angle.sin();

    match arrow_type {
        ArrowType::Arrow => {
            let p1 = (x - cos * 8.0 + sin * 4.8, y - sin * 8.0 - cos * 4.8);
            let p2 = (x - cos * 8.0 - sin * 4.8, y - sin * 8.0 + cos * 4.8);
            format!(
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" />"#,
                x, y, p1.0, p1.1, p2.0, p2.1, style.edge_stroke
            )
        }
        ArrowType::Circle => {
            format!(
                r#"<circle cx="{:.2}" cy="{:.2}" r="5" fill="{}" stroke="{}" stroke-width="1" />"#,
                x - cos * 5.0,
                y - sin * 5.0,
                style.node_fill,
                style.edge_stroke
            )
        }
        ArrowType::Cross => {
            let s = 7.0_f32;
            let cx = x - cos * s;
            let cy = y - sin * s;
            // Rotate ±45° from the edge direction for an "×" shape
            let angle_a = angle + std::f32::consts::FRAC_PI_4;
            let angle_b = angle - std::f32::consts::FRAC_PI_4;
            let (ca, sa) = (angle_a.cos(), angle_a.sin());
            let (cb, sb) = (angle_b.cos(), angle_b.sin());
            format!(
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="2.5" /><line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="2.5" />"#,
                cx + ca * s,
                cy + sa * s,
                cx - ca * s,
                cy - sa * s,
                style.edge_stroke,
                cx + cb * s,
                cy + sb * s,
                cx - cb * s,
                cy - sb * s,
                style.edge_stroke
            )
        }
        ArrowType::None => String::new(),
    }
}

fn subgraph_bbox(
    subgraph: &super::types::Subgraph,
    positions: &HashMap<String, LayoutPos>,
) -> Option<(f32, f32, f32, f32)> {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_right = f32::MIN;
    let mut max_bottom = f32::MIN;
    let mut found = false;

    for node_id in &subgraph.nodes {
        if let Some(pos) = positions.get(node_id) {
            found = true;
            min_x = min_x.min(pos.x);
            min_y = min_y.min(pos.y);
            max_right = max_right.max(pos.right());
            max_bottom = max_bottom.max(pos.bottom());
        }
    }

    if !found {
        return None;
    }

    let content_bbox = BBox::new(min_x, min_y, max_right - min_x, max_bottom - min_y);
    let padded_bbox = content_bbox.with_padding(20.0);
    let raw_y = padded_bbox.y - 20.0;
    let clamped_y = raw_y.max(0.0);
    Some((
        padded_bbox.x,
        clamped_y,
        padded_bbox.width,
        padded_bbox.height + 20.0 + (raw_y - clamped_y).abs(),
    ))
}

fn render_subgraph_box(
    subgraph: &super::types::Subgraph,
    positions: &HashMap<String, LayoutPos>,
    style: &DiagramStyle,
) -> String {
    let Some((min_x, min_y, width, height)) = subgraph_bbox(subgraph, positions) else {
        return String::new();
    };

    format!(
        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="8" fill="{}" fill-opacity="0.3" stroke="{}" stroke-width="1" stroke-dasharray="4,2" />"#,
        min_x, min_y, width, height,
        style.node_fill, style.node_stroke
    )
}

fn render_subgraph_title(
    subgraph: &super::types::Subgraph,
    positions: &HashMap<String, LayoutPos>,
    style: &DiagramStyle,
    used_rects: &mut Vec<Rect>,
) -> String {
    if subgraph.title.is_empty() {
        return String::new();
    }

    let Some((min_x, min_y, _width, _height)) = subgraph_bbox(subgraph, positions) else {
        return String::new();
    };

    let title_font = style.font_size * 0.9;
    let approx_char_w = title_font * 0.6;
    let title_w = subgraph.title.len() as f32 * approx_char_w + 12.0;
    let title_h = title_font + 6.0;

    let title_x = min_x + 12.0;
    let mut title_y = min_y + title_font + 4.0;

    // Offset title if it would overlap a previously placed title
    for used in used_rects.iter() {
        let candidate = Rect::new(title_x - 4.0, title_y - title_h / 2.0 - 1.0, title_w, title_h);
        if candidate.overlaps(used) {
            title_y = used.y + used.h + title_h / 2.0 + 3.0;
        }
    }

    let pill_x = title_x - 4.0;
    let pill_y = title_y - title_h / 2.0 - 1.0;
    used_rects.push(Rect::new(pill_x, pill_y, title_w, title_h));

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="3" fill="{}" fill-opacity="0.9" />"#,
        pill_x, pill_y, title_w, title_h, style.node_fill
    ));
    svg.push_str(&format!(
        r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" font-weight="bold" text-anchor="start">{}</text>"#,
        title_x, title_y, style.font_family, title_font, style.node_text, escape_xml(&subgraph.title)
    ));

    svg
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

    fn first_polygon_points(svg: &str) -> Vec<(f32, f32)> {
        let marker = "<polygon points=\"";
        let start = svg.find(marker).expect("expected polygon");
        let rest = &svg[start + marker.len()..];
        let end = rest.find('"').expect("expected polygon points close quote");
        rest[..end]
            .split_whitespace()
            .map(|pair| {
                let mut it = pair.split(',');
                let x = it
                    .next()
                    .expect("x")
                    .parse::<f32>()
                    .expect("x should parse");
                let y = it
                    .next()
                    .expect("y")
                    .parse::<f32>()
                    .expect("y should parse");
                (x, y)
            })
            .collect()
    }

    #[test]
    fn orthogonal_edge_arrow_points_in_correct_direction() {
        let mut measure = MockMeasure;
        let style = DiagramStyle::default();
        let edge = super::super::types::FlowchartEdge {
            from: "A".to_string(),
            to: "B".to_string(),
            label: None,
            style: EdgeStyle::Solid,
            arrow_head: ArrowType::Arrow,
            arrow_tail: ArrowType::None,
            min_length: 1,
        };
        let from_node = super::super::types::FlowchartNode {
            id: "A".to_string(),
            label: "A".to_string(),
            shape: NodeShape::Rect,
        };
        let to_node = super::super::types::FlowchartNode {
            id: "B".to_string(),
            label: "B".to_string(),
            shape: NodeShape::Rect,
        };
        let from = LayoutPos::new(0.0, 0.0, 100.0, 40.0);
        let to = LayoutPos::new(200.0, 100.0, 100.0, 40.0);

        let svg = render_edge(&mut RenderEdgeContext {
            edge: &edge,
            from_node: &from_node,
            from: &from,
            to_node: &to_node,
            to: &to,
            style: &style,
            direction: &FlowDirection::LeftRight,
            waypoints: &[],
            measure: &mut measure,
        });

        let pts = first_polygon_points(&svg);
        assert_eq!(pts.len(), 3);

        // Orthogonal routing: exit right of from=(100,20), enter left of to=(200,120)
        // Arrow tip at entry point (200, 120), pointing right (angle=0)
        assert!((pts[0].0 - 200.0).abs() < 1.0, "tip x={}", pts[0].0);
        assert!((pts[0].1 - 120.0).abs() < 1.0, "tip y={}", pts[0].1);
    }

    #[test]
    fn test_self_loop_renders_visible_curve() {
        let source = "flowchart LR\n    A[Node A] --> A";
        let diagram = match crate::parse_mermaid(source).unwrap() {
            crate::MermaidDiagram::Flowchart(fc) => fc,
            _ => panic!("Expected flowchart"),
        };
        let style = DiagramStyle::default();
        let mut measure = MockMeasure;
        let (svg, _w, _h) = render_flowchart(&diagram, &style, &mut measure).unwrap();

        assert!(
            svg.contains("<path"),
            "Self-loop should render as a curve (path element), got:\n{}",
            svg
        );
    }

    #[test]
    fn test_self_loop_has_arrowhead() {
        let source = "flowchart TD\n    A --> A";
        let diagram = match crate::parse_mermaid(source).unwrap() {
            crate::MermaidDiagram::Flowchart(fc) => fc,
            _ => panic!("Expected flowchart"),
        };
        let style = DiagramStyle::default();
        let mut measure = MockMeasure;
        let (svg, _w, _h) = render_flowchart(&diagram, &style, &mut measure).unwrap();

        assert!(
            svg.contains("<polygon"),
            "Self-loop should have an arrowhead"
        );
    }
}
