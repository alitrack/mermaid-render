//! Layout engine using dagre-rs.
//! dagre-rs is a complete Rust port of dagre.js (v0.1.1) — the same layout
//! library that mermaid.js uses internally. 528 tests, 100% pass rate.

use std::collections::{HashMap, HashSet};

use dagre::graph::{Graph, GraphOptions};
use dagre::{layout, LayoutOptions, RankDir, NodeLabel, EdgeLabel};

use crate::TextMeasure;
use super::types::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct BBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl BBox {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self { Self { x, y, width, height } }
    pub fn right(&self) -> f32 { self.x + self.width }
    pub fn bottom(&self) -> f32 { self.y + self.height }
    pub fn with_padding(&self, p: f32) -> Self {
        Self::new(self.x - p, self.y - p, self.width + p * 2.0, self.height + p * 2.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutPos {
    pub x: f32, pub y: f32, pub width: f32, pub height: f32,
}
impl LayoutPos {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self { Self { x, y, width: w, height: h } }
    pub fn center(&self) -> (f32, f32) { (self.x + self.width / 2.0, self.y + self.height / 2.0) }
    pub fn right(&self) -> f32 { self.x + self.width }
    pub fn bottom(&self) -> f32 { self.y + self.height }
}

pub type EdgeWaypoints = HashMap<(String, String), Vec<(f32, f32)>>;

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
        Self { measure, font_size, node_spacing_x: 64.0, node_spacing_y: 72.0, edge_label_padding: 8.0, node_padding_h: 18.0, node_padding_v: 12.0 }
    }

    fn text_w(&mut self, text: &str, fs: f32, code: bool, bold: bool, italic: bool) -> f32 {
        self.measure.measure_text(&crate::xml::sanitize_xml_text(text), fs, code, bold, italic, None).0
    }

    fn text_wh(&mut self, text: &str, fs: f32, code: bool, bold: bool, italic: bool) -> (f32, usize) {
        let mut mw: f32 = 0.0; let mut lc = 0;
        for line in text.lines() { lc += 1; mw = mw.max(self.text_w(line, fs, code, bold, italic)); }
        if lc == 0 { lc = 1; mw = self.text_w(text, fs, code, bold, italic); }
        (mw, lc)
    }

    fn node_size(&mut self, label: &str, shape: &NodeShape) -> (f64, f64) {
        let lh = self.font_size as f64 * 1.2;
        let (tw, lc) = self.text_wh(label, self.font_size, false, false, false);
        let pad_w = self.node_padding_h as f64 * 2.0;
        let pad_h = self.node_padding_v as f64 * 2.0;
        let th = lh * lc as f64;
        let mut w = (tw as f64 + pad_w).max(56.0);
        let mut h = (th + pad_h).max(36.0);
        match shape {
            NodeShape::Circle | NodeShape::DoubleCircle => { let s = w.max(h); w = s; h = s; }
            NodeShape::Rhombus => { w += 26.0; h += 16.0; }
            NodeShape::Hexagon => { w += 24.0; }
            NodeShape::Parallelogram | NodeShape::ParallelogramAlt => { w += 20.0; }
            NodeShape::Trapezoid | NodeShape::TrapezoidAlt => { w += 16.0; }
            NodeShape::Stadium => { h = h.max(40.0); w = w.max(h + 20.0); }
            NodeShape::Cylinder => { h += 24.0; }
            NodeShape::Subroutine => { w += 16.0; }
            _ => {}
        }
        (w, h)
    }

    pub fn layout_flowchart(&mut self, fc: &Flowchart) -> (HashMap<String, LayoutPos>, EdgeWaypoints, BBox) {
        if fc.nodes.is_empty() { return (HashMap::new(), HashMap::new(), BBox::default()); }
        let rankdir = match fc.direction {
            FlowDirection::TopDown => RankDir::TB, FlowDirection::BottomUp => RankDir::BT,
            FlowDirection::LeftRight => RankDir::LR, FlowDirection::RightLeft => RankDir::RL,
        };
        let mut g = Graph::with_options(GraphOptions { directed: true, multigraph: true, compound: true });
        for node in &fc.nodes {
            let (w, h) = self.node_size(&node.label, &node.shape);
            g.set_node(node.id.clone(), Some(NodeLabel { width: w, height: h, ..Default::default() }));
        }
        for edge in &fc.edges {
            let mut el = EdgeLabel::default();
            if let Some(ref label) = edge.label {
                el.width = self.text_w(label, self.font_size * 0.82, false, false, false) as f64;
                el.height = (self.font_size * 1.1) as f64;
            }
            g.set_edge(edge.from.clone(), edge.to.clone(), Some(el), None);
        }
        layout(&mut g, Some(LayoutOptions {
            rankdir, nodesep: self.node_spacing_x as f64, ranksep: self.node_spacing_y as f64,
            edgesep: 15.0, marginx: 20.0, marginy: 20.0, tie_keep_first: true, ..Default::default()
        }));
        let edge_keys: Vec<(String, String)> = fc.edges.iter().map(|e| (e.from.clone(), e.to.clone())).collect();
        self.extract(&g, &fc.nodes.iter().map(|n| n.id.clone()).collect::<Vec<_>>(), &edge_keys)
    }

    pub fn layout_class(&mut self, d: &ClassDiagram) -> (HashMap<String, LayoutPos>, EdgeWaypoints, BBox) {
        if d.classes.is_empty() { return (HashMap::new(), HashMap::new(), BBox::default()); }
        if d.relations.is_empty() {
            let ids: Vec<_> = d.classes.iter().map(|c| c.name.clone()).collect();
            let sizes: HashMap<_, _> = d.classes.iter().map(|c| { let (w, h) = self.class_size(c); (c.name.clone(), (w as f32, h as f32)) }).collect();
            return self.simple_grid(&ids, &sizes, 40.0, 40.0, 180.0, 110.0);
        }
        let mut g = Graph::with_options(GraphOptions { directed: true, multigraph: true, compound: false });
        for c in &d.classes {
            let (w, h) = self.class_size(c);
            g.set_node(c.name.clone(), Some(NodeLabel { width: w, height: h, ..Default::default() }));
        }
        for r in &d.relations { g.set_edge(r.from.clone(), r.to.clone(), Some(EdgeLabel::default()), None); }
        layout(&mut g, Some(LayoutOptions {
            rankdir: RankDir::TB, nodesep: self.node_spacing_x as f64, ranksep: self.node_spacing_y as f64,
            marginx: 30.0, marginy: 30.0, tie_keep_first: true, ..Default::default()
        }));
        let ids: Vec<_> = d.classes.iter().map(|c| c.name.clone()).collect();
        let edge_keys: Vec<_> = d.relations.iter().map(|r| (r.from.clone(), r.to.clone())).collect();
        self.extract(&g, &ids, &edge_keys)
    }

    pub fn layout_state(&mut self, d: &StateDiagram) -> (HashMap<String, LayoutPos>, EdgeWaypoints, BBox) {
        if d.states.is_empty() { return (HashMap::new(), HashMap::new(), BBox::default()); }
        let child_ids: HashSet<&str> = d.states.iter().flat_map(|s| s.children.iter())
            .filter_map(|c| match c { StateElement::State(s) => Some(s.id.as_str()), _ => None }).collect();
        let target: Vec<&State> = d.states.iter().filter(|s| !child_ids.contains(s.id.as_str())).collect();
        let target = if target.is_empty() { d.states.iter().collect() } else { target };
        let ids: Vec<_> = target.iter().map(|s| s.id.clone()).collect();
        let id_set: HashSet<&str> = ids.iter().map(String::as_str).collect();
        if d.transitions.is_empty() {
            let sizes: HashMap<_, _> = target.iter().map(|s| { let (w, h) = self.state_size(s); (s.id.clone(), (w as f32, h as f32)) }).collect();
            return self.simple_grid(&ids, &sizes, 40.0, 40.0, 120.0, 95.0);
        }
        let mut g = Graph::with_options(GraphOptions { directed: true, multigraph: true, compound: true });
        for s in &target { let (w, h) = self.state_size(s); g.set_node(s.id.clone(), Some(NodeLabel { width: w, height: h, ..Default::default() })); }
        for t in &d.transitions { if id_set.contains(t.from.as_str()) && id_set.contains(t.to.as_str()) { g.set_edge(t.from.clone(), t.to.clone(), Some(EdgeLabel::default()), None); } }
        layout(&mut g, Some(LayoutOptions {
            rankdir: RankDir::TB, nodesep: self.node_spacing_x as f64, ranksep: self.node_spacing_y as f64,
            marginx: 30.0, marginy: 30.0, tie_keep_first: true, ..Default::default()
        }));
        let edge_keys: Vec<_> = d.transitions.iter().map(|t| (t.from.clone(), t.to.clone())).collect();
        self.extract(&g, &ids, &edge_keys)
    }

    pub fn layout_er(&mut self, d: &ErDiagram) -> (HashMap<String, LayoutPos>, EdgeWaypoints, BBox) {
        if d.entities.is_empty() { return (HashMap::new(), HashMap::new(), BBox::default()); }
        let mut seen = HashSet::new();
        let ids: Vec<_> = d.entities.iter().filter(|e| seen.insert(e.name.clone())).map(|e| e.name.clone()).collect();
        if d.relationships.is_empty() {
            let sizes: HashMap<_, _> = d.entities.iter().map(|e| { let (w, h) = self.er_size(e); (e.name.clone(), (w as f32, h as f32)) }).collect();
            return self.simple_grid(&ids, &sizes, 40.0, 40.0, 180.0, 140.0);
        }
        let mut g = Graph::with_options(GraphOptions { directed: true, multigraph: true, compound: false });
        for e in &d.entities { let (w, h) = self.er_size(e); g.set_node(e.name.clone(), Some(NodeLabel { width: w, height: h, ..Default::default() })); }
        for r in &d.relationships { g.set_edge(r.from.clone(), r.to.clone(), Some(EdgeLabel::default()), None); }
        layout(&mut g, Some(LayoutOptions {
            rankdir: RankDir::LR, nodesep: self.node_spacing_x as f64, ranksep: self.node_spacing_y as f64,
            marginx: 30.0, marginy: 30.0, tie_keep_first: true, ..Default::default()
        }));
        let edge_keys: Vec<_> = d.relationships.iter().map(|r| (r.from.clone(), r.to.clone())).collect();
        self.extract(&g, &ids, &edge_keys)
    }

    pub fn layout_sequence(&mut self, d: &SequenceDiagram) -> (HashMap<String, LayoutPos>, BBox) {
        let mut pos = HashMap::new();
        if d.participants.is_empty() { return (pos, BBox::default()); }
        let ph = (self.font_size * 2.4).max(36.0) as f32;
        let widths: Vec<f32> = d.participants.iter().map(|p| {
            let label = p.alias.as_ref().unwrap_or(&p.id);
            (self.text_w(label, self.font_size, false, false, false) + 28.0).max(96.0) as f32
        }).collect();
        let mut centers = vec![40.0 + widths[0] / 2.0];
        for i in 1..d.participants.len() {
            let prev = centers[i-1];
            let gap = ((widths[i-1] + widths[i]) / 2.0 + 72.0).max(140.0);
            centers.push(prev + gap);
        }
        let mut idx = HashMap::new();
        for (i, p) in d.participants.iter().enumerate() { idx.insert(p.id.as_str(), i); }
        let mut reqs = Vec::new();
        collect_seq_reqs(&d.elements, &idx, self, &mut reqs);
        for _ in 0..3 {
            let mut changed = false;
            for (a, b, req) in &reqs {
                if *a >= *b { continue; }
                let dist = centers[*b] - centers[*a];
                if dist + 0.5 < *req { let delta = *req - dist; for c in &mut centers[*b..] { *c += delta; } changed = true; }
            }
            if !changed { break; }
        }
        for (i, p) in d.participants.iter().enumerate() {
            pos.insert(p.id.clone(), LayoutPos::new(centers[i] - widths[i] / 2.0, 20.0, widths[i], ph));
        }
        let mut cy = 20.0 + ph + 40.0;
        let mut mhw = 0.0f32;
        measure_seq(&d.elements, self, &mut cy, &mut mhw);
        if d.elements.is_empty() { cy += 40.0; }
        let l = pos.values().map(|p| p.x).fold(f32::MAX, f32::min).min(40.0);
        let r = pos.values().map(|p| p.right()).fold(0.0, f32::max).max(160.0);
        let bb = BBox::new(l, 0.0, (r - l) + mhw * 2.0, cy + 20.0).with_padding(self.edge_label_padding / 2.0);
        (pos, bb)
    }

    // ── helpers ──

    fn extract(&self, g: &Graph<NodeLabel, EdgeLabel>, node_ids: &[String], edge_keys: &[(String, String)]) -> (HashMap<String, LayoutPos>, EdgeWaypoints, BBox) {
        let mut pos = HashMap::new();
        for id in node_ids {
            if let Some(n) = g.node(id) {
                if let (Some(x), Some(y)) = (n.x, n.y) {
                    pos.insert(id.clone(), LayoutPos::new((x - n.width / 2.0) as f32, (y - n.height / 2.0) as f32, n.width as f32, n.height as f32));
                }
            }
        }
        let mut wps = HashMap::new();
        for (from, to) in edge_keys {
            if let Some(e) = g.edge(from, to, None) {
                let pts: Vec<_> = e.points.iter().map(|p| (p.x as f32, p.y as f32)).collect();
                if pts.len() > 2 { wps.insert((from.clone(), to.clone()), pts[1..pts.len()-1].to_vec()); }
            }
        }
        let mut min_x = f32::MAX; let mut min_y = f32::MAX;
        let mut max_x = f32::MIN; let mut max_y = f32::MIN;
        for p in pos.values() { min_x = min_x.min(p.x); min_y = min_y.min(p.y); max_x = max_x.max(p.right()); max_y = max_y.max(p.bottom()); }
        let bb = if pos.is_empty() { BBox::default() } else { BBox::new(min_x, min_y, max_x - min_x + 20.0, max_y - min_y + 20.0) };
        (pos, wps, bb)
    }

    fn simple_grid(&self, ids: &[String], sizes: &HashMap<String, (f32, f32)>, sx: f32, sy: f32, spacing_x: f32, spacing_y: f32) -> (HashMap<String, LayoutPos>, EdgeWaypoints, BBox) {
        let mut pos = HashMap::new();
        if ids.is_empty() { return (pos, HashMap::new(), BBox::default()); }
        let cols = (ids.len() as f32).sqrt().ceil() as usize; let cols = cols.max(1);
        let mut rh = vec![0.0f32; ids.len().div_ceil(cols)];
        for (i, id) in ids.iter().enumerate() { let r = i / cols; rh[r] = rh[r].max(sizes.get(id).map(|(_, h)| *h).unwrap_or(40.0)); }
        for (i, id) in ids.iter().enumerate() {
            let col = i % cols; let row = i / cols;
            let (w, h) = sizes.get(id).copied().unwrap_or((120.0, 40.0));
            pos.insert(id.clone(), LayoutPos::new(sx + col as f32 * (w + spacing_x), sy + rh[..row].iter().sum::<f32>() + row as f32 * spacing_y, w, h));
        }
        let mut min_x = f32::MAX; let mut min_y = f32::MAX; let mut max_x = f32::MIN; let mut max_y = f32::MIN;
        for p in pos.values() { min_x = min_x.min(p.x); min_y = min_y.min(p.y); max_x = max_x.max(p.right()); max_y = max_y.max(p.bottom()); }
        (pos, HashMap::new(), BBox::new(min_x, min_y, max_x - min_x + 20.0, max_y - min_y + 20.0))
    }

    fn class_size(&mut self, c: &ClassDefinition) -> (f64, f64) {
        let hf = self.font_size; let mf = hf * 0.85;
        let header = if c.is_interface { format!("<<{}>> {}", c.stereotype.as_deref().unwrap_or("interface"), c.name) }
                     else if c.stereotype.is_some() { format!("<<{}>> {}", c.stereotype.as_ref().unwrap(), c.name) }
                     else { c.name.clone() };
        let mut mw = self.text_w(&header, hf, false, true, c.is_abstract).max(120.0);
        for a in &c.attributes {
            let vis = vis_char(a.member.visibility);
            let t = if let Some(ref ty) = a.type_annotation { format!("{} {}: {}", vis, a.member.name, ty) } else { format!("{} {}", vis, a.member.name) };
            mw = mw.max(self.text_w(&t, mf, true, false, a.member.is_abstract));
        }
        for m in &c.methods {
            let vis = vis_char(m.member.visibility);
            let params: Vec<_> = m.parameters.iter().map(|(n, t)| if let Some(ty) = t { format!("{}: {}", n, ty) } else { n.clone() }).collect();
            let t = if let Some(ref r) = m.return_type { format!("{} {}({}): {}", vis, m.member.name, params.join(", "), r) }
                    else { format!("{} {}({})", vis, m.member.name, params.join(", ")) };
            mw = mw.max(self.text_w(&t, mf, true, false, m.member.is_abstract));
        }
        let w = (mw as f64 + self.node_padding_h as f64 * 2.0).max(180.0);
        let lh = (mf * 1.2) as f64;
        let mut h = hf as f64 + 16.0 + hf as f64 + 4.0;
        h += c.attributes.len() as f64 * lh;
        if !c.methods.is_empty() { h += 4.0 + hf as f64 + 2.0 + c.methods.len() as f64 * lh; }
        h += 14.0; h = h.max(64.0);
        (w, h)
    }

    fn state_size(&mut self, s: &State) -> (f64, f64) {
        if s.is_start || s.is_end { return (24.0, 24.0); }
        let lw = self.text_w(&s.label, self.font_size, false, false, false) as f64;
        let _w = (lw + self.node_padding_h as f64 * 2.0).max(120.0);
        let bh = (self.font_size as f64 * 2.2).max(40.0);
        if !s.is_composite { return (lw + self.node_padding_h as f64 * 2.0, bh); }
        // composite states use simple estimate
        ((lw + self.node_padding_h as f64 * 2.0).max(200.0), (bh * 3.0).max(120.0))
    }

    fn er_size(&mut self, e: &ErEntity) -> (f64, f64) {
        let mut mw = self.text_w(&e.name, self.font_size, false, true, false);
        for a in &e.attributes {
            let t = if a.is_key { format!("[{}]", a.name) } else { a.name.clone() };
            mw = mw.max(self.text_w(&t, self.font_size * 0.85, false, false, false));
        }
        let w = (mw as f64 + self.node_padding_h as f64 * 2.0).max(150.0);
        let d = if e.attributes.is_empty() { 0.0 } else { self.font_size as f64 * 0.5 + 4.0 };
        let h = (34.0 + d + e.attributes.len() as f64 * (self.font_size as f64 * 1.3) + 10.0).max(56.0);
        (w, h)
    }
}

fn vis_char(v: Visibility) -> &'static str {
    match v { Visibility::Public => "+", Visibility::Private => "-", Visibility::Protected => "#", Visibility::Package => "~" }
}

fn collect_seq_reqs<T: TextMeasure>(elements: &[SequenceElement], idx: &HashMap<&str, usize>, eng: &mut LayoutEngine<T>, out: &mut Vec<(usize, usize, f32)>) {
    for el in elements {
        match el {
            SequenceElement::Message(msg) => {
                if let (Some(&f), Some(&t)) = (idx.get(msg.from.as_str()), idx.get(msg.to.as_str())) {
                    if f != t {
                        let a = f.min(t); let b = f.max(t);
                        let lw = eng.text_w(&msg.label, eng.font_size * 0.85, false, false, false);
                        let req = (lw + 42.0).max(120.0);
                        out.push((a, b, req));
                        if b - a > 1 {
                            let per = ((lw + 20.0) / (b - a - 1).max(1) as f32 + 40.0).max(140.0);
                            for i in a..b { out.push((i, i + 1, per)); }
                        }
                    }
                }
            }
            SequenceElement::Block(block) => {
                collect_seq_reqs(&block.messages, idx, eng, out);
                for (_, br) in &block.else_branches { collect_seq_reqs(br, idx, eng, out); }
            }
            _ => {}
        }
    }
}

fn measure_seq<T: TextMeasure>(elements: &[SequenceElement], eng: &mut LayoutEngine<T>, cy: &mut f32, mhw: &mut f32) {
    for el in elements {
        match el {
            SequenceElement::Message(msg) => {
                let lw = eng.text_w(&msg.label, eng.font_size * 0.85, false, false, false);
                *mhw = (*mhw).max(lw / 2.0 + eng.edge_label_padding);
                *cy += 50.0;
            }
            SequenceElement::Activation(_) | SequenceElement::Deactivation(_) => { *cy += 24.0; }
            SequenceElement::Note { text, .. } => { eng.text_w(text, eng.font_size * 0.8, false, false, false); *cy += 42.0; }
            SequenceElement::Block(block) => {
                *cy += 34.0;
                measure_seq(&block.messages, eng, cy, mhw);
                for (_, br) in &block.else_branches { *cy += 30.0; measure_seq(br, eng, cy, mhw); }
                *cy += 20.0;
            }
        }
    }
}
