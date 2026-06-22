use std::collections::HashMap;

use super::types::*;

#[derive(Debug, Clone)]
pub enum MermaidDiagram {
    Flowchart(Flowchart),
    Sequence(SequenceDiagram),
    ClassDiagram(ClassDiagram),
    StateDiagram(StateDiagram),
    ErDiagram(ErDiagram),
}

/// Parse a mermaid diagram from source text
pub fn parse_mermaid(input: &str) -> Result<MermaidDiagram, String> {
    let input = input.trim();

    // Detect diagram type from first line
    let first_line = input.lines().next().unwrap_or("");

    if first_line.starts_with("flowchart")
        || first_line.starts_with("graph")
        || first_line.starts_with("flowchart ")
        || first_line.starts_with("graph ")
    {
        let diagram =
            parse_flowchart(input).map_err(|e| format!("Flowchart parse error: {}", e))?;
        Ok(MermaidDiagram::Flowchart(diagram))
    } else if first_line.starts_with("sequenceDiagram") || first_line.starts_with("sequence") {
        let diagram =
            parse_sequence(input).map_err(|e| format!("Sequence parse error: {}", e))?;
        Ok(MermaidDiagram::Sequence(diagram))
    } else if first_line.starts_with("classDiagram") || first_line.starts_with("class") {
        let diagram = parse_class(input).map_err(|e| format!("Class parse error: {}", e))?;
        Ok(MermaidDiagram::ClassDiagram(diagram))
    } else if first_line.starts_with("stateDiagram") || first_line.starts_with("state") {
        let diagram = parse_state(input).map_err(|e| format!("State parse error: {}", e))?;
        Ok(MermaidDiagram::StateDiagram(diagram))
    } else if first_line.starts_with("erDiagram") || first_line.starts_with("er") {
        let diagram = parse_er(input).map_err(|e| format!("ER parse error: {}", e))?;
        Ok(MermaidDiagram::ErDiagram(diagram))
    } else {
        let diagram_type = first_line.split_whitespace().next().unwrap_or(first_line);
        eprintln!(
            "Warning: unrecognized Mermaid diagram type '{}', attempting flowchart parse",
            diagram_type
        );
        let diagram =
            parse_flowchart(input).map_err(|e| format!("Flowchart parse error: {}", e))?;
        Ok(MermaidDiagram::Flowchart(diagram))
    }
}

// ============================================
// FLOWCHART PARSER
// ============================================

fn parse_flowchart(input: &str) -> Result<Flowchart, String> {
    let mut lines = input.lines().peekable();

    // Parse direction from first line
    let first_line = lines.next().unwrap_or("");
    let direction = parse_flow_direction(first_line);

    let mut nodes: Vec<FlowchartNode> = Vec::new();
    let mut edges: Vec<FlowchartEdge> = Vec::new();
    let mut subgraphs: Vec<Subgraph> = Vec::new();
    let mut current_subgraph: Option<Subgraph> = None;
    let mut node_labels: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Skip comments
        if line.starts_with("%%") {
            continue;
        }

        // Subgraph start
        if line.starts_with("subgraph ") {
            let raw_title = line.strip_prefix("subgraph ").unwrap_or("").trim();
            let title = if let Some(bracket_pos) = raw_title.find("[\"") {
                let after = &raw_title[bracket_pos + 2..];
                if let Some(end) = after.find("\"]") {
                    after[..end].to_string()
                } else {
                    raw_title.to_string()
                }
            } else {
                raw_title.to_string()
            };
            let id = format!("subgraph_{}", subgraphs.len());
            current_subgraph = Some(Subgraph {
                id: id.clone(),
                title,
                nodes: Vec::new(),
            });
            continue;
        }

        // Subgraph end
        if line == "end" {
            if let Some(sg) = current_subgraph.take() {
                subgraphs.push(sg);
            }
            continue;
        }

        // Try to parse as edge or node definition
        if let Some(parsed) = parse_edge_line(line) {
            // Register nodes if not already present (with full info including label and shape)
            if !node_labels.contains_key(&parsed.from.id) {
                nodes.push(FlowchartNode {
                    id: parsed.from.id.clone(),
                    label: parsed.from.label.clone(),
                    shape: parsed.from.shape,
                });
                node_labels.insert(parsed.from.id.clone(), parsed.from.label.clone());
            }
            if !node_labels.contains_key(&parsed.to.id) {
                nodes.push(FlowchartNode {
                    id: parsed.to.id.clone(),
                    label: parsed.to.label.clone(),
                    shape: parsed.to.shape,
                });
                node_labels.insert(parsed.to.id.clone(), parsed.to.label.clone());
            }

            // Add edge endpoint nodes to current subgraph if inside one
            if let Some(ref mut sg) = current_subgraph {
                if !sg.nodes.contains(&parsed.from.id) {
                    sg.nodes.push(parsed.from.id.clone());
                }
                if !sg.nodes.contains(&parsed.to.id) {
                    sg.nodes.push(parsed.to.id.clone());
                }
            }

            edges.push(FlowchartEdge {
                from: parsed.from.id,
                to: parsed.to.id,
                label: parsed.label,
                style: parsed.style,
                arrow_head: parsed.arrow_head,
                arrow_tail: parsed.arrow_tail,
                min_length: 1,
            });
        } else if let Some((id, label, shape)) = parse_node_definition(line) {
            nodes.push(FlowchartNode {
                id: id.clone(),
                label: label.clone(),
                shape,
            });
            node_labels.insert(id.clone(), label);

            // Add to current subgraph if in one
            if let Some(ref mut sg) = current_subgraph {
                sg.nodes.push(id);
            }
        }
    }

    Ok(Flowchart {
        direction,
        nodes,
        edges,
        subgraphs,
    })
}

fn parse_flow_direction(line: &str) -> FlowDirection {
    let line = line.to_lowercase();
    if line.contains("tb") || line.contains("td") {
        FlowDirection::TopDown
    } else if line.contains("bt") {
        FlowDirection::BottomUp
    } else if line.contains("lr") {
        FlowDirection::LeftRight
    } else if line.contains("rl") {
        FlowDirection::RightLeft
    } else {
        FlowDirection::TopDown
    }
}

struct ParsedNodeInfo {
    id: String,
    label: String,
    shape: NodeShape,
}

struct ParsedEdgeLine {
    from: ParsedNodeInfo,
    to: ParsedNodeInfo,
    label: Option<String>,
    style: EdgeStyle,
    arrow_head: ArrowType,
    arrow_tail: ArrowType,
}

fn parse_edge_line(
    line: &str,
) -> Option<ParsedEdgeLine> {
    // Edge patterns (order matters - longer patterns first)
    let patterns = [
        ("<==>", EdgeStyle::Thick, ArrowType::Arrow, ArrowType::Arrow),
        ("<-->", EdgeStyle::Solid, ArrowType::Arrow, ArrowType::Arrow),
        (
            "<.->",
            EdgeStyle::Dotted,
            ArrowType::Arrow,
            ArrowType::Arrow,
        ),
        ("x==x", EdgeStyle::Thick, ArrowType::Cross, ArrowType::Cross),
        (
            "o==o",
            EdgeStyle::Thick,
            ArrowType::Circle,
            ArrowType::Circle,
        ),
        ("x--x", EdgeStyle::Solid, ArrowType::Cross, ArrowType::Cross),
        (
            "o--o",
            EdgeStyle::Solid,
            ArrowType::Circle,
            ArrowType::Circle,
        ),
        (
            "x-.x",
            EdgeStyle::Dotted,
            ArrowType::Cross,
            ArrowType::Cross,
        ),
        (
            "o-.o",
            EdgeStyle::Dotted,
            ArrowType::Circle,
            ArrowType::Circle,
        ),
        ("<==", EdgeStyle::Thick, ArrowType::None, ArrowType::Arrow),
        ("<--", EdgeStyle::Solid, ArrowType::None, ArrowType::Arrow),
        ("<-.", EdgeStyle::Dotted, ArrowType::None, ArrowType::Arrow),
        (
            "o-->",
            EdgeStyle::Solid,
            ArrowType::Arrow,
            ArrowType::Circle,
        ),
        (
            "--o>",
            EdgeStyle::Solid,
            ArrowType::Arrow,
            ArrowType::Circle,
        ),
        (
            "o==>",
            EdgeStyle::Thick,
            ArrowType::Arrow,
            ArrowType::Circle,
        ),
        (
            "o-.->",
            EdgeStyle::Dotted,
            ArrowType::Arrow,
            ArrowType::Circle,
        ),
        ("==o", EdgeStyle::Thick, ArrowType::Circle, ArrowType::None),
        ("==x", EdgeStyle::Thick, ArrowType::Cross, ArrowType::None),
        ("o==", EdgeStyle::Thick, ArrowType::None, ArrowType::Circle),
        ("x==", EdgeStyle::Thick, ArrowType::None, ArrowType::Cross),
        ("==>", EdgeStyle::Thick, ArrowType::Arrow, ArrowType::None),
        ("--x", EdgeStyle::Solid, ArrowType::Cross, ArrowType::None),
        ("x--", EdgeStyle::Solid, ArrowType::None, ArrowType::Cross),
        ("--o", EdgeStyle::Solid, ArrowType::Circle, ArrowType::None),
        ("o--", EdgeStyle::Solid, ArrowType::None, ArrowType::Circle),
        ("-.->", EdgeStyle::Dotted, ArrowType::Arrow, ArrowType::None),
        ("-.x", EdgeStyle::Dotted, ArrowType::Cross, ArrowType::None),
        ("x-.", EdgeStyle::Dotted, ArrowType::None, ArrowType::Cross),
        ("-.o", EdgeStyle::Dotted, ArrowType::Circle, ArrowType::None),
        ("o-.", EdgeStyle::Dotted, ArrowType::None, ArrowType::Circle),
        ("-.-", EdgeStyle::Dotted, ArrowType::None, ArrowType::None),
        ("-->", EdgeStyle::Solid, ArrowType::Arrow, ArrowType::None),
        ("---", EdgeStyle::Solid, ArrowType::None, ArrowType::None),
        ("->>", EdgeStyle::Solid, ArrowType::Arrow, ArrowType::Arrow),
        ("->", EdgeStyle::Solid, ArrowType::Arrow, ArrowType::None),
        ("--", EdgeStyle::Solid, ArrowType::None, ArrowType::None),
    ];

    for (pattern, style, head, tail) in &patterns {
        if let Some(pos) = line.find(pattern) {
            let from_part = line[..pos].trim();
            let rest = &line[pos + pattern.len()..];

            // Parse optional label
            let (to_part, label) = if let Some(stripped) = rest.strip_prefix('|') {
                // Label before target: A -->|label| B
                if let Some(end_label) = stripped.find('|') {
                    let label_text = stripped[..end_label].trim();
                    let after_label = stripped[end_label + 1..].trim();
                    (after_label, Some(label_text.to_string()))
                } else {
                    (rest.trim(), None)
                }
            } else {
                // No label, just target
                (rest.trim(), None)
            };

            // Extract full node info (id, label, shape)
            let from_info = extract_node_info(from_part)?;
            let to_info = extract_node_info(to_part)?;

            return Some(ParsedEdgeLine {
                from: ParsedNodeInfo {
                    id: from_info.0,
                    label: from_info.1,
                    shape: from_info.2,
                },
                to: ParsedNodeInfo {
                    id: to_info.0,
                    label: to_info.1,
                    shape: to_info.2,
                },
                label,
                style: style.clone(),
                arrow_head: head.clone(),
                arrow_tail: tail.clone(),
            });
        }
    }

    None
}

fn extract_node_id(part: &str) -> Option<String> {
    let part = part.trim();

    // Handle shaped nodes like A[Label], A(Label), A{Label}, etc.
    // Check multi-char patterns first, then single chars
    let multi_char_patterns = ["[[", "((", "[/", "[\\"];
    for pattern in &multi_char_patterns {
        if part.contains(pattern) {
            let pos = part.find(pattern)?;
            return Some(part[..pos].trim().to_string());
        }
    }

    let single_char_patterns = ['[', '(', '{', '<'];
    for bracket in &single_char_patterns {
        if part.contains(*bracket) {
            let pos = part.find(*bracket)?;
            return Some(part[..pos].trim().to_string());
        }
    }

    // Simple node id - alphanumeric and underscores
    let id: String = part
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();

    if id.is_empty() { None } else { Some(id) }
}

/// Extract node ID, label, and shape from a part of an edge definition
fn extract_node_info(part: &str) -> Option<(String, String, NodeShape)> {
    let part = part.trim();

    // Check for shaped node definitions inline
    let patterns: &[(&str, &str, NodeShape)] = &[
        ("(((", ")))", NodeShape::DoubleCircle),
        ("[[", "]]", NodeShape::Subroutine),
        ("((", "))", NodeShape::Circle),
        ("[(", ")]", NodeShape::Cylinder),
        ("([", "])", NodeShape::Stadium),
        ("[/", "/]", NodeShape::Parallelogram),
        ("[\\", "\\]", NodeShape::ParallelogramAlt),
        ("[/", "\\]", NodeShape::Trapezoid),
        ("[\\", "/]", NodeShape::TrapezoidAlt),
        ("{{", "}}", NodeShape::Hexagon),
        ("[", "]", NodeShape::Rect),
        ("(", ")", NodeShape::RoundedRect),
        ("{", "}", NodeShape::Rhombus),
    ];

    for (open, close, shape) in patterns {
        if let Some(pos) = part.find(open) {
            let after_open = &part[pos + open.len()..];
            if let Some(end_pos) = after_open.find(close) {
                let id = part[..pos].trim().to_string();
                let label = normalize_flowchart_label(&after_open[..end_pos]);

                if !id.is_empty() {
                    return Some((id, label, shape.clone()));
                }
            }
        }
    }

    // Simple node - just an ID
    let id = extract_node_id(part)?;
    Some((id.clone(), id, NodeShape::RoundedRect))
}

fn parse_node_definition(line: &str) -> Option<(String, String, NodeShape)> {
    let line = line.trim();

    // Patterns: id[Label], id(Label), id{Label}, etc.
    // Order matters: check longer patterns first
    let patterns: &[(&str, &str, NodeShape)] = &[
        ("(((", ")))", NodeShape::DoubleCircle),
        ("[[", "]]", NodeShape::Subroutine),
        ("((", "))", NodeShape::Circle),
        ("[(", ")]", NodeShape::Cylinder),
        ("([", "])", NodeShape::Stadium),
        ("[/", "/]", NodeShape::Parallelogram),
        ("[\\", "\\]", NodeShape::ParallelogramAlt),
        ("[/", "\\]", NodeShape::Trapezoid),
        ("[\\", "/]", NodeShape::TrapezoidAlt),
        ("{{", "}}", NodeShape::Hexagon),
        ("[", "]", NodeShape::Rect),
        ("(", ")", NodeShape::RoundedRect),
        ("{", "}", NodeShape::Rhombus),
    ];

    for (open, close, shape) in patterns {
        if let Some(pos) = line.find(open) {
            let after_open = &line[pos + open.len()..];
            if let Some(end_pos) = after_open.find(close) {
                let id = line[..pos].trim().to_string();
                let label = normalize_flowchart_label(&after_open[..end_pos]);

                if !id.is_empty() {
                    return Some((id, label, shape.clone()));
                }
            }
        }
    }

    None
}

fn normalize_flowchart_label(raw: &str) -> String {
    let mut label = raw.trim().to_string();

    // Mermaid line-break tags should become actual newlines before layout/measurement.
    label = label
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n");

    // Labels like A["Text"] should render as Text, not "Text".
    if let Some(inner) = label
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        label = inner.trim().to_string();
    }

    label
}

// ============================================
// SEQUENCE DIAGRAM PARSER
// ============================================

fn parse_sequence(input: &str) -> Result<SequenceDiagram, String> {
    let lines: Vec<String> = input
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("%%"))
        .map(str::to_string)
        .collect();

    let mut participants: Vec<Participant> = Vec::new();
    for line in &lines {
        // Participant declaration
        if line.starts_with("participant ") {
            let rest = line.strip_prefix("participant ").unwrap_or("");
            // Mermaid syntax: participant A as "Display Name" or participant A
            // The ID comes first, then optional "as Alias"
            let parts: Vec<&str> = rest.splitn(2, " as ").collect();
            if parts.len() == 2 {
                participants.push(Participant {
                    id: parts[0].trim().to_string(),
                    alias: Some(parts[1].trim().to_string()),
                });
            } else {
                participants.push(Participant {
                    id: rest.trim().to_string(),
                    alias: None,
                });
            }
            continue;
        }
    }

    let mut index = 0;
    let elements = parse_sequence_elements(&lines, &mut index, true);

    Ok(SequenceDiagram {
        participants,
        elements,
    })
}

fn parse_sequence_elements(
    lines: &[String],
    index: &mut usize,
    allow_break_tokens: bool,
) -> Vec<SequenceElement> {
    let mut elements = Vec::new();

    while *index < lines.len() {
        let line = lines[*index].trim();

        if allow_break_tokens && (line == "end" || line == "else" || line.starts_with("else ")) {
            break;
        }

        if line.starts_with("participant ") || line.starts_with("actor ") {
            *index += 1;
            continue;
        }

        if let Some(msg) = parse_sequence_message(line) {
            elements.push(SequenceElement::Message(msg));
            *index += 1;
            continue;
        }

        if let Some(note) = parse_sequence_note(line) {
            elements.push(note);
            *index += 1;
            continue;
        }

        if let Some(bt) = parse_sequence_block_type(line) {
            let rest = line.split_once(' ').map(|(_, r)| r).unwrap_or("");
            let label = if let Some(colon_pos) = rest.find(':') {
                rest[colon_pos + 1..].trim().to_string()
            } else {
                rest.trim().to_string()
            };

            *index += 1;
            let messages = parse_sequence_elements(lines, index, true);

            let mut else_branches = Vec::new();
            while *index < lines.len() {
                let branch_line = lines[*index].trim();
                if branch_line == "else" || branch_line.starts_with("else ") {
                    let branch_label = branch_line
                        .strip_prefix("else")
                        .map(str::trim)
                        .unwrap_or("")
                        .to_string();
                    *index += 1;
                    let branch_messages = parse_sequence_elements(lines, index, true);
                    else_branches.push((branch_label, branch_messages));
                } else {
                    break;
                }
            }

            if *index < lines.len() && lines[*index].trim() == "end" {
                *index += 1;
            }

            elements.push(SequenceElement::Block(SequenceBlock {
                block_type: bt,
                label,
                messages,
                else_branches,
            }));
            continue;
        }

        if line.starts_with("activate ") {
            let participant = line.strip_prefix("activate ").unwrap_or("").trim();
            elements.push(SequenceElement::Activation(Activation {
                participant: participant.to_string(),
            }));
            *index += 1;
            continue;
        }

        if line.starts_with("deactivate ") {
            let participant = line.strip_prefix("deactivate ").unwrap_or("").trim();
            elements.push(SequenceElement::Deactivation(Activation {
                participant: participant.to_string(),
            }));
            *index += 1;
            continue;
        }

        *index += 1;
    }

    elements
}

fn parse_sequence_block_type(line: &str) -> Option<SequenceBlockType> {
    if line.starts_with("alt ") {
        Some(SequenceBlockType::Alt)
    } else if line.starts_with("opt ") {
        Some(SequenceBlockType::Opt)
    } else if line.starts_with("loop ") {
        Some(SequenceBlockType::Loop)
    } else if line.starts_with("par ") {
        Some(SequenceBlockType::Par)
    } else if line.starts_with("critical ") {
        Some(SequenceBlockType::Critical)
    } else {
        None
    }
}

fn parse_sequence_note(line: &str) -> Option<SequenceElement> {
    if !line.starts_with("Note ") {
        return None;
    }

    let rest = line.strip_prefix("Note ").unwrap_or("").trim();

    let (position, after_prefix) = if let Some(after) = rest.strip_prefix("right of ") {
        ("right", after)
    } else if let Some(after) = rest.strip_prefix("left of ") {
        ("left", after)
    } else if let Some(after) = rest.strip_prefix("over ") {
        ("over", after)
    } else {
        return None;
    };

    let colon_pos = after_prefix.find(':')?;
    let participant = after_prefix[..colon_pos].trim();
    let text = after_prefix[colon_pos + 1..].trim();
    if participant.is_empty() {
        return None;
    }

    Some(SequenceElement::Note {
        participant: participant.to_string(),
        position: position.to_string(),
        text: text.to_string(),
    })
}

fn parse_sequence_message(line: &str) -> Option<SequenceMessage> {
    // Order matters: longer patterns first to avoid partial matches
    let patterns = [
        ("-->>", MessageType::Dotted, MessageKind::Reply),
        ("->>", MessageType::Solid, MessageKind::Sync),
        (">>+", MessageType::Solid, MessageKind::Async),
        (">>-", MessageType::Solid, MessageKind::Async),
        ("-->", MessageType::Dotted, MessageKind::Sync),
        ("->", MessageType::Solid, MessageKind::Sync),
        ("-x", MessageType::Solid, MessageKind::Sync),
        ("-)", MessageType::Solid, MessageKind::Sync),
    ];

    for (pattern, msg_type, kind) in &patterns {
        if let Some(pos) = line.find(pattern) {
            let from = line[..pos].trim().to_string();
            let rest = &line[pos + pattern.len()..];

            // Parse "To: Label" or just "To"
            let (to, label) = if let Some(colon_pos) = rest.find(':') {
                let to_part = rest[..colon_pos].trim().to_string();
                let label_part = rest[colon_pos + 1..].trim().to_string();
                (to_part, Some(label_part))
            } else {
                (rest.trim().to_string(), None)
            };

            return Some(SequenceMessage {
                from,
                to,
                label: label.unwrap_or_default(),
                msg_type: msg_type.clone(),
                kind: kind.clone(),
            });
        }
    }

    None
}

// ============================================
// CLASS DIAGRAM PARSER
// ============================================

fn parse_class(input: &str) -> Result<ClassDiagram, String> {
    let mut lines = input.lines().skip(1); // Skip "classDiagram"

    let mut classes: Vec<ClassDefinition> = Vec::new();
    let mut relations: Vec<ClassRelation> = Vec::new();
    let mut current_class: Option<ClassDefinition> = None;

    for (line_idx, line) in (&mut lines).enumerate() {
        let line_num = line_idx + 2; // +2: skip first line + 1-indexed
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Class definition end
        if line == "}" {
            if let Some(cls) = current_class.take() {
                classes.push(cls);
            }
            continue;
        }

        // Class definition start
        if line.starts_with("class ") {
            let rest = line.strip_prefix("class ").unwrap_or("");

            // Check for stereotype
            let (name, stereotype) = if rest.starts_with("<<") {
                let end = rest.find(">>").ok_or_else(|| format!("line {}: missing '>>' in stereotype", line_num))?;
                let st = &rest[2..end];
                let after = rest[end + 2..].trim();
                (after.to_string(), Some(st.trim().to_string()))
            } else {
                (rest.trim().to_string(), None)
            };

            // Check if it's a one-liner or has body
            if name.ends_with('{') {
                current_class = Some(ClassDefinition {
                    name: name.trim_end_matches('{').trim().to_string(),
                    stereotype,
                    attributes: Vec::new(),
                    methods: Vec::new(),
                    is_abstract: false,
                    is_interface: false,
                });
            } else {
                classes.push(ClassDefinition {
                    name,
                    stereotype,
                    attributes: Vec::new(),
                    methods: Vec::new(),
                    is_abstract: false,
                    is_interface: false,
                });
            }
            continue;
        }

        // If inside a class body
        if let Some(ref mut cls) = current_class {
            // Stereotype annotation inside body (e.g. <<abstract>>, <<interface>>)
            if line.starts_with("<<") {
                if let Some(end) = line.find(">>") {
                    let stereo = line[2..end].trim().to_ascii_lowercase();
                    cls.stereotype = Some(line[2..end].trim().to_string());
                    if stereo == "abstract" {
                        cls.is_abstract = true;
                    } else if stereo == "interface" {
                        cls.is_interface = true;
                    }
                }
                continue;
            }

            // Attribute or method
            if line.starts_with('+')
                || line.starts_with('-')
                || line.starts_with('#')
                || line.starts_with('~')
            {
                let vis = match line.chars().next() {
                    Some('+') => Visibility::Public,
                    Some('-') => Visibility::Private,
                    Some('#') => Visibility::Protected,
                    Some('~') => Visibility::Package,
                    _ => Visibility::Public,
                };

                let member = &line[1..].trim();

                // Check if method (has parentheses)
                if member.contains('(') {
                    if let Some(method) = parse_class_method(vis, member) {
                        cls.methods.push(method);
                    }
                } else if let Some(attr) = parse_class_attribute(vis, member) {
                    cls.attributes.push(attr);
                }
            }
            continue;
        }

        // Relation
        if let Some(rel) = parse_class_relation(line) {
            relations.push(rel);
        }
    }

    Ok(ClassDiagram { classes, relations })
}

fn parse_class_attribute(vis: Visibility, member: &str) -> Option<ClassAttribute> {
    let parts: Vec<&str> = member.splitn(2, ':').collect();
    let name = parts[0].trim();
    let type_ann = parts.get(1).map(|s| s.trim().to_string());

    Some(ClassAttribute {
        member: ClassMember {
            visibility: vis,
            name: name.to_string(),
            is_static: false,
            is_abstract: false,
        },
        type_annotation: type_ann,
    })
}

fn parse_class_method(vis: Visibility, member: &str) -> Option<ClassMethod> {
    let paren_pos = member.find('(')?;
    let name = member[..paren_pos].trim();

    let close_paren = member.find(')')?;
    let params_str = &member[paren_pos + 1..close_paren];

    let return_type = if close_paren + 1 < member.len() {
        let after = member[close_paren + 1..].trim();
        after.strip_prefix(':').map(|stripped| stripped.trim().to_string())
    } else {
        None
    };

    let parameters = if params_str.is_empty() {
        Vec::new()
    } else {
        params_str
            .split(',')
            .filter_map(|p| {
                let parts: Vec<&str> = p.trim().splitn(2, ':').collect();
                if parts.is_empty() {
                    None
                } else {
                    Some((
                        parts[0].trim().to_string(),
                        parts.get(1).map(|s| s.trim().to_string()),
                    ))
                }
            })
            .collect()
    };

    Some(ClassMethod {
        member: ClassMember {
            visibility: vis,
            name: name.to_string(),
            is_static: false,
            is_abstract: false,
        },
        parameters,
        return_type,
    })
}

fn parse_class_relation(line: &str) -> Option<ClassRelation> {
    let patterns = [
        ("<|--", ClassRelationType::Inheritance),
        ("*--", ClassRelationType::Composition),
        ("o--", ClassRelationType::Aggregation),
        ("-->", ClassRelationType::Association),
        ("--", ClassRelationType::Association),
        ("..>", ClassRelationType::Dependency),
        ("..|>", ClassRelationType::Realization),
        ("..", ClassRelationType::Dependency),
    ];

    for (pattern, rel_type) in &patterns {
        if let Some(pos) = line.find(pattern) {
            let from = line[..pos].trim().to_string();
            let rest = line[pos + pattern.len()..].trim();

            let (to, label) = if let Some((to_part, label_part)) = rest.split_once(':') {
                let to = to_part.trim().to_string();
                let label = label_part
                    .trim()
                    .trim_matches('"')
                    .trim()
                    .to_string();
                let label = if label.is_empty() { None } else { Some(label) };
                (to, label)
            } else {
                (rest.to_string(), None)
            };

            return Some(ClassRelation {
                from,
                to,
                relation_type: rel_type.clone(),
                label,
                multiplicity_from: None,
                multiplicity_to: None,
            });
        }
    }

    None
}

// ============================================
// STATE DIAGRAM PARSER
// ============================================

fn parse_state(input: &str) -> Result<StateDiagram, String> {
    const START_STATE_ID: &str = "__start__";
    const END_STATE_ID: &str = "__end__";

    let mut lines = input.lines().skip(1); // Skip "stateDiagram"

    let mut states: Vec<State> = Vec::new();
    let mut transitions: Vec<StateTransition> = Vec::new();
    let mut composite_stack: Vec<String> = Vec::new();

    // Add start state
    states.push(State {
        id: START_STATE_ID.to_string(),
        label: "[*]".to_string(),
        is_start: true,
        is_end: false,
        is_composite: false,
        children: Vec::new(),
    });

    for (line_idx, line) in (&mut lines).enumerate() {
        let line_num = line_idx + 2; // +2: skip first line + 1-indexed
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if line == "}" {
            composite_stack.pop();
            continue;
        }

        if let Some((state_id, text)) = parse_state_note(line) {
            add_state_note(&mut states, &state_id, text);
            continue;
        }

        // State definition
        if line.starts_with("state ") {
            let rest = line.strip_prefix("state ").unwrap_or("");

            let (id, label, is_composite) = parse_state_definition(rest)
                .map_err(|e| format!("line {}: {}", line_num, e))?;
            let state = ensure_state(&mut states, &id, &label, false, false, is_composite);

            if let Some(parent_id) = composite_stack.last().cloned() {
                add_state_child_state(&mut states, &parent_id, &state);
            }

            if is_composite {
                composite_stack.push(id);
            }
            continue;
        }

        // Transition
        if line.contains("-->") || line.contains("->") {
            let arrow = if line.contains("-->") { "-->" } else { "->" };
            if let Some(pos) = line.find(arrow) {
                let from = line[..pos].trim().to_string();
                let rest = &line[pos + arrow.len()..];

                let (to, label) = if rest.contains(':') {
                    let colon_pos = rest.find(':').ok_or_else(|| format!("line {}: expected ':' separator in transition", line_num))?;
                    (
                        rest[..colon_pos].trim().to_string(),
                        Some(rest[colon_pos + 1..].trim().to_string()),
                    )
                } else {
                    (rest.trim().to_string(), None)
                };

                let normalized_from = if from == "[*]" {
                    START_STATE_ID.to_string()
                } else {
                    from.clone()
                };
                let normalized_to = if to == "[*]" {
                    END_STATE_ID.to_string()
                } else {
                    to.clone()
                };

                // Add end state if needed
                if to == "[*]" {
                    ensure_state(&mut states, END_STATE_ID, "[*]", false, true, false);
                }

                // Add states from transition if not already present
                let from_state = if from != "[*]" {
                    Some(ensure_state(
                        &mut states,
                        &normalized_from,
                        &normalized_from,
                        false,
                        false,
                        false,
                    ))
                } else {
                    ensure_state(&mut states, START_STATE_ID, "[*]", true, false, false);
                    None
                };
                let to_state = if to != "[*]" {
                    Some(ensure_state(
                        &mut states,
                        &normalized_to,
                        &normalized_to,
                        false,
                        false,
                        false,
                    ))
                } else {
                    None
                };

                let transition = StateTransition {
                    from: normalized_from,
                    to: normalized_to,
                    label,
                };

                if let Some(parent_id) = composite_stack.last().cloned() {
                    if let Some(state) = from_state.as_ref() {
                        add_state_child_state(&mut states, &parent_id, state);
                    }
                    if let Some(state) = to_state.as_ref() {
                        add_state_child_state(&mut states, &parent_id, state);
                    }
                    add_state_child_transition(&mut states, &parent_id, &transition);
                }

                if composite_stack.is_empty() {
                    transitions.push(transition);
                }
            }
        }
    }

    // Sync composite state data into parent children entries.
    // During parsing, children are added as clones before the composite's own children
    // are parsed, so the child-of-parent copy may be stale.  Walk every parent and
    // update its child entries from the authoritative top-level state.
    let state_index: HashMap<String, usize> = states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.clone(), i))
        .collect();
    // Collect (parent_idx, child_pos, source_idx) tuples first to avoid borrow issues.
    let mut patches: Vec<(usize, usize, usize)> = Vec::new();
    for (pi, parent) in states.iter().enumerate() {
        for (ci, child_elem) in parent.children.iter().enumerate() {
            if let StateElement::State(child_state) = child_elem
                && let Some(&si) = state_index.get(&child_state.id)
                    && si != pi && states[si].is_composite && !child_state.is_composite {
                        patches.push((pi, ci, si));
                    }
        }
    }
    for (pi, ci, si) in patches {
        let fresh = states[si].clone();
        states[pi].children[ci] = StateElement::State(fresh);
    }

    Ok(StateDiagram {
        states,
        transitions,
    })
}

fn parse_state_definition(rest: &str) -> Result<(String, String, bool), String> {
    let mut value = rest.trim();
    let mut is_composite = false;

    if value.ends_with('{') {
        is_composite = true;
        value = value[..value.len() - 1].trim();
    }

    if value.is_empty() {
        return Err("Missing state definition".to_string());
    }

    if let Some((left, right)) = value.split_once(" as ") {
        let id = right.trim();
        if id.is_empty() {
            return Err("Missing state alias identifier".to_string());
        }
        let label = left.trim().trim_matches('"');
        return Ok((id.to_string(), label.to_string(), is_composite));
    }

    if value.contains('"') {
        let label_start = value
            .find('"')
            .ok_or("Missing opening quote in state label")?;
        let label_end = value[label_start + 1..]
            .find('"')
            .ok_or("Missing closing quote in state label")?
            + label_start
            + 1;
        let id = value[..label_start].trim();
        if id.is_empty() {
            return Err("Missing state identifier".to_string());
        }
        let label = value[label_start + 1..label_end].to_string();
        return Ok((id.to_string(), label, is_composite));
    }

    Ok((value.to_string(), value.to_string(), is_composite))
}

fn parse_state_note(line: &str) -> Option<(String, String)> {
    let lower = line.to_ascii_lowercase();
    let prefixes = ["note right of ", "note left of ", "note over "];

    for prefix in prefixes {
        if lower.starts_with(prefix) {
            let after = line[prefix.len()..].trim();
            let colon_pos = after.find(':')?;
            let state_id = after[..colon_pos].trim();
            let text = after[colon_pos + 1..].trim();
            if state_id.is_empty() {
                return None;
            }
            return Some((state_id.to_string(), text.to_string()));
        }
    }

    None
}

fn ensure_state(
    states: &mut Vec<State>,
    id: &str,
    label: &str,
    is_start: bool,
    is_end: bool,
    is_composite: bool,
) -> State {
    if let Some(state) = states.iter_mut().find(|state| state.id == id) {
        if !label.is_empty() {
            state.label = label.to_string();
        }
        state.is_start |= is_start;
        state.is_end |= is_end;
        state.is_composite |= is_composite;
        return state.clone();
    }

    let state = State {
        id: id.to_string(),
        label: if label.is_empty() {
            id.to_string()
        } else {
            label.to_string()
        },
        is_start,
        is_end,
        is_composite,
        children: Vec::new(),
    };
    states.push(state.clone());
    state
}

fn add_state_child_state(states: &mut [State], parent_id: &str, child: &State) {
    if parent_id == child.id {
        return;
    }

    if let Some(parent) = states.iter_mut().find(|state| state.id == parent_id) {
        parent.is_composite = true;
        let has_child = parent
            .children
            .iter()
            .any(|element| matches!(element, StateElement::State(state) if state.id == child.id));
        if !has_child {
            parent.children.push(StateElement::State(child.clone()));
        }
    }
}

fn add_state_child_transition(states: &mut [State], parent_id: &str, transition: &StateTransition) {
    if let Some(parent) = states.iter_mut().find(|state| state.id == parent_id) {
        parent.is_composite = true;
        let has_transition = parent.children.iter().any(|element| {
            matches!(
                element,
                StateElement::Transition(existing)
                    if existing.from == transition.from
                        && existing.to == transition.to
                        && existing.label == transition.label
            )
        });
        if !has_transition {
            parent
                .children
                .push(StateElement::Transition(transition.clone()));
        }
    }
}

fn add_state_note(states: &mut Vec<State>, state_id: &str, text: String) {
    let state = ensure_state(states, state_id, state_id, false, false, false);
    if let Some(target_state) = states.iter_mut().find(|s| s.id == state.id) {
        let has_note = target_state.children.iter().any(|element| {
            matches!(
                element,
                StateElement::Note {
                    state,
                    text: existing_text,
                } if state == state_id && existing_text == &text
            )
        });
        if !has_note {
            target_state.children.push(StateElement::Note {
                state: state_id.to_string(),
                text: text.clone(),
            });
        }
    }

    for parent in states.iter_mut() {
        let contains_state = parent
            .children
            .iter()
            .any(|element| matches!(element, StateElement::State(s) if s.id == state_id));
        if !contains_state {
            continue;
        }

        let has_note = parent.children.iter().any(|element| {
            matches!(
                element,
                StateElement::Note {
                    state,
                    text: existing_text,
                } if state == state_id && existing_text == &text
            )
        });

        if !has_note {
            parent.children.push(StateElement::Note {
                state: state_id.to_string(),
                text: text.clone(),
            });
        }
    }
}

// ============================================
// ER DIAGRAM PARSER
// ============================================

fn parse_er(input: &str) -> Result<ErDiagram, String> {
    let mut lines = input.lines().skip(1); // Skip "erDiagram"

    let mut entities: Vec<ErEntity> = Vec::new();
    let mut relationships: Vec<ErRelationship> = Vec::new();
    let mut current_entity: Option<String> = None;
    let mut current_attributes: Vec<ErAttribute> = Vec::new();

    for raw_line in &mut lines {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }

        // Inside an entity block
        if current_entity.is_some() {
            if trimmed == "}" {
                let entity_name = current_entity.take().unwrap();
                entities.push(ErEntity {
                    name: entity_name,
                    attributes: std::mem::take(&mut current_attributes),
                });
                continue;
            }

            let is_indented = raw_line
                .chars()
                .next()
                .map(|c| c.is_whitespace())
                .unwrap_or(false);

            if is_indented {
                let attr_line = trimmed;
                let is_key = attr_line.starts_with('*');
                let name = if is_key {
                    attr_line[1..].trim()
                } else {
                    attr_line
                };

                current_attributes.push(ErAttribute {
                    name: name.to_string(),
                    is_key,
                    is_composite: false,
                });
                continue;
            }

            // Unexpected unindented line while inside a block: close the block and
            // re-process this line as a top-level statement.
            let entity_name = current_entity.take().unwrap();
            entities.push(ErEntity {
                name: entity_name,
                attributes: std::mem::take(&mut current_attributes),
            });
            // fallthrough
        }

        if trimmed == "}" {
            continue;
        }

        // Relationship
        if let Some(rel) = parse_er_relationship(trimmed) {
            relationships.push(rel);
            continue;
        }

        // Entity definition (block)
        if trimmed.contains('{') {
            let name = trimmed.trim_end_matches('{').trim().to_string();
            current_entity = Some(name);
            current_attributes.clear();
            continue;
        }

        // Simple entity declaration
        entities.push(ErEntity {
            name: trimmed.to_string(),
            attributes: Vec::new(),
        });
    }

    // Save last entity
    if let Some(entity_name) = current_entity {
        entities.push(ErEntity {
            name: entity_name,
            attributes: current_attributes,
        });
    }

    // Auto-create entities referenced in relationships but not explicitly declared
    let mut existing: std::collections::HashSet<&str> =
        entities.iter().map(|e| e.name.as_str()).collect();
    let mut new_entities = Vec::new();
    for rel in &relationships {
        for name in [&rel.from, &rel.to] {
            if !existing.contains(name.as_str()) {
                existing.insert(name.as_str());
                new_entities.push(ErEntity {
                    name: name.clone(),
                    attributes: Vec::new(),
                });
            }
        }
    }
    entities.extend(new_entities);

    Ok(ErDiagram {
        entities,
        relationships,
    })
}

fn parse_er_relationship(line: &str) -> Option<ErRelationship> {
    let patterns = [
        // Solid line patterns (--)
        (
            "||--||",
            ErCardinality::ExactlyOne,
            ErCardinality::ExactlyOne,
        ),
        (
            "||--o{",
            ErCardinality::ExactlyOne,
            ErCardinality::ZeroOrMore,
        ),
        (
            "||--|{",
            ErCardinality::ExactlyOne,
            ErCardinality::OneOrMore,
        ),
        (
            "}o--o{",
            ErCardinality::ZeroOrMore,
            ErCardinality::ZeroOrMore,
        ),
        (
            "}o--||",
            ErCardinality::ZeroOrMore,
            ErCardinality::ExactlyOne,
        ),
        (
            "}o--|{",
            ErCardinality::ZeroOrMore,
            ErCardinality::OneOrMore,
        ),
        (
            "|o--o{",
            ErCardinality::ZeroOrOne,
            ErCardinality::ZeroOrMore,
        ),
        (
            "|o--||",
            ErCardinality::ZeroOrOne,
            ErCardinality::ExactlyOne,
        ),
        ("|o--|{", ErCardinality::ZeroOrOne, ErCardinality::OneOrMore),
        // Dotted line patterns (..)
        (
            "||..||",
            ErCardinality::ExactlyOne,
            ErCardinality::ExactlyOne,
        ),
        (
            "||..o{",
            ErCardinality::ExactlyOne,
            ErCardinality::ZeroOrMore,
        ),
        (
            "||..|{",
            ErCardinality::ExactlyOne,
            ErCardinality::OneOrMore,
        ),
        (
            "}o..o{",
            ErCardinality::ZeroOrMore,
            ErCardinality::ZeroOrMore,
        ),
        (
            "}o..||",
            ErCardinality::ZeroOrMore,
            ErCardinality::ExactlyOne,
        ),
        (
            "}o..|{",
            ErCardinality::ZeroOrMore,
            ErCardinality::OneOrMore,
        ),
        (
            "|o..o{",
            ErCardinality::ZeroOrOne,
            ErCardinality::ZeroOrMore,
        ),
        (
            "|o..||",
            ErCardinality::ZeroOrOne,
            ErCardinality::ExactlyOne,
        ),
        ("|o..|{", ErCardinality::ZeroOrOne, ErCardinality::OneOrMore),
        // }| patterns (OneOrMore from-side) - solid
        (
            "}|--||",
            ErCardinality::OneOrMore,
            ErCardinality::ExactlyOne,
        ),
        (
            "}|--o{",
            ErCardinality::OneOrMore,
            ErCardinality::ZeroOrMore,
        ),
        (
            "}|--|{",
            ErCardinality::OneOrMore,
            ErCardinality::OneOrMore,
        ),
        // }| patterns (OneOrMore from-side) - dotted
        (
            "}|..||",
            ErCardinality::OneOrMore,
            ErCardinality::ExactlyOne,
        ),
        (
            "}|..o{",
            ErCardinality::OneOrMore,
            ErCardinality::ZeroOrMore,
        ),
        (
            "}|..|{",
            ErCardinality::OneOrMore,
            ErCardinality::OneOrMore,
        ),
    ];

    for (pattern, from_card, to_card) in &patterns {
        if let Some(pos) = line.find(pattern) {
            let from = line[..pos].trim().to_string();
            let rest = line[pos + pattern.len()..].trim();

            let (to, label) = if let Some((to_part, label_part)) = rest.split_once(':') {
                let to = to_part.trim().to_string();
                let label = label_part
                    .trim()
                    .trim_matches('"')
                    .trim()
                    .to_string();
                let label = if label.is_empty() { None } else { Some(label) };
                (to, label)
            } else if let Some(stripped) = rest.strip_prefix('"') {
                let end = stripped.find('"')?;
                (stripped[..end].to_string(), None)
            } else {
                (rest.trim().to_string(), None)
            };

            return Some(ErRelationship {
                from,
                to,
                from_cardinality: from_card.clone(),
                to_cardinality: to_card.clone(),
                label,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_flowchart() {
        let input = r#"
flowchart TD
    A --> B
"#;
        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::Flowchart(fc) = result {
            assert_eq!(fc.nodes.len(), 2);
            assert_eq!(fc.edges.len(), 1);
            assert_eq!(fc.nodes[0].id, "A");
            assert_eq!(fc.nodes[0].label, "A");
            assert_eq!(fc.nodes[1].id, "B");
            assert_eq!(fc.edges[0].from, "A");
            assert_eq!(fc.edges[0].to, "B");
        } else {
            panic!("Expected flowchart");
        }
    }

    #[test]
    fn test_parse_flowchart_with_labels() {
        let input = r#"
flowchart TD
    A[Start] --> B{Decision?}
    B -->|Yes| C[Continue]
    B -->|No| D[Stop]
"#;
        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::Flowchart(fc) = result {
            assert_eq!(fc.nodes.len(), 4);

            // Check node A
            let node_a = fc.nodes.iter().find(|n| n.id == "A").unwrap();
            assert_eq!(node_a.label, "Start", "Node A should have label 'Start'");
            assert_eq!(node_a.shape, NodeShape::Rect);

            // Check node B (decision diamond)
            let node_b = fc.nodes.iter().find(|n| n.id == "B").unwrap();
            assert_eq!(
                node_b.label, "Decision?",
                "Node B should have label 'Decision?'"
            );
            assert_eq!(node_b.shape, NodeShape::Rhombus);

            // Check node C
            let node_c = fc.nodes.iter().find(|n| n.id == "C").unwrap();
            assert_eq!(
                node_c.label, "Continue",
                "Node C should have label 'Continue'"
            );

            // Check node D
            let node_d = fc.nodes.iter().find(|n| n.id == "D").unwrap();
            assert_eq!(node_d.label, "Stop", "Node D should have label 'Stop'");

            // Check edge labels
            let edge_bc = fc
                .edges
                .iter()
                .find(|e| e.from == "B" && e.to == "C")
                .unwrap();
            assert_eq!(edge_bc.label, Some("Yes".to_string()));

            let edge_bd = fc
                .edges
                .iter()
                .find(|e| e.from == "B" && e.to == "D")
                .unwrap();
            assert_eq!(edge_bd.label, Some("No".to_string()));
        } else {
            panic!("Expected flowchart");
        }
    }

    #[test]
    fn test_parse_flowchart_shapes() {
        let input = r#"
flowchart LR
    A([Stadium]) --> B[[Subroutine]]
    B --> C[(Database)]
    C --> D((Circle))
    D --> E{Diamond}
"#;
        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::Flowchart(fc) = result {
            assert_eq!(fc.nodes.len(), 5);

            let node_a = fc.nodes.iter().find(|n| n.id == "A").unwrap();
            assert_eq!(node_a.label, "Stadium");
            assert_eq!(node_a.shape, NodeShape::Stadium);

            let node_b = fc.nodes.iter().find(|n| n.id == "B").unwrap();
            assert_eq!(node_b.label, "Subroutine");
            assert_eq!(node_b.shape, NodeShape::Subroutine);

            let node_c = fc.nodes.iter().find(|n| n.id == "C").unwrap();
            assert_eq!(node_c.label, "Database");
            assert_eq!(node_c.shape, NodeShape::Cylinder);

            let node_d = fc.nodes.iter().find(|n| n.id == "D").unwrap();
            assert_eq!(node_d.label, "Circle");
            assert_eq!(node_d.shape, NodeShape::Circle);

            let node_e = fc.nodes.iter().find(|n| n.id == "E").unwrap();
            assert_eq!(node_e.label, "Diamond");
            assert_eq!(node_e.shape, NodeShape::Rhombus);
        } else {
            panic!("Expected flowchart");
        }
    }

    #[test]
    fn test_extract_node_info() {
        // Rect
        let (id, label, shape) = extract_node_info("A[Start]").unwrap();
        assert_eq!(id, "A");
        assert_eq!(label, "Start");
        assert_eq!(shape, NodeShape::Rect);

        // Rhombus
        let (id, label, shape) = extract_node_info("B{Decision?}").unwrap();
        assert_eq!(id, "B");
        assert_eq!(label, "Decision?");
        assert_eq!(shape, NodeShape::Rhombus);

        // Circle
        let (id, label, shape) = extract_node_info("C((Circle))").unwrap();
        assert_eq!(id, "C");
        assert_eq!(label, "Circle");
        assert_eq!(shape, NodeShape::Circle);

        // Simple node
        let (id, label, shape) = extract_node_info("D").unwrap();
        assert_eq!(id, "D");
        assert_eq!(label, "D");
        assert_eq!(shape, NodeShape::RoundedRect);
    }

    #[test]
    fn test_extract_node_info_normalizes_quoted_labels_and_breaks() {
        let (id, label, shape) = extract_node_info(r#"A["Build<br/>Render"]"#).unwrap();
        assert_eq!(id, "A");
        assert_eq!(label, "Build\nRender");
        assert_eq!(shape, NodeShape::Rect);
    }

    #[test]
    fn test_parse_flowchart_normalizes_quoted_labels() {
        let input = r#"
flowchart TD
    A["Start Here"] --> B["Ship<br>Now"]
"#;
        let result = parse_mermaid(input).unwrap();

        if let MermaidDiagram::Flowchart(fc) = result {
            let node_a = fc.nodes.iter().find(|n| n.id == "A").unwrap();
            let node_b = fc.nodes.iter().find(|n| n.id == "B").unwrap();
            assert_eq!(node_a.label, "Start Here");
            assert_eq!(node_b.label, "Ship\nNow");
        } else {
            panic!("Expected flowchart");
        }
    }

    #[test]
    fn test_parse_sequence_diagram() {
        let input = r#"
sequenceDiagram
    participant Alice
    participant Bob
    Alice->>Bob: Hello!
    Bob-->>Alice: Hi!
"#;
        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::Sequence(seq) = result {
            assert_eq!(seq.participants.len(), 2);
            assert_eq!(seq.participants[0].id, "Alice");
            assert_eq!(seq.participants[1].id, "Bob");

            // Check messages exist
            let msg_count = seq
                .elements
                .iter()
                .filter(|e| matches!(e, SequenceElement::Message(_)))
                .count();
            assert_eq!(msg_count, 2);
        } else {
            panic!("Expected sequence diagram");
        }
    }

    #[test]
    fn test_parse_sequence_messages() {
        let input = r#"
sequenceDiagram
    participant Alice
    participant Bob
    Alice->>Bob: Hello
    Bob-->>Alice: Hi there
"#;
        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::Sequence(seq) = result {
            assert_eq!(seq.participants.len(), 2, "Should have 2 participants");

            // Check messages are parsed
            let msg_count = seq
                .elements
                .iter()
                .filter(|e| matches!(e, SequenceElement::Message(_)))
                .count();
            assert_eq!(msg_count, 2, "Should have 2 messages, got {}", msg_count);

            // Check first message
            let first_msg = seq.elements.iter().find_map(|e| {
                if let SequenceElement::Message(m) = e {
                    Some(m)
                } else {
                    None
                }
            });
            assert!(first_msg.is_some(), "Should have at least one message");
            let msg = first_msg.unwrap();
            assert_eq!(msg.from, "Alice", "From should be Alice, got {}", msg.from);
            assert_eq!(msg.to, "Bob", "To should be Bob, got {}", msg.to);
            assert_eq!(
                msg.label, "Hello",
                "Label should be Hello, got '{}'",
                msg.label
            );
        } else {
            panic!("Expected sequence diagram");
        }
    }

    #[test]
    fn test_parse_sequence_with_aliases() {
        let input = r#"
sequenceDiagram
    participant U as User
    participant S as Server
    U->>S: Request
"#;
        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::Sequence(seq) = result {
            assert_eq!(seq.participants.len(), 2);

            let user = &seq.participants[0];
            assert_eq!(user.id, "U");
            assert_eq!(user.alias, Some("User".to_string()));

            let server = &seq.participants[1];
            assert_eq!(server.id, "S");
            assert_eq!(server.alias, Some("Server".to_string()));
        } else {
            panic!("Expected sequence diagram");
        }
    }

    #[test]
    fn test_parse_sequence_notes_and_blocks() {
        let input = r#"
sequenceDiagram
    participant Alice
    participant Bob
    Note right of Alice: Start here
    alt success path
        Alice->>Bob: do work
    else retry
        Bob-->>Alice: try again
    end
"#;

        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::Sequence(seq) = result {
            assert!(seq
                .elements
                .iter()
                .any(|e| matches!(e, SequenceElement::Note { position, participant, text } if position == "right" && participant == "Alice" && text == "Start here")));

            let block = seq.elements.iter().find_map(|e| {
                if let SequenceElement::Block(block) = e {
                    Some(block)
                } else {
                    None
                }
            });
            assert!(block.is_some());
            let block = block.unwrap();
            assert_eq!(block.messages.len(), 1);
            assert_eq!(block.else_branches.len(), 1);
            assert_eq!(block.else_branches[0].0, "retry");
            assert_eq!(block.else_branches[0].1.len(), 1);
        } else {
            panic!("Expected sequence diagram");
        }
    }

    #[test]
    fn test_parse_flowchart_bidirectional_arrows_precedence() {
        let input = r#"
flowchart TD
    A <==> B
    C <--> D
    E <.-> F
"#;

        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::Flowchart(fc) = result {
            assert_eq!(fc.edges.len(), 3);

            let a_b = fc
                .edges
                .iter()
                .find(|e| e.from == "A" && e.to == "B")
                .unwrap();
            assert_eq!(a_b.style, EdgeStyle::Thick);
            assert_eq!(a_b.arrow_head, ArrowType::Arrow);
            assert_eq!(a_b.arrow_tail, ArrowType::Arrow);

            let c_d = fc
                .edges
                .iter()
                .find(|e| e.from == "C" && e.to == "D")
                .unwrap();
            assert_eq!(c_d.style, EdgeStyle::Solid);
            assert_eq!(c_d.arrow_head, ArrowType::Arrow);
            assert_eq!(c_d.arrow_tail, ArrowType::Arrow);

            let e_f = fc
                .edges
                .iter()
                .find(|e| e.from == "E" && e.to == "F")
                .unwrap();
            assert_eq!(e_f.style, EdgeStyle::Dotted);
            assert_eq!(e_f.arrow_head, ArrowType::Arrow);
            assert_eq!(e_f.arrow_tail, ArrowType::Arrow);
        } else {
            panic!("Expected flowchart");
        }
    }

    #[test]
    fn test_parse_class_diagram() {
        let input = r#"
classDiagram
    class Animal {
        +String name
        +int age
        +makeSound()
    }
"#;
        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::ClassDiagram(cls) = result {
            assert!(!cls.classes.is_empty(), "Should have at least one class");
            let animal = cls.classes.iter().find(|c| c.name == "Animal");
            assert!(animal.is_some(), "Should find Animal class");
            let animal = animal.unwrap();
            assert!(
                animal.attributes.len() >= 2,
                "Animal should have at least 2 attributes, got {}",
                animal.attributes.len()
            );
            assert!(
                !animal.methods.is_empty(),
                "Animal should have at least 1 method, got {}",
                animal.methods.len()
            );
            // Attribute name includes type in current parsing
            assert!(
                animal
                    .attributes
                    .iter()
                    .any(|a| a.member.name.contains("name")),
                "Should have name attribute"
            );
            assert!(
                animal
                    .methods
                    .iter()
                    .any(|m| m.member.name.contains("makeSound")),
                "Should have makeSound method"
            );
        } else {
            panic!("Expected class diagram");
        }
    }

    #[test]
    fn test_parse_class_relations_with_labels() {
        let input = r#"
classDiagram
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
        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::ClassDiagram(cls) = result {
            assert_eq!(cls.classes.len(), 3);
            assert_eq!(cls.relations.len(), 2);

            let creates = cls
                .relations
                .iter()
                .find(|r| r.from == "User" && r.to == "Session")
                .expect("missing creates relation");
            assert_eq!(creates.label.as_deref(), Some("creates"));

            let writes = cls
                .relations
                .iter()
                .find(|r| r.from == "User" && r.to == "AuditLog")
                .expect("missing writes relation");
            assert_eq!(writes.label.as_deref(), Some("writes"));
            assert!(matches!(writes.relation_type, ClassRelationType::Dependency));
        } else {
            panic!("Expected class diagram");
        }
    }

    #[test]
    fn test_parse_er_entity_blocks_and_relationship_labels() {
        let input = r#"
erDiagram
    USER ||--o{ ORDER : places
    ORDER ||--|{ ORDER_ITEM : contains

    USER {
        string id
        string email
    }

    ORDER {
        string id
    }

    ORDER_ITEM {
        string order_id
    }
"#;
        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::ErDiagram(er) = result {
            assert_eq!(er.entities.len(), 3);
            assert_eq!(er.relationships.len(), 2);

            let user = er
                .entities
                .iter()
                .find(|e| e.name == "USER")
                .expect("missing USER");
            assert_eq!(user.attributes.len(), 2);
            assert_eq!(user.attributes[0].name, "string id");
            assert_eq!(user.attributes[1].name, "string email");

            let places = er
                .relationships
                .iter()
                .find(|r| r.from == "USER" && r.to == "ORDER")
                .expect("missing places relationship");
            assert_eq!(places.label.as_deref(), Some("places"));
        } else {
            panic!("Expected ER diagram");
        }
    }

    #[test]
    fn test_parse_state_diagram() {
        let input = r#"
stateDiagram
    [*] --> Idle
    Idle --> Processing
    Processing --> [*]
"#;
        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::StateDiagram(st) = result {
            assert!(!st.states.is_empty());
            assert!(!st.transitions.is_empty());

            // Check start state exists
            let has_start = st.states.iter().any(|s| s.is_start);
            assert!(has_start);

            // Check transitions
            let idle_to_processing = st
                .transitions
                .iter()
                .any(|t| t.from == "Idle" && t.to == "Processing");
            assert!(idle_to_processing);
        } else {
            panic!("Expected state diagram");
        }
    }

    #[test]
    fn test_parse_state_composite_with_note_children() {
        let input = r#"
stateDiagram
    state Parent {
        state Child
        Child --> Child: loop
    }
    Note right of Child: child note
"#;
        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::StateDiagram(st) = result {
            let parent = st.states.iter().find(|s| s.id == "Parent").unwrap();
            assert!(parent.is_composite);
            assert!(
                parent
                    .children
                    .iter()
                    .any(|e| matches!(e, StateElement::State(s) if s.id == "Child"))
            );
            assert!(parent.children.iter().any(
                |e| matches!(e, StateElement::Transition(t) if t.from == "Child" && t.to == "Child")
            ));

            let child = st.states.iter().find(|s| s.id == "Child").unwrap();
            assert!(child.children.iter().any(|e| {
                matches!(
                    e,
                    StateElement::Note {
                        state,
                        text,
                    } if state == "Child" && text == "child note"
                )
            }));
        } else {
            panic!("Expected state diagram");
        }
    }

    #[test]
    fn test_flowchart_cycle() {
        let input = r#"
flowchart TD
    A --> B
    B --> C
    C --> A
"#;
        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::Flowchart(fc) = result {
            assert_eq!(fc.nodes.len(), 3);
            assert_eq!(fc.edges.len(), 3);
        } else {
            panic!("Expected flowchart");
        }
    }

    #[test]
    fn test_parse_flowchart_cross_and_circle_arrows() {
        let input = r#"
flowchart TD
    A --x B
    C x-- D
    E --o F
    G o-- H
    I x--x J
    K o--o L
"#;

        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::Flowchart(fc) = result {
            assert_eq!(fc.edges.len(), 6);

            let a_b = fc
                .edges
                .iter()
                .find(|e| e.from == "A" && e.to == "B")
                .unwrap();
            assert_eq!(a_b.arrow_head, ArrowType::Cross);
            assert_eq!(a_b.arrow_tail, ArrowType::None);

            let c_d = fc
                .edges
                .iter()
                .find(|e| e.from == "C" && e.to == "D")
                .unwrap();
            assert_eq!(c_d.arrow_head, ArrowType::None);
            assert_eq!(c_d.arrow_tail, ArrowType::Cross);

            let e_f = fc
                .edges
                .iter()
                .find(|e| e.from == "E" && e.to == "F")
                .unwrap();
            assert_eq!(e_f.arrow_head, ArrowType::Circle);
            assert_eq!(e_f.arrow_tail, ArrowType::None);

            let g_h = fc
                .edges
                .iter()
                .find(|e| e.from == "G" && e.to == "H")
                .unwrap();
            assert_eq!(g_h.arrow_head, ArrowType::None);
            assert_eq!(g_h.arrow_tail, ArrowType::Circle);

            let i_j = fc
                .edges
                .iter()
                .find(|e| e.from == "I" && e.to == "J")
                .unwrap();
            assert_eq!(i_j.arrow_head, ArrowType::Cross);
            assert_eq!(i_j.arrow_tail, ArrowType::Cross);

            let k_l = fc
                .edges
                .iter()
                .find(|e| e.from == "K" && e.to == "L")
                .unwrap();
            assert_eq!(k_l.arrow_head, ArrowType::Circle);
            assert_eq!(k_l.arrow_tail, ArrowType::Circle);
        } else {
            panic!("Expected flowchart");
        }
    }

    #[test]
    fn test_parse_flowchart_dotted_cross_and_circle_arrows() {
        let input = r#"
flowchart TD
    A -.x B
    C x-. D
    E -.o F
    G o-. H
"#;

        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::Flowchart(fc) = result {
            assert_eq!(fc.edges.len(), 4);

            let a_b = fc
                .edges
                .iter()
                .find(|e| e.from == "A" && e.to == "B")
                .unwrap();
            assert_eq!(a_b.style, EdgeStyle::Dotted);
            assert_eq!(a_b.arrow_head, ArrowType::Cross);
            assert_eq!(a_b.arrow_tail, ArrowType::None);

            let c_d = fc
                .edges
                .iter()
                .find(|e| e.from == "C" && e.to == "D")
                .unwrap();
            assert_eq!(c_d.style, EdgeStyle::Dotted);
            assert_eq!(c_d.arrow_head, ArrowType::None);
            assert_eq!(c_d.arrow_tail, ArrowType::Cross);

            let e_f = fc
                .edges
                .iter()
                .find(|e| e.from == "E" && e.to == "F")
                .unwrap();
            assert_eq!(e_f.style, EdgeStyle::Dotted);
            assert_eq!(e_f.arrow_head, ArrowType::Circle);
            assert_eq!(e_f.arrow_tail, ArrowType::None);

            let g_h = fc
                .edges
                .iter()
                .find(|e| e.from == "G" && e.to == "H")
                .unwrap();
            assert_eq!(g_h.style, EdgeStyle::Dotted);
            assert_eq!(g_h.arrow_head, ArrowType::None);
            assert_eq!(g_h.arrow_tail, ArrowType::Circle);
        } else {
            panic!("Expected flowchart");
        }
    }

    #[test]
    fn test_parse_flowchart_open_arrow_variant() {
        let input = r#"
flowchart TD
    A --o> B
"#;

        let result = parse_mermaid(input).unwrap();
        if let MermaidDiagram::Flowchart(fc) = result {
            assert_eq!(fc.edges.len(), 1);
            let edge = &fc.edges[0];
            assert_eq!(edge.from, "A");
            assert_eq!(edge.to, "B");
            assert_eq!(edge.arrow_head, ArrowType::Arrow);
            assert_eq!(edge.arrow_tail, ArrowType::Circle);
        } else {
            panic!("Expected flowchart");
        }
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_parse_class_missing_stereotype_end() {
        let input = "classDiagram\nclass <<abstract Animal";
        let result = parse_mermaid(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing '>>' in stereotype"));
    }

    #[test]
    fn test_parse_state_missing_definition() {
        let input = "stateDiagram\nstate {";
        let result = parse_mermaid(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing state definition"));
    }

    #[test]
    fn test_parse_state_missing_closing_quote() {
        let input = "stateDiagram\nstate \"Label";
        let result = parse_mermaid(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing closing quote in state label"));
    }

    #[test]
    fn test_parse_state_missing_identifier() {
        let input = "stateDiagram\nstate \"Label\"";
        let result = parse_mermaid(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing state identifier"));
    }

    #[test]
    fn test_unknown_diagram_type_produces_result() {
        let result = parse_mermaid("unknownDiagram\n  A --> B");
        assert!(result.is_ok(), "Unknown diagram type should not error");
    }
}
