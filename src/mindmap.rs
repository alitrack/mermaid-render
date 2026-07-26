//! Mindmap diagram: parser, layout, and SVG renderer.
//!
//! Syntax:
//! ```mermaid
//! mindmap
//!   root((Central))
//!     Topic A
//!       Detail A1
//!     Topic B
//!       Detail B1
//! ```

use std::collections::HashMap;

use crate::layout::{BBox, LayoutPos};
use crate::TextMeasure;

// ── Types ──

#[derive(Debug, Clone)]
pub struct MindmapNode {
    pub id: String,
    pub label: String,
    /// Node shape: ((circle)), [rect], (rounded), ))cloud((
    pub shape: MindmapShape,
    pub children: Vec<MindmapNode>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MindmapShape {
    Circle,
    RoundedRect,
    Rect,
    Cloud,
    Hexagon,
}

impl MindmapShape {
    fn from_shape_str(s: &str) -> Self {
        match s {
            "circle" | "((...))" => Self::Circle,
            "rounded" | "(...)" => Self::RoundedRect,
            "rect" | "[...]" => Self::Rect,
            "cloud" | "))...((" => Self::Cloud,
            "hexagon" | "{{...}}" => Self::Hexagon,
            _ => Self::RoundedRect,
        }
    }
}

// ── Parser ──

pub fn parse_mindmap(input: &str) -> Result<MindmapNode, String> {
    let lines: Vec<&str> = input.lines().skip(1).collect(); // skip "mindmap"
    if lines.is_empty() {
        return Err("mindmap: empty input".to_string());
    }

    // Build a stack of (indent, node) for tree construction
    let _node_id = 0usize;
    let _root: Option<MindmapNode> = None;
    let _stack: Vec<(usize, usize)> = Vec::new(); // (indent, node_index_in_flat_list)

    // First pass: parse all nodes into a flat list with indentation
    struct RawNode {
        indent: usize,
        label: String,
        shape: MindmapShape,
        parent_idx: Option<usize>,
        flat_idx: usize,
    }
    let mut raw_nodes: Vec<RawNode> = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }

        let indent = line.len() - line.trim_start().len();
        let (label, shape) = parse_mindmap_node_label(trimmed);

        let parent_idx = raw_nodes.iter().rev()
            .find(|n| n.indent < indent)
            .map(|n| n.flat_idx);

        raw_nodes.push(RawNode {
            indent,
            label,
            shape,
            parent_idx,
            flat_idx: raw_nodes.len(),
        });
    }

    if raw_nodes.is_empty() {
        return Err("mindmap: no nodes found".to_string());
    }

    // Second pass: build tree from flat list
    // Collect children for each parent
    let n = raw_nodes.len();
    let mut children_map: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut root_idx = 0usize;

    for node in &raw_nodes {
        if let Some(pidx) = node.parent_idx {
            children_map[pidx].push(node.flat_idx);
        } else {
            root_idx = node.flat_idx;
        }
    }

    fn build_node(idx: usize, raw: &[RawNode], children_map: &[Vec<usize>]) -> MindmapNode {
        let r = &raw[idx];
        let children: Vec<MindmapNode> = children_map[idx].iter()
            .map(|&ci| build_node(ci, raw, children_map))
            .collect();
        MindmapNode {
            id: format!("mm-{idx}"),
            label: r.label.clone(),
            shape: r.shape,
            children,
        }
    }

    Ok(build_node(root_idx, &raw_nodes, &children_map))
}

fn parse_mindmap_node_label(s: &str) -> (String, MindmapShape) {
    // root((label))
    if s.starts_with("root((") && s.ends_with("))") {
        let inner = &s[6..s.len()-2];
        return (inner.to_string(), MindmapShape::Circle);
    }
    if s.starts_with("root[") && s.ends_with(']') {
        let inner = &s[5..s.len()-1];
        return (inner.to_string(), MindmapShape::Rect);
    }
    if s.starts_with("root(") && s.ends_with(')') {
        let inner = &s[5..s.len()-1];
        return (inner.to_string(), MindmapShape::RoundedRect);
    }
    // ((label))
    if s.starts_with("((") && s.ends_with("))") {
        return (s[2..s.len()-2].to_string(), MindmapShape::Circle);
    }
    // ))label((
    if s.starts_with("))") && s.ends_with("((") {
        return (s[2..s.len()-2].to_string(), MindmapShape::Cloud);
    }
    // {{label}}
    if s.starts_with("{{") && s.ends_with("}}") {
        return (s[2..s.len()-2].to_string(), MindmapShape::Hexagon);
    }
    // [label]
    if s.starts_with('[') && s.ends_with(']') {
        return (s[1..s.len()-1].to_string(), MindmapShape::Rect);
    }
    // (label)
    if s.starts_with('(') && s.ends_with(')') {
        return (s[1..s.len()-1].to_string(), MindmapShape::RoundedRect);
    }
    // Plain text
    (s.to_string(), MindmapShape::RoundedRect)
}

// ── Layout ──

struct TreeLayout {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    children: Vec<TreeLayout>,
}

fn layout_tree(node: &MindmapNode, measure: &mut impl TextMeasure, font_size: f32, node_pad: f32) -> TreeLayout {
    let (w, h) = node_size(node, measure, font_size, node_pad);

    let child_layouts: Vec<TreeLayout> = node.children.iter()
        .map(|c| layout_tree(c, measure, font_size, node_pad))
        .collect();

    // Stack children vertically
    let mut total_child_h = 0.0f32;
    let mut max_child_w = 0.0f32;
    for cl in &child_layouts {
        total_child_h += cl.height + 12.0;
        max_child_w = max_child_w.max(cl.width);
    }
    if !child_layouts.is_empty() {
        total_child_h -= 12.0; // remove trailing gap
    }

    // Height is max of own height and children total
    let height = h.max(total_child_h);
    let width = w + if child_layouts.is_empty() { 0.0 } else { max_child_w + 60.0 };

    TreeLayout {
        x: 0.0, y: 0.0, // set during position assignment
        width,
        height,
        children: child_layouts,
    }
}

fn assign_positions(layout: &mut TreeLayout, x: f32, y: f32) {
    layout.x = x;
    layout.y = y;

    let node_h = layout.height;
    let mut child_y = y + (node_h - layout.children.iter().map(|c| c.height).sum::<f32>()
        - (layout.children.len().saturating_sub(1)) as f32 * 12.0) / 2.0;

    // Clamp child_y to not go above parent top
    child_y = child_y.max(y - 4.0);

    for child in &mut layout.children {
        let child_x = x + 60.0 + 80.0; // gap between parent and child
        assign_positions(child, child_x, child_y);
        child_y += child.height + 12.0;
    }
}

fn collect_positions(layout: &TreeLayout, id_prefix: &str, positions: &mut HashMap<String, LayoutPos>, id_counter: &mut usize) {
    let id = format!("{id_prefix}-{}", *id_counter);
    *id_counter += 1;
    positions.insert(id, LayoutPos::new(layout.x, layout.y, layout.width.min(120.0), layout.height.min(36.0)));

    for child in &layout.children {
        collect_positions(child, id_prefix, positions, id_counter);
    }
}

fn node_size(node: &MindmapNode, measure: &mut impl TextMeasure, font_size: f32, pad: f32) -> (f32, f32) {
    let w = measure.measure_text(&node.label, font_size, false, false, false, None).0 + pad * 2.0;
    let h = font_size * 1.6 + pad * 2.0;
    (w.max(60.0), h.max(28.0))
}

pub fn layout_mindmap(root: &MindmapNode, measure: &mut impl TextMeasure, font_size: f32) -> (HashMap<String, LayoutPos>, BBox) {
    let node_pad = 10.0;
    let mut tree = layout_tree(root, measure, font_size, node_pad);
    assign_positions(&mut tree, 40.0, 40.0);

    let mut positions = HashMap::new();
    let mut id_counter = 0usize;
    collect_positions(&tree, "mm", &mut positions, &mut id_counter);

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

pub fn render_mindmap(root: &MindmapNode, style: &crate::render::DiagramStyle, measure: &mut impl TextMeasure) -> Result<(String, f32, f32), String> {
    use crate::render::escape_xml;

    let (positions, bbox) = layout_mindmap(root, measure, style.font_size);
    if positions.is_empty() {
        return Ok(("<g></g>".to_string(), 100.0, 50.0));
    }

    let mut svg = String::new();
    let _node_pad = 10.0;

    // Collect flat list of nodes with positions for edge drawing
    let _node_list: Vec<(String, LayoutPos)> = positions.iter()
        .map(|(id, pos)| (id.clone(), *pos))
        .collect();
    let pos_map: HashMap<&str, LayoutPos> = positions.iter().map(|(k, v)| (k.as_str(), *v)).collect();

    // Build id-to-label map
    let mut label_map: HashMap<String, (String, MindmapShape)> = HashMap::new();
    let _id_counter = 0usize;
    fn collect_labels(node: &MindmapNode, id_prefix: &str, map: &mut HashMap<String, (String, MindmapShape)>, counter: &mut usize) {
        let id = format!("{id_prefix}-{}", *counter);
        *counter += 1;
        map.insert(id, (node.label.clone(), node.shape));
        for child in &node.children {
            collect_labels(child, id_prefix, map, counter);
        }
    }
    collect_labels(root, "mm", &mut label_map, &mut 0usize);

    // Build parent-child edge list from tree structure
    let mut edges: Vec<(String, String)> = Vec::new(); // (parent_id, child_id)
    let _edge_counter = 0usize;
    fn collect_edges(node: &MindmapNode, id_prefix: &str, edges: &mut Vec<(String, String)>, counter: &mut usize) {
        let parent_id = format!("{id_prefix}-{}", *counter);
        *counter += 1;
        for _child in &node.children {
            let child_id = format!("{id_prefix}-{}", *counter);
            edges.push((parent_id.clone(), child_id));
            // Recurse — but we already incremented counter in the loop above
            // Actually, need different approach since collect_labels uses separate pass
        }
    }
    // Simpler: just iterate positions in order, connecting parent to children
    let _flat_idx = 0usize;
    fn collect_edges_flat(node: &MindmapNode, id_prefix: &str, edges: &mut Vec<(String, String)>, counter: &mut usize) {
        let pid = format!("{id_prefix}-{}", *counter);
        *counter += 1;
        for child in &node.children {
            let cid = format!("{id_prefix}-{}", *counter);
            edges.push((pid.clone(), cid.clone()));
            collect_edges_flat(child, id_prefix, edges, counter);
        }
    }
    collect_edges_flat(root, "mm", &mut edges, &mut 0usize);

    // Draw edges
    for (pid, cid) in &edges {
        if let (Some(pp), Some(cp)) = (pos_map.get(pid.as_str()), pos_map.get(cid.as_str())) {
            let px = pp.x + pp.width;
            let py = pp.y + pp.height / 2.0;
            let cx = cp.x;
            let cy = cp.y + cp.height / 2.0;
            // Curved path
            let mid_x = (px + cx) / 2.0;
            svg.push_str(&format!(
                r#"<path d="M {:.2},{:.2} C {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="none" stroke="{}" stroke-width="1.2" />"#,
                px, py, mid_x, py, mid_x, cy, cx, cy, style.edge_stroke
            ));
        }
    }

    // Draw nodes in sorted order (parent before children)
    let mut sorted_ids: Vec<String> = positions.keys().cloned().collect();
    sorted_ids.sort();

    for id in &sorted_ids {
        if let Some(pos) = positions.get(id) {
            if let Some((label, shape)) = label_map.get(id) {
                let nx = pos.x;
                let ny = pos.y;
                let nw = pos.width;
                let nh = pos.height;

                match shape {
                    MindmapShape::Circle => {
                        let cx = nx + nw / 2.0; let cy = ny + nh / 2.0; let r = nw.min(nh) / 2.0;
                        svg.push_str(&format!(
                            r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}" stroke="{}" stroke-width="1.5" />"#,
                            cx, cy, r, style.node_fill, style.node_stroke
                        ));
                    }
                    MindmapShape::Rect => {
                        svg.push_str(&format!(
                            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="{}" stroke-width="1.5" />"#,
                            nx, ny, nw, nh, style.node_fill, style.node_stroke
                        ));
                    }
                    MindmapShape::Cloud => {
                        // Ellipse as cloud approximation
                        let cx = nx + nw / 2.0; let cy = ny + nh / 2.0;
                        svg.push_str(&format!(
                            r#"<ellipse cx="{:.2}" cy="{:.2}" rx="{:.2}" ry="{:.2}" fill="{}" stroke="{}" stroke-width="1.5" />"#,
                            cx, cy, nw/2.0, nh/2.0, style.node_fill, style.node_stroke
                        ));
                    }
                    MindmapShape::Hexagon => {
                        let offset = 8.0;
                        svg.push_str(&format!(
                            r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" stroke="{}" stroke-width="1.5" />"#,
                            nx+offset, ny, nx+nw-offset, ny, nx+nw, ny+nh/2.0, nx+nw-offset, ny+nh, nx+offset, ny+nh, nx, ny+nh/2.0,
                            style.node_fill, style.node_stroke
                        ));
                    }
                    _ => { // RoundedRect
                        let rx = 6.0;
                        svg.push_str(&format!(
                            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" fill="{}" stroke="{}" stroke-width="1.5" />"#,
                            nx, ny, nw, nh, rx, style.node_fill, style.node_stroke
                        ));
                    }
                }

                svg.push_str(&format!(
                    r#"<text x="{:.2}" y="{:.2}" dy="0.35em" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="middle">{}</text>"#,
                    nx + nw / 2.0, ny + nh / 2.0, style.font_family, style.font_size, style.node_text,
                    escape_xml(label)
                ));
            }
        }
    }

    let total_w = bbox.width + 40.0;
    let total_h = bbox.height + 40.0;
    Ok((svg, total_w, total_h))
}
