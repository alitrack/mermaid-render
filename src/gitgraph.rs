//! GitGraph diagram: parser, layout, and SVG renderer.
//!
//! Syntax:
//! ```mermaid
//! gitGraph
//!    commit
//!    commit
//!    branch develop
//!    checkout develop
//!    commit
//!    checkout main
//!    merge develop
//! ```

use std::collections::HashMap;

use crate::layout::{BBox, LayoutPos};

// ── Types ──

#[derive(Debug, Clone)]
pub enum GitAction {
    Commit {
        id: Option<String>,
        message: Option<String>,
        tag: Option<String>,
        commit_type: GitCommitType,
    },
    Branch(String),
    Checkout(String),
    Merge(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GitCommitType {
    Normal,
    Reverse,
    Highlight,
}

#[derive(Debug, Clone)]
pub struct GitGraph {
    pub actions: Vec<GitAction>,
}

// ── Parser ──

pub fn parse_gitgraph(input: &str) -> Result<GitGraph, String> {
    let mut actions = Vec::new();

    for line in input.lines().skip(1) {
        // skip "gitGraph" header
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if line.starts_with("commit") {
            let (id, msg, tag, ct) = parse_commit_options(line.strip_prefix("commit").unwrap_or(""));
            actions.push(GitAction::Commit { id, message: msg, tag, commit_type: ct });
        } else if let Some(name) = line.strip_prefix("branch ") {
            actions.push(GitAction::Branch(name.trim().to_string()));
        } else if let Some(name) = line.strip_prefix("checkout ") {
            actions.push(GitAction::Checkout(name.trim().to_string()));
        } else if let Some(name) = line.strip_prefix("merge ") {
            actions.push(GitAction::Merge(name.trim().to_string()));
        }
    }

    if actions.is_empty() {
        return Err("gitGraph: no actions found".to_string());
    }
    Ok(GitGraph { actions })
}

fn parse_commit_options(rest: &str) -> (Option<String>, Option<String>, Option<String>, GitCommitType) {
    let rest = rest.trim();
    let mut id = None;
    let message = None;
    let mut tag = None;
    let mut commit_type = GitCommitType::Normal;

    // Parse id: "123abc"
    let mut pos = 0;
    let chars: Vec<char> = rest.chars().collect();
    while pos < chars.len() && chars[pos].is_whitespace() { pos += 1; }
    if pos < chars.len() && chars[pos] == '"' {
        pos += 1;
        let mut s = String::new();
        while pos < chars.len() && chars[pos] != '"' {
            s.push(chars[pos]); pos += 1;
        }
        if !s.is_empty() { id = Some(s); }
        pos += 1; // skip closing quote
    }

    // Parse tag: tag: "v1.0"
    while pos < chars.len() && chars[pos].is_whitespace() { pos += 1; }
    if pos + 4 < chars.len() && rest[pos..].starts_with("tag:") {
        pos += 4;
        while pos < chars.len() && chars[pos].is_whitespace() { pos += 1; }
        if pos < chars.len() && chars[pos] == '"' {
            pos += 1;
            let mut s = String::new();
            while pos < chars.len() && chars[pos] != '"' {
                s.push(chars[pos]); pos += 1;
            }
            if !s.is_empty() { tag = Some(s); }
        }
    }

    // Parse type: type: HIGHLIGHT / REVERSE
    while pos < chars.len() && chars[pos].is_whitespace() { pos += 1; }
    if pos + 5 < chars.len() && rest[pos..].to_lowercase().starts_with("type:") {
        let after = rest[pos+5..].trim().to_lowercase();
        if after.contains("highlight") { commit_type = GitCommitType::Highlight; }
        else if after.contains("reverse") { commit_type = GitCommitType::Reverse; }
    }

    (id, message, tag, commit_type)
}

// ── Layout ──

struct BranchState {
    name: String,
    color_idx: usize,
    commits: Vec<usize>, // indices into commit_list
}

const BRANCH_COLORS: &[&str] = &[
    "#3b82f6", "#ef4444", "#10b981", "#f59e0b",
    "#8b5cf6", "#ec4899", "#06b6d4", "#f97316",
];

#[derive(Debug, Clone, Copy)]
struct CommitPos {
    x: f32,
    y: f32,
    branch_color_idx: usize,
    commit_type: GitCommitType,
}

pub fn layout_gitgraph(graph: &GitGraph, _font_size: f32) -> (HashMap<String, LayoutPos>, BBox) {
    let mut positions = HashMap::new();
    if graph.actions.is_empty() {
        return (positions, BBox::default());
    }

    let mut branches: Vec<BranchState> = vec![BranchState {
        name: "main".to_string(),
        color_idx: 0,
        commits: Vec::new(),
    }];
    let mut current_branch = 0usize;
    let mut commit_list: Vec<(usize, GitCommitType, Option<String>, Option<String>)> = Vec::new();
    // (branch_idx, type, id, tag)

    let node_w = 14.0;
    let node_h = 14.0;
    let _spacing_x = 30.0;
    let spacing_y = 30.0;

    for action in &graph.actions {
        match action {
            GitAction::Commit { id, tag, commit_type, .. } => {
                let idx = commit_list.len();
                branches[current_branch].commits.push(idx);
                commit_list.push((current_branch, *commit_type, id.clone(), tag.clone()));
            }
            GitAction::Branch(name) => {
                let ci = branches.len();
                branches.push(BranchState {
                    name: name.clone(),
                    color_idx: ci % BRANCH_COLORS.len(),
                    commits: Vec::new(),
                });
                current_branch = ci;
            }
            GitAction::Checkout(name) => {
                if let Some(idx) = branches.iter().position(|b| b.name == *name) {
                    current_branch = idx;
                }
            }
            GitAction::Merge(_from_name) => {
                // Add a merge commit on current branch
                let idx = commit_list.len();
                branches[current_branch].commits.push(idx);
                commit_list.push((current_branch, GitCommitType::Normal, None, None));
            }
        }
    }

    if commit_list.is_empty() {
        return (positions, BBox::default());
    }

    // Layout: commits stacked vertically per branch, branches side by side
    let n_branches = branches.len();
    let branch_x_start = 40.0;
    let branch_x_gap = 80.0;

    // Assign x per branch
    let branch_x: Vec<f32> = (0..n_branches)
        .map(|i| branch_x_start + i as f32 * branch_x_gap)
        .collect();

    let mut branch_y: Vec<f32> = vec![40.0; n_branches];
    let mut commit_positions: Vec<CommitPos> = Vec::new();

    for (branch_idx, ct, _id, _tag) in &commit_list {
        let bi = *branch_idx;
        let y = branch_y[bi];
        commit_positions.push(CommitPos {
            x: branch_x[bi],
            y,
            branch_color_idx: branches[bi].color_idx,
            commit_type: *ct,
        });
        branch_y[bi] += spacing_y;
    }

    for (i, cp) in commit_positions.iter().enumerate() {
        positions.insert(
            format!("gc-{i}"),
            LayoutPos::new(cp.x, cp.y, node_w, node_h),
        );
    }

    // Calculate bbox
    let mut min_x = f32::MAX; let mut min_y = f32::MAX;
    let mut max_x = f32::MIN; let mut max_y = f32::MIN;
    for pos in positions.values() {
        min_x = min_x.min(pos.x); min_y = min_y.min(pos.y);
        max_x = max_x.max(pos.x + pos.width); max_y = max_y.max(pos.y + pos.height);
    }
    let bbox = BBox::new(
        min_x - 20.0,
        min_y - 10.0,
        max_x - min_x + 120.0,
        max_y - min_y + 40.0,
    );
    (positions, bbox)
}

// ── Render ──

pub fn render_gitgraph(
    graph: &GitGraph,
    style: &crate::render::DiagramStyle,
    font_size: f32,
) -> Result<(String, f32, f32), String> {
    use crate::render::escape_xml;

    let (positions, bbox) = layout_gitgraph(graph, font_size);
    if positions.is_empty() {
        return Ok(("<g></g>".to_string(), 100.0, 50.0));
    }

    let mut svg = String::new();

    // Organize commits by branch for line drawing
    let mut branches: Vec<BranchState> = vec![BranchState {
        name: "main".to_string(), color_idx: 0, commits: Vec::new(),
    }];
    let mut current_branch = 0usize;
    let mut commit_list: Vec<(usize, GitCommitType, Option<String>, Option<String>)> = Vec::new();

    for action in &graph.actions {
        match action {
            GitAction::Commit { id, tag, commit_type, .. } => {
                let idx = commit_list.len();
                branches[current_branch].commits.push(idx);
                commit_list.push((current_branch, *commit_type, id.clone(), tag.clone()));
            }
            GitAction::Branch(name) => {
                let ci = branches.len();
                branches.push(BranchState { name: name.clone(), color_idx: ci % BRANCH_COLORS.len(), commits: Vec::new() });
                current_branch = ci;
            }
            GitAction::Checkout(name) => {
                if let Some(idx) = branches.iter().position(|b| b.name == *name) { current_branch = idx; }
            }
            GitAction::Merge(_) => {
                let idx = commit_list.len();
                branches[current_branch].commits.push(idx);
                commit_list.push((current_branch, GitCommitType::Normal, None, None));
            }
        }
    }

    // Draw branch lines
    for branch in &branches {
        if branch.commits.len() < 2 { continue; }
        let color = BRANCH_COLORS[branch.color_idx];
        let first = positions.get(&format!("gc-{}", branch.commits[0]));
        let last = positions.get(&format!("gc-{}", branch.commits[branch.commits.len() - 1]));
        if let (Some(f), Some(l)) = (first, last) {
            let cx = f.x + f.width / 2.0;
            svg.push_str(&format!(
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{color}" stroke-width="2" />"#,
                cx, f.y + f.height / 2.0, cx, l.y + l.height / 2.0
            ));
        }
    }

    // Draw commits
    for i in 0..commit_list.len() {
        if let Some(pos) = positions.get(&format!("gc-{i}")) {
            let (_, ct, _, tag) = &commit_list[i];
            let cx = pos.x + pos.width / 2.0;
            let cy = pos.y + pos.height / 2.0;
            let r = 7.0;
            let (fill, stroke) = match ct {
                GitCommitType::Normal => (style.node_fill.as_str(), style.node_stroke.as_str()),
                GitCommitType::Reverse => (style.node_stroke.as_str(), style.node_fill.as_str()),
                GitCommitType::Highlight => ("#f59e0b", "#d97706"),
            };

            svg.push_str(&format!(
                r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{fill}" stroke="{stroke}" stroke-width="1.5" />"#,
                cx, cy, r
            ));

            // Tag label
            if let Some(t) = tag {
                svg.push_str(&format!(
                    r#"<text x="{:.2}" y="{:.2}" dy="-0.5em" font-family="{}" font-size="9" fill="{}" text-anchor="middle">{}</text>"#,
                    cx, cy, style.font_family, style.edge_text, escape_xml(t)
                ));
            }
        }
    }

    let total_w = bbox.width + 40.0;
    let total_h = bbox.height + 40.0;
    Ok((svg, total_w, total_h))
}
