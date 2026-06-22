use std::collections::{HashMap, HashSet, VecDeque};

use crate::TextMeasure;

use super::types::*;

/// Bounding box for layout elements
#[derive(Debug, Clone, Copy, Default)]
pub struct BBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl BBox {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn with_padding(&self, padding: f32) -> Self {
        Self::new(
            self.x - padding,
            self.y - padding,
            self.width + padding * 2.0,
            self.height + padding * 2.0,
        )
    }
}

/// Layout position for a node
#[derive(Debug, Clone, Copy)]
pub struct LayoutPos {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LayoutPos {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
}

/// Intermediate waypoints for edges routed through dummy nodes.
pub type EdgeWaypoints = HashMap<(String, String), Vec<(f32, f32)>>;

/// Layout engine for diagrams
pub struct LayoutEngine<'a, T: TextMeasure> {
    measure: &'a mut T,
    font_size: f32,
    pub node_spacing_x: f32,
    pub node_spacing_y: f32,
    pub edge_label_padding: f32,
    pub node_padding_h: f32,
    pub node_padding_v: f32,
}

impl<'a, T: TextMeasure> LayoutEngine<'a, T> {
    pub fn new(measure: &'a mut T, font_size: f32) -> Self {
        Self {
            measure,
            font_size,
            // Slightly larger default spacing improves readability in dense docs.
            node_spacing_x: 64.0,
            node_spacing_y: 72.0,
            edge_label_padding: 8.0,
            node_padding_h: 18.0,
            node_padding_v: 12.0,
        }
    }

    fn measure_text_width(
        &mut self,
        text: &str,
        font_size: f32,
        is_code: bool,
        bold: bool,
        italic: bool,
    ) -> f32 {
        let cleaned = crate::xml::sanitize_xml_text(text);
        self.measure
            .measure_text(&cleaned, font_size, is_code, bold, italic, None)
            .0
    }

    fn measure_multiline(
        &mut self,
        text: &str,
        font_size: f32,
        is_code: bool,
        bold: bool,
        italic: bool,
    ) -> (f32, usize) {
        let mut max_width: f32 = 0.0;
        let mut lines = 0;

        for line in text.lines() {
            lines += 1;
            let width = self.measure_text_width(line, font_size, is_code, bold, italic);
            max_width = max_width.max(width);
        }

        if lines == 0 {
            lines = 1;
            max_width = self.measure_text_width(text, font_size, is_code, bold, italic);
        }

        (max_width, lines)
    }

    /// Layout a flowchart diagram using layered layout with barycenter ordering.
    pub fn layout_flowchart(
        &mut self,
        flowchart: &Flowchart,
    ) -> (HashMap<String, LayoutPos>, EdgeWaypoints, BBox) {
        let positions: HashMap<String, LayoutPos> = HashMap::new();

        if flowchart.nodes.is_empty() {
            return (positions, HashMap::new(), BBox::default());
        }

        let nodes: Vec<String> = flowchart.nodes.iter().map(|n| n.id.clone()).collect();
        let edges: Vec<(String, String, usize)> = flowchart
            .edges
            .iter()
            .map(|e| (e.from.clone(), e.to.clone(), e.min_length.max(1)))
            .collect();
        let mut node_sizes: HashMap<String, (f32, f32)> = HashMap::new();
        for node in &flowchart.nodes {
            node_sizes.insert(
                node.id.clone(),
                self.calculate_flowchart_node_size(&node.label, &node.shape),
            );
        }

        self.layout_layered_graph(&nodes, &edges, &node_sizes, flowchart.direction)
    }

    fn calculate_flowchart_node_size(&mut self, label: &str, shape: &NodeShape) -> (f32, f32) {
        let line_height = self.font_size * 1.2;
        let (text_width, lines) =
            self.measure_multiline(label, self.font_size, false, false, false);
        let pad_w = self.node_padding_h * 2.0;
        let pad_h = self.node_padding_v * 2.0;

        let text_h = line_height * lines as f32;
        let mut width = (text_width + pad_w).max(56.0);
        let mut height = (text_h + pad_h).max(36.0);

        match shape {
            NodeShape::Circle => {
                let size = width.max(height);
                width = size;
                height = size;
            }
            NodeShape::DoubleCircle => {
                let size = width.max(height) + 8.0;
                width = size;
                height = size;
            }
            NodeShape::Rhombus => {
                width += 26.0;
                height += 16.0;
            }
            NodeShape::Hexagon => {
                width += 24.0;
            }
            NodeShape::Parallelogram | NodeShape::ParallelogramAlt => {
                width += 20.0;
            }
            NodeShape::Trapezoid | NodeShape::TrapezoidAlt => {
                width += 16.0;
            }
            NodeShape::Stadium => {
                height = height.max(40.0);
                width = width.max(height + 20.0);
            }
            NodeShape::Cylinder => {
                height += 24.0;
            }
            NodeShape::Subroutine => {
                width += 16.0;
            }
            NodeShape::Rect | NodeShape::RoundedRect => {}
        }

        (width, height)
    }

    /// Layout a sequence diagram with measured actor and label widths.
    pub fn layout_sequence(
        &mut self,
        diagram: &SequenceDiagram,
    ) -> (HashMap<String, LayoutPos>, BBox) {
        let mut positions: HashMap<String, LayoutPos> = HashMap::new();

        if diagram.participants.is_empty() {
            return (positions, BBox::default());
        }

        let participant_height = (self.font_size * 2.4).max(36.0);
        let start_x = 40.0;
        let start_y = 20.0;

        let mut widths: Vec<f32> = Vec::with_capacity(diagram.participants.len());
        for participant in &diagram.participants {
            let label = participant.alias.as_ref().unwrap_or(&participant.id);
            let label_w = self.measure_text_width(label, self.font_size, false, false, false);
            widths.push((label_w + 28.0).max(96.0));
        }

        let mut centers: Vec<f32> = Vec::with_capacity(diagram.participants.len());
        centers.push(start_x + widths[0] / 2.0);
        for i in 1..diagram.participants.len() {
            let prev = centers[i - 1];
            // Sequence diagrams get cramped quickly; prefer a wider default gap.
            let base_gap = ((widths[i - 1] + widths[i]) / 2.0 + 72.0).max(140.0);
            centers.push(prev + base_gap);
        }

        let mut index_by_id: HashMap<&str, usize> = HashMap::new();
        for (idx, participant) in diagram.participants.iter().enumerate() {
            index_by_id.insert(participant.id.as_str(), idx);
        }

        let mut pair_requirements: Vec<(usize, usize, f32)> = Vec::new();
        self.collect_sequence_pair_requirements(
            &diagram.elements,
            &index_by_id,
            &mut pair_requirements,
        );

        for _ in 0..3 {
            let mut changed = false;
            for (a, b, required) in &pair_requirements {
                if *a >= *b {
                    continue;
                }
                let distance = centers[*b] - centers[*a];
                if distance + 0.5 < *required {
                    let delta = *required - distance;
                    for center in centers.iter_mut().skip(*b) {
                        *center += delta;
                    }
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        for (i, participant) in diagram.participants.iter().enumerate() {
            let width = widths[i];
            let x = centers[i] - width / 2.0;
            positions.insert(
                participant.id.clone(),
                LayoutPos::new(x, start_y, width, participant_height),
            );
        }

        let mut current_y = start_y + participant_height + 40.0;
        let mut max_label_half_width = 0.0;
        self.measure_sequence_metrics(&diagram.elements, &mut current_y, &mut max_label_half_width);

        if diagram.elements.is_empty() {
            current_y += 40.0;
        }

        let left = positions
            .values()
            .map(|p| p.x)
            .fold(f32::MAX, f32::min)
            .min(start_x);
        let right = positions
            .values()
            .map(|p| p.right())
            .fold(0.0, f32::max)
            .max(start_x + 120.0);

        let bbox = BBox::new(
            left,
            0.0,
            (right - left) + max_label_half_width * 2.0,
            current_y + 20.0,
        )
        .with_padding(self.edge_label_padding / 2.0);

        (positions, bbox)
    }

    fn collect_sequence_pair_requirements(
        &mut self,
        elements: &[SequenceElement],
        index_by_id: &HashMap<&str, usize>,
        out: &mut Vec<(usize, usize, f32)>,
    ) {
        for element in elements {
            match element {
                SequenceElement::Message(msg) => {
                    if let (Some(&from), Some(&to)) = (
                        index_by_id.get(msg.from.as_str()),
                        index_by_id.get(msg.to.as_str()),
                    )
                        && from != to {
                            let a = from.min(to);
                            let b = from.max(to);
                            let label_w = self.measure_text_width(
                                &msg.label,
                                self.font_size * 0.85,
                                false,
                                false,
                                false,
                            );
                            let total_req = (label_w + 42.0).max(120.0);
                            out.push((a, b, total_req));

                            // For messages spanning multiple participants, push
                            // intermediate pairs apart so labels have room.
                            if b - a > 1 {
                                let pill_w = label_w + 20.0;
                                let per_pair = (pill_w / (b - a - 1).max(1) as f32 + 40.0).max(140.0);
                                for i in a..b {
                                    out.push((i, i + 1, per_pair));
                                }
                            }
                        }
                }
                SequenceElement::Block(block) => {
                    self.collect_sequence_pair_requirements(&block.messages, index_by_id, out);
                    for (_, branch_elements) in &block.else_branches {
                        self.collect_sequence_pair_requirements(branch_elements, index_by_id, out);
                    }
                }
                SequenceElement::Activation(_)
                | SequenceElement::Deactivation(_)
                | SequenceElement::Note { .. } => {}
            }
        }
    }

    fn measure_sequence_metrics(
        &mut self,
        elements: &[SequenceElement],
        current_y: &mut f32,
        max_label_half_width: &mut f32,
    ) {
        for element in elements {
            match element {
                SequenceElement::Message(msg) => {
                    let label_w = self.measure_text_width(
                        &msg.label,
                        self.font_size * 0.85,
                        false,
                        false,
                        false,
                    );
                    *max_label_half_width =
                        (*max_label_half_width).max(label_w / 2.0 + self.edge_label_padding);
                    *current_y += 50.0;
                }
                SequenceElement::Activation(_) | SequenceElement::Deactivation(_) => {
                    *current_y += 24.0;
                }
                SequenceElement::Note { text, .. } => {
                    let _ =
                        self.measure_text_width(text, self.font_size * 0.8, false, false, false);
                    *current_y += 42.0;
                }
                SequenceElement::Block(block) => {
                    // 6 (pre-padding) + 22 (title + post-gap)
                    *current_y += 34.0;
                    self.measure_sequence_metrics(&block.messages, current_y, max_label_half_width);
                    for (_, branch_elements) in &block.else_branches {
                        *current_y += 30.0;
                        self.measure_sequence_metrics(
                            branch_elements,
                            current_y,
                            max_label_half_width,
                        );
                    }
                    *current_y += 20.0;
                }
            }
        }
    }

    /// Layout a class diagram.
    pub fn layout_class(&mut self, diagram: &ClassDiagram) -> (HashMap<String, LayoutPos>, EdgeWaypoints, BBox) {
        let positions: HashMap<String, LayoutPos> = HashMap::new();

        if diagram.classes.is_empty() {
            return (positions, HashMap::new(), BBox::default());
        }

        let mut node_sizes: HashMap<String, (f32, f32)> = HashMap::new();
        for class in &diagram.classes {
            node_sizes.insert(class.name.clone(), self.calculate_class_size(class));
        }

        let nodes: Vec<String> = diagram.classes.iter().map(|c| c.name.clone()).collect();
        let edges: Vec<(String, String, usize)> = diagram
            .relations
            .iter()
            .map(|rel| (rel.from.clone(), rel.to.clone(), 1))
            .collect();

        if edges.is_empty() {
            return self.layout_grid(&nodes, &node_sizes, 40.0, 40.0, 140.0, 110.0);
        }

        self.layout_layered_graph(&nodes, &edges, &node_sizes, FlowDirection::TopDown)
    }

    fn calculate_class_size(&mut self, class: &ClassDefinition) -> (f32, f32) {
        let header_font = self.font_size;
        let member_font = self.font_size * 0.85;
        let line_h = member_font * 1.2;

        // Measure header including stereotype prefix (e.g. "<<interface>> ClassName")
        let header_text = if class.is_interface {
            let stereo = class.stereotype.as_deref().unwrap_or("interface");
            format!("<<{}>> {}", stereo, class.name)
        } else if let Some(ref stereo) = class.stereotype {
            format!("<<{}>> {}", stereo, class.name)
        } else {
            class.name.clone()
        };
        let mut max_width = self
            .measure_text_width(&header_text, header_font, false, true, class.is_abstract)
            .max(120.0);

        let mut attr_lines = 0usize;
        for attr in &class.attributes {
            let vis = match attr.member.visibility {
                Visibility::Public => "+",
                Visibility::Private => "-",
                Visibility::Protected => "#",
                Visibility::Package => "~",
            };
            let text = if let Some(ref t) = attr.type_annotation {
                format!("{} {}: {}", vis, attr.member.name, t)
            } else {
                format!("{} {}", vis, attr.member.name)
            };
            max_width = max_width.max(self.measure_text_width(
                &text,
                member_font,
                true,
                false,
                attr.member.is_abstract,
            ));
            attr_lines += 1;
        }

        let mut method_lines = 0usize;
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
            let text = if let Some(ref ret) = method.return_type {
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
            max_width = max_width.max(self.measure_text_width(
                &text,
                member_font,
                true,
                false,
                method.member.is_abstract,
            ));
            method_lines += 1;
        }

        let width = (max_width + self.node_padding_h * 2.0).max(180.0);

        let mut height = self.font_size + 16.0;
        // The render always advances by font_size + 4.0 before the first attribute,
        // so account for that even when there are no attributes.
        height += self.font_size + 4.0;
        height += attr_lines as f32 * line_h;
        if method_lines > 0 {
            // Divider gap + first method offset + method lines
            height += 4.0 + self.font_size + 2.0;
            height += method_lines as f32 * line_h;
        }
        // Breathing room for descenders in the last member row.
        height += 14.0;
        height = height.max(64.0);

        (width, height)
    }

    /// Layout a state diagram.
    pub fn layout_state(&mut self, diagram: &StateDiagram) -> (HashMap<String, LayoutPos>, EdgeWaypoints, BBox) {
        let positions: HashMap<String, LayoutPos> = HashMap::new();

        if diagram.states.is_empty() {
            return (positions, HashMap::new(), BBox::default());
        }

        let child_state_ids: HashSet<&str> = diagram
            .states
            .iter()
            .flat_map(|state| state.children.iter())
            .filter_map(|child| match child {
                StateElement::State(s) => Some(s.id.as_str()),
                _ => None,
            })
            .collect();

        let top_level_states: Vec<&State> = diagram
            .states
            .iter()
            .filter(|state| !child_state_ids.contains(state.id.as_str()))
            .collect();

        let target_states: Vec<&State> = if top_level_states.is_empty() {
            diagram.states.iter().collect()
        } else {
            top_level_states
        };

        let nodes: Vec<String> = target_states.iter().map(|s| s.id.clone()).collect();
        let node_id_set: HashSet<&str> = nodes.iter().map(String::as_str).collect();
        let mut node_sizes: HashMap<String, (f32, f32)> = HashMap::new();
        for state in &target_states {
            node_sizes.insert(state.id.clone(), self.calculate_state_size(state));
        }

        let edges: Vec<(String, String, usize)> = diagram
            .transitions
            .iter()
            .filter(|t| {
                node_id_set.contains(t.from.as_str()) && node_id_set.contains(t.to.as_str())
            })
            .map(|t| (t.from.clone(), t.to.clone(), 1))
            .collect();

        if edges.is_empty() {
            return self.layout_grid(&nodes, &node_sizes, 40.0, 40.0, 120.0, 95.0);
        }

        self.layout_layered_graph(&nodes, &edges, &node_sizes, FlowDirection::TopDown)
    }

    fn calculate_state_size(&mut self, state: &State) -> (f32, f32) {
        if state.is_start || state.is_end {
            return (24.0, 24.0);
        }

        let label_w = self.measure_text_width(&state.label, self.font_size, false, false, false);
        let base_width = (label_w + self.node_padding_h * 2.0).max(120.0);
        let base_height = (self.font_size * 2.2).max(40.0);

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
            .map(|s| self.calculate_state_size(s))
            .collect();

        let child_gap = 20.0;
        let inner_pad = 16.0;
        let route_lane = 40.0; // routing lanes on each side for transitions
        let header_h = self.font_size * 2.0 + 16.0;

        let max_child_w: f32 = child_sizes.iter().map(|(w, _)| *w).fold(0.0, f32::max);
        let total_child_h: f32 = child_sizes.iter().map(|(_, h)| *h).sum::<f32>()
            + child_gap * (child_sizes.len().saturating_sub(1)) as f32;

        let width = base_width.max(max_child_w + inner_pad * 2.0 + route_lane * 2.0);
        let height = header_h + total_child_h + inner_pad * 2.0;

        (width, height)
    }

    /// Layout an ER diagram.
    pub fn layout_er(&mut self, diagram: &ErDiagram) -> (HashMap<String, LayoutPos>, EdgeWaypoints, BBox) {
        let positions: HashMap<String, LayoutPos> = HashMap::new();

        if diagram.entities.is_empty() {
            return (positions, HashMap::new(), BBox::default());
        }

        let mut seen = std::collections::HashSet::new();
        let nodes: Vec<String> = diagram
            .entities
            .iter()
            .filter(|e| seen.insert(e.name.clone()))
            .map(|e| e.name.clone())
            .collect();
        let edges: Vec<(String, String, usize)> = diagram
            .relationships
            .iter()
            .map(|r| (r.from.clone(), r.to.clone(), 1))
            .collect();

        let mut node_sizes: HashMap<String, (f32, f32)> = HashMap::new();
        for entity in &diagram.entities {
            node_sizes.insert(entity.name.clone(), self.calculate_er_size(entity));
        }

        if edges.is_empty() {
            return self.layout_grid(&nodes, &node_sizes, 40.0, 40.0, 180.0, 140.0);
        }

        self.layout_layered_graph(&nodes, &edges, &node_sizes, FlowDirection::LeftRight)
    }

    fn calculate_er_size(&mut self, entity: &ErEntity) -> (f32, f32) {
        let title_font = self.font_size;
        let attr_font = self.font_size * 0.85;

        let mut max_w = self.measure_text_width(&entity.name, title_font, false, true, false);
        for attr in &entity.attributes {
            let marker = if attr.is_key { "[" } else { "" };
            let attr_name = if attr.is_key {
                format!("{}{}]", marker, attr.name)
            } else {
                attr.name.clone()
            };
            max_w = max_w.max(self.measure_text_width(&attr_name, attr_font, false, false, false));
        }

        let width = (max_w + self.node_padding_h * 2.0).max(150.0);
        let divider_space = if entity.attributes.is_empty() { 0.0 } else { self.font_size * 0.5 + 4.0 };
        let height = (34.0 + divider_space + entity.attributes.len() as f32 * (self.font_size * 1.3) + 10.0).max(56.0);
        (width, height)
    }

    fn layout_grid(
        &self,
        nodes: &[String],
        node_sizes: &HashMap<String, (f32, f32)>,
        start_x: f32,
        start_y: f32,
        spacing_x: f32,
        spacing_y: f32,
    ) -> (HashMap<String, LayoutPos>, EdgeWaypoints, BBox) {
        let mut positions: HashMap<String, LayoutPos> = HashMap::new();
        if nodes.is_empty() {
            return (positions, HashMap::new(), BBox::default());
        }

        let cols = (nodes.len() as f32).sqrt().ceil() as usize;
        let cols = cols.max(1);

        let mut row_heights: Vec<f32> = vec![0.0; nodes.len().div_ceil(cols)];
        for (idx, node_id) in nodes.iter().enumerate() {
            let row = idx / cols;
            let (_, h) = node_sizes.get(node_id).copied().unwrap_or((120.0, 40.0));
            row_heights[row] = row_heights[row].max(h);
        }

        for (idx, node_id) in nodes.iter().enumerate() {
            let col = idx % cols;
            let row = idx / cols;
            let (w, h) = node_sizes.get(node_id).copied().unwrap_or((120.0, 40.0));
            let y = start_y + row_heights[..row].iter().sum::<f32>() + row as f32 * spacing_y;
            let x = start_x + col as f32 * (w + spacing_x);
            positions.insert(node_id.clone(), LayoutPos::new(x, y, w, h));
        }

        let bbox = Self::calculate_bbox(&positions);
        (positions, HashMap::new(), bbox)
    }

    fn layout_layered_graph(
        &self,
        nodes: &[String],
        edges: &[(String, String, usize)],
        node_sizes: &HashMap<String, (f32, f32)>,
        direction: FlowDirection,
    ) -> (HashMap<String, LayoutPos>, EdgeWaypoints, BBox) {
        let mut positions: HashMap<String, LayoutPos> = HashMap::new();
        if nodes.is_empty() {
            return (positions, HashMap::new(), BBox::default());
        }

        let order_index: HashMap<&str, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();

        let mut incoming_init: HashMap<&str, Vec<(&str, usize)>> = HashMap::new();
        let mut outgoing_init: HashMap<&str, Vec<(&str, usize)>> = HashMap::new();

        for node in nodes {
            incoming_init.entry(node.as_str()).or_default();
            outgoing_init.entry(node.as_str()).or_default();
        }

        for (from, to, min_len) in edges {
            if !order_index.contains_key(from.as_str()) || !order_index.contains_key(to.as_str()) {
                continue;
            }
            outgoing_init
                .entry(from.as_str())
                .or_default()
                .push((to.as_str(), *min_len));
            incoming_init
                .entry(to.as_str())
                .or_default()
                .push((from.as_str(), *min_len));
        }

        let mut ranks: HashMap<&str, usize> = HashMap::new();
        let roots: Vec<&str> = nodes
            .iter()
            .map(String::as_str)
            .filter(|n| incoming_init.get(n).is_none_or(|parents| parents.is_empty()))
            .collect();

        let mut queue: VecDeque<&str> = VecDeque::new();
        if roots.is_empty() {
            if let Some(first) = nodes.first() {
                ranks.insert(first.as_str(), 0);
                queue.push_back(first.as_str());
            }
        } else {
            for root in roots {
                ranks.insert(root, 0);
                queue.push_back(root);
            }
        }

        while let Some(node) = queue.pop_front() {
            let rank = *ranks.get(node).unwrap_or(&0);
            if let Some(neighbors) = outgoing_init.get(node) {
                for &(neighbor, min_len) in neighbors {
                    if !ranks.contains_key(neighbor) {
                        ranks.insert(neighbor, rank + min_len.max(1));
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        let mut max_rank = ranks.values().copied().max().unwrap_or(0);
        for node in nodes {
            if !ranks.contains_key(node.as_str()) {
                max_rank += 1;
                ranks.insert(node.as_str(), max_rank);
                queue.push_back(node.as_str());

                while let Some(cur) = queue.pop_front() {
                    let rank = *ranks.get(cur).unwrap_or(&0);
                    if let Some(neighbors) = outgoing_init.get(cur) {
                        for &(neighbor, min_len) in neighbors {
                            if !ranks.contains_key(neighbor) {
                                ranks.insert(neighbor, rank + min_len.max(1));
                                queue.push_back(neighbor);
                            }
                        }
                    }
                }
            }
        }

        // --- Phase 2: Insert dummy nodes for long edges ---
        let mut all_node_ids: Vec<String> = nodes.to_vec();
        let mut all_sizes: HashMap<String, (f32, f32)> = node_sizes.clone();
        let mut ranks_owned: HashMap<String, usize> = ranks.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let mut dummy_set: HashSet<String> = HashSet::new();
        let mut edge_dummy_chains: HashMap<(String, String), Vec<String>> = HashMap::new();
        let mut augmented_edges: Vec<(String, String, usize)> = Vec::new();

        for (from, to, min_len) in edges {
            if !order_index.contains_key(from.as_str()) || !order_index.contains_key(to.as_str()) {
                augmented_edges.push((from.clone(), to.clone(), *min_len));
                continue;
            }
            let from_rank = ranks.get(from.as_str()).copied().unwrap_or(0);
            let to_rank = ranks.get(to.as_str()).copied().unwrap_or(0);

            if to_rank > from_rank + 1 {
                let mut chain: Vec<String> = Vec::new();
                let mut prev = from.clone();
                for rank in (from_rank + 1)..to_rank {
                    let dummy_id = format!("__d_{}_{}_{}", from, to, rank);
                    all_node_ids.push(dummy_id.clone());
                    all_sizes.insert(dummy_id.clone(), (0.0, 0.0));
                    ranks_owned.insert(dummy_id.clone(), rank);
                    dummy_set.insert(dummy_id.clone());
                    chain.push(dummy_id.clone());
                    augmented_edges.push((prev, dummy_id.clone(), 1));
                    prev = dummy_id;
                }
                augmented_edges.push((prev, to.clone(), 1));
                edge_dummy_chains.insert((from.clone(), to.clone()), chain);
            } else {
                augmented_edges.push((from.clone(), to.clone(), *min_len));
            }
        }

        // Rebuild data structures with dummies
        let max_rank = ranks_owned.values().copied().max().unwrap_or(0);
        let aug_order: HashMap<&str, usize> = all_node_ids
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();

        let mut incoming: HashMap<&str, Vec<(&str, usize)>> = HashMap::new();
        let mut outgoing: HashMap<&str, Vec<(&str, usize)>> = HashMap::new();

        for node in &all_node_ids {
            incoming.entry(node.as_str()).or_default();
            outgoing.entry(node.as_str()).or_default();
        }

        for (from, to, min_len) in &augmented_edges {
            if aug_order.contains_key(from.as_str()) && aug_order.contains_key(to.as_str()) {
                outgoing.entry(from.as_str()).or_default().push((to.as_str(), *min_len));
                incoming.entry(to.as_str()).or_default().push((from.as_str(), *min_len));
            }
        }

        let mut rank_nodes: Vec<Vec<&str>> = vec![Vec::new(); max_rank + 1];
        for node in &all_node_ids {
            let rank = ranks_owned.get(node.as_str()).copied().unwrap_or(0);
            if rank < rank_nodes.len() {
                rank_nodes[rank].push(node.as_str());
            }
        }

        for rank in &mut rank_nodes {
            rank.sort_by_key(|id| aug_order.get(id).copied().unwrap_or(usize::MAX));
        }

        for _ in 0..6 {
            for rank_idx in 1..rank_nodes.len() {
                let prev_rank_pos: HashMap<&str, usize> = rank_nodes[rank_idx - 1]
                    .iter()
                    .enumerate()
                    .map(|(i, id)| (*id, i))
                    .collect();

                rank_nodes[rank_idx].sort_by(|a, b| {
                    let bc_a = incoming
                        .get(a)
                        .and_then(|parents| barycenter(parents, &prev_rank_pos));
                    let bc_b = incoming
                        .get(b)
                        .and_then(|parents| barycenter(parents, &prev_rank_pos));

                    match (bc_a, bc_b) {
                        (Some(x), Some(y)) => x
                            .partial_cmp(&y)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| {
                                aug_order
                                    .get(a)
                                    .copied()
                                    .unwrap_or(usize::MAX)
                                    .cmp(&aug_order.get(b).copied().unwrap_or(usize::MAX))
                            }),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => aug_order
                            .get(a)
                            .copied()
                            .unwrap_or(usize::MAX)
                            .cmp(&aug_order.get(b).copied().unwrap_or(usize::MAX)),
                    }
                });
            }

            for rank_idx in (0..rank_nodes.len().saturating_sub(1)).rev() {
                let next_rank_pos: HashMap<&str, usize> = rank_nodes[rank_idx + 1]
                    .iter()
                    .enumerate()
                    .map(|(i, id)| (*id, i))
                    .collect();

                rank_nodes[rank_idx].sort_by(|a, b| {
                    let bc_a = outgoing
                        .get(a)
                        .and_then(|children| barycenter(children, &next_rank_pos));
                    let bc_b = outgoing
                        .get(b)
                        .and_then(|children| barycenter(children, &next_rank_pos));

                    match (bc_a, bc_b) {
                        (Some(x), Some(y)) => x
                            .partial_cmp(&y)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| {
                                aug_order
                                    .get(a)
                                    .copied()
                                    .unwrap_or(usize::MAX)
                                    .cmp(&aug_order.get(b).copied().unwrap_or(usize::MAX))
                            }),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => aug_order
                            .get(a)
                            .copied()
                            .unwrap_or(usize::MAX)
                            .cmp(&aug_order.get(b).copied().unwrap_or(usize::MAX)),
                    }
                });
            }
        }

        let vertical = matches!(direction, FlowDirection::TopDown | FlowDirection::BottomUp);
        let base_x = 30.0;
        let base_y = 30.0;

        if vertical {
            let rank_widths: Vec<f32> = rank_nodes
                .iter()
                .map(|rank| {
                    if rank.is_empty() {
                        0.0
                    } else {
                        rank.iter()
                            .map(|id| all_sizes.get(*id).copied().unwrap_or((100.0, 40.0)).0)
                            .sum::<f32>()
                            + self.node_spacing_x * rank.len().saturating_sub(1) as f32
                    }
                })
                .collect();

            let max_rank_w = rank_widths.iter().copied().fold(0.0, f32::max);
            let mut y = base_y;

            for (rank_idx, rank) in rank_nodes.iter().enumerate() {
                let mut x = base_x + (max_rank_w - rank_widths[rank_idx]).max(0.0) / 2.0;
                let mut rank_max_h: f32 = 0.0;

                for node_id in rank {
                    let (w, h) = all_sizes.get(*node_id).copied().unwrap_or((100.0, 40.0));
                    positions.insert((*node_id).to_string(), LayoutPos::new(x, y, w, h));
                    x += w + self.node_spacing_x;
                    rank_max_h = rank_max_h.max(h);
                }

                y += rank_max_h + self.node_spacing_y;
            }

            // Phase 3: Coordinate refinement
            for _ in 0..4 {
                // Forward pass
                for rank in rank_nodes.iter().skip(1) {
                    for node_id in rank {
                        if let Some(neighbors) = incoming.get(node_id) {
                            let mut centers: Vec<f32> = neighbors
                                .iter()
                                .filter_map(|(n, _)| positions.get(*n))
                                .map(|p| p.x + p.width / 2.0)
                                .collect();
                            if !centers.is_empty() {
                                centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                                let median = centers[centers.len() / 2];
                                let w = all_sizes.get(*node_id).map(|s| s.0).unwrap_or(100.0);
                                if let Some(pos) = positions.get_mut(*node_id) {
                                    pos.x = median - w / 2.0;
                                }
                            }
                        }
                    }
                    // Enforce minimum spacing
                    let mut prev_right = f32::NEG_INFINITY;
                    for node_id in rank {
                        if let Some(pos) = positions.get_mut(*node_id) {
                            pos.x = pos.x.max(prev_right + self.node_spacing_x);
                            prev_right = pos.x + pos.width;
                        }
                    }
                }

                // Backward pass
                for rank_idx in (0..rank_nodes.len().saturating_sub(1)).rev() {
                    for node_id in &rank_nodes[rank_idx] {
                        if let Some(neighbors) = outgoing.get(node_id) {
                            let mut centers: Vec<f32> = neighbors
                                .iter()
                                .filter_map(|(n, _)| positions.get(*n))
                                .map(|p| p.x + p.width / 2.0)
                                .collect();
                            if !centers.is_empty() {
                                centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                                let median = centers[centers.len() / 2];
                                let w = all_sizes.get(*node_id).map(|s| s.0).unwrap_or(100.0);
                                if let Some(pos) = positions.get_mut(*node_id) {
                                    pos.x = median - w / 2.0;
                                }
                            }
                        }
                    }
                    let mut prev_right = f32::NEG_INFINITY;
                    for node_id in &rank_nodes[rank_idx] {
                        if let Some(pos) = positions.get_mut(*node_id) {
                            pos.x = pos.x.max(prev_right + self.node_spacing_x);
                            prev_right = pos.x + pos.width;
                        }
                    }
                }
            }
        } else {
            let rank_heights: Vec<f32> = rank_nodes
                .iter()
                .map(|rank| {
                    if rank.is_empty() {
                        0.0
                    } else {
                        rank.iter()
                            .map(|id| all_sizes.get(*id).copied().unwrap_or((100.0, 40.0)).1)
                            .sum::<f32>()
                            + self.node_spacing_y * rank.len().saturating_sub(1) as f32
                    }
                })
                .collect();

            let max_rank_h = rank_heights.iter().copied().fold(0.0, f32::max);
            let mut x = base_x;

            for (rank_idx, rank) in rank_nodes.iter().enumerate() {
                let mut y = base_y + (max_rank_h - rank_heights[rank_idx]).max(0.0) / 2.0;
                let mut rank_max_w: f32 = 0.0;

                for node_id in rank {
                    let (w, h) = all_sizes.get(*node_id).copied().unwrap_or((100.0, 40.0));
                    positions.insert((*node_id).to_string(), LayoutPos::new(x, y, w, h));
                    y += h + self.node_spacing_y;
                    rank_max_w = rank_max_w.max(w);
                }

                x += rank_max_w + self.node_spacing_x;
            }

            // Phase 3: Coordinate refinement (horizontal)
            for _ in 0..4 {
                // Forward pass
                for rank in rank_nodes.iter().skip(1) {
                    for node_id in rank {
                        if let Some(neighbors) = incoming.get(node_id) {
                            let mut centers: Vec<f32> = neighbors
                                .iter()
                                .filter_map(|(n, _)| positions.get(*n))
                                .map(|p| p.y + p.height / 2.0)
                                .collect();
                            if !centers.is_empty() {
                                centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                                let median = centers[centers.len() / 2];
                                let h = all_sizes.get(*node_id).map(|s| s.1).unwrap_or(40.0);
                                if let Some(pos) = positions.get_mut(*node_id) {
                                    pos.y = median - h / 2.0;
                                }
                            }
                        }
                    }
                    // Enforce minimum spacing
                    let mut prev_bottom = f32::NEG_INFINITY;
                    for node_id in rank {
                        if let Some(pos) = positions.get_mut(*node_id) {
                            pos.y = pos.y.max(prev_bottom + self.node_spacing_y);
                            prev_bottom = pos.y + pos.height;
                        }
                    }
                }

                // Backward pass
                for rank_idx in (0..rank_nodes.len().saturating_sub(1)).rev() {
                    for node_id in &rank_nodes[rank_idx] {
                        if let Some(neighbors) = outgoing.get(node_id) {
                            let mut centers: Vec<f32> = neighbors
                                .iter()
                                .filter_map(|(n, _)| positions.get(*n))
                                .map(|p| p.y + p.height / 2.0)
                                .collect();
                            if !centers.is_empty() {
                                centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                                let median = centers[centers.len() / 2];
                                let h = all_sizes.get(*node_id).map(|s| s.1).unwrap_or(40.0);
                                if let Some(pos) = positions.get_mut(*node_id) {
                                    pos.y = median - h / 2.0;
                                }
                            }
                        }
                    }
                    let mut prev_bottom = f32::NEG_INFINITY;
                    for node_id in &rank_nodes[rank_idx] {
                        if let Some(pos) = positions.get_mut(*node_id) {
                            pos.y = pos.y.max(prev_bottom + self.node_spacing_y);
                            prev_bottom = pos.y + pos.height;
                        }
                    }
                }
            }
        }

        // Extract waypoints from dummy positions
        let mut edge_waypoints: EdgeWaypoints = HashMap::new();
        for ((from, to), chain) in &edge_dummy_chains {
            let waypoints: Vec<(f32, f32)> = chain
                .iter()
                .filter_map(|dummy_id| positions.get(dummy_id))
                .map(|pos| pos.center())
                .collect();
            if !waypoints.is_empty() {
                edge_waypoints.insert((from.clone(), to.clone()), waypoints);
            }
        }

        // Remove dummy positions
        for dummy_id in &dummy_set {
            positions.remove(dummy_id);
        }

        // Normalize positions so diagram starts near origin
        Self::normalize_positions(&mut positions, &mut edge_waypoints, base_x, base_y);

        // Direction flip
        let mut bbox = Self::calculate_bbox(&positions);

        if matches!(direction, FlowDirection::BottomUp) {
            let bottom = bbox.bottom();
            for pos in positions.values_mut() {
                pos.y = bottom - (pos.y + pos.height);
            }
            for wps in edge_waypoints.values_mut() {
                for wp in wps.iter_mut() {
                    wp.1 = bottom - wp.1;
                }
            }
            bbox = Self::calculate_bbox(&positions);
        } else if matches!(direction, FlowDirection::RightLeft) {
            let right = bbox.right();
            for pos in positions.values_mut() {
                pos.x = right - (pos.x + pos.width);
            }
            for wps in edge_waypoints.values_mut() {
                for wp in wps.iter_mut() {
                    wp.0 = right - wp.0;
                }
            }
            bbox = Self::calculate_bbox(&positions);
        }

        (positions, edge_waypoints, bbox)
    }

    fn normalize_positions(
        positions: &mut HashMap<String, LayoutPos>,
        edge_waypoints: &mut EdgeWaypoints,
        target_x: f32,
        target_y: f32,
    ) {
        if positions.is_empty() {
            return;
        }
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        for pos in positions.values() {
            min_x = min_x.min(pos.x);
            min_y = min_y.min(pos.y);
        }
        let dx = target_x - min_x;
        let dy = target_y - min_y;
        if dx.abs() < 0.01 && dy.abs() < 0.01 {
            return;
        }
        for pos in positions.values_mut() {
            pos.x += dx;
            pos.y += dy;
        }
        for wps in edge_waypoints.values_mut() {
            for wp in wps.iter_mut() {
                wp.0 += dx;
                wp.1 += dy;
            }
        }
    }

    fn calculate_bbox(positions: &HashMap<String, LayoutPos>) -> BBox {
        if positions.is_empty() {
            return BBox::default();
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for pos in positions.values() {
            min_x = min_x.min(pos.x);
            min_y = min_y.min(pos.y);
            max_x = max_x.max(pos.right());
            max_y = max_y.max(pos.bottom());
        }

        BBox::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

fn barycenter(neighbors: &[(&str, usize)], rank_pos: &HashMap<&str, usize>) -> Option<f32> {
    let mut total = 0.0;
    let mut count = 0.0;
    for (neighbor, _) in neighbors {
        if let Some(pos) = rank_pos.get(neighbor) {
            total += *pos as f32;
            count += 1.0;
        }
    }

    if count > 0.0 {
        Some(total / count)
    } else {
        None
    }
}
