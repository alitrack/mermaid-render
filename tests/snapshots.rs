//! Visual regression tests using insta snapshots.
//! Run with: `cargo test` or `cargo insta review` to review diffs.

use mermaid_render::{parse_mermaid, render_diagram, EstimatedMeasure, DiagramStyle};

fn default_style() -> DiagramStyle {
    DiagramStyle {
        node_fill: "#eff6ff".into(),
        node_stroke: "#3b82f6".into(),
        node_text: "#1e293b".into(),
        edge_stroke: "#64748b".into(),
        edge_text: "#475569".into(),
        background: "transparent".into(),
        font_family: "sans-serif".into(),
        font_size: 13.0,
    }
}

fn render(source: &str) -> String {
    let (svg, _, _) = render_diagram(source, &default_style(), &mut EstimatedMeasure).unwrap();
    svg
}

// ── Flowchart ──

#[test]
fn snapshot_flowchart_simple() {
    insta::assert_yaml_snapshot!(render("graph TD\n  A-->B\n  B-->C\n"));
}

#[test]
fn snapshot_flowchart_shapes() {
    insta::assert_yaml_snapshot!(render("graph TD\n  A[Box]\n  B(Rounded)\n  C((Circle))\n  D{Rhombus}\n  A-->B-->C-->D\n"));
}

#[test]
fn snapshot_flowchart_multiline() {
    insta::assert_yaml_snapshot!(render("graph LR\n  A[Node with a very\n  long description that\n  spans multiple lines]\n  A-->B[Short]\n"));
}

#[test]
fn snapshot_flowchart_autowrap() {
    // This label should auto-wrap because it exceeds max_node_width (280px)
    insta::assert_yaml_snapshot!(render("graph TD\n  A[This is an extremely long node label that should be automatically wrapped to fit within the node boundaries]\n  A-->B[OK]\n"));
}

#[test]
fn snapshot_flowchart_direction_lr() {
    insta::assert_yaml_snapshot!(render("graph LR\n  Start-->Process-->End\n"));
}

// ── Sequence ──

#[test]
fn snapshot_sequence_simple() {
    insta::assert_yaml_snapshot!(render("sequenceDiagram\n  Alice->>Bob: Hello\n  Bob->>Alice: Hi!\n"));
}

#[test]
fn snapshot_sequence_activation() {
    insta::assert_yaml_snapshot!(render("sequenceDiagram\n  A->>+B: Request\n  B->>-A: Response\n"));
}

// ── Class ──

#[test]
fn snapshot_class_simple() {
    insta::assert_yaml_snapshot!(render("classDiagram\n  Animal <|-- Dog\n  Animal : +name\n  Dog : +bark()\n"));
}

// ── State ──

#[test]
fn snapshot_state_simple() {
    insta::assert_yaml_snapshot!(render("stateDiagram-v2\n  [*]-->Idle\n  Idle-->Running\n  Running-->[*]\n"));
}

// ── ER ──

#[test]
fn snapshot_er_simple() {
    insta::assert_yaml_snapshot!(render("erDiagram\n  CUSTOMER ||--o{ ORDER : places\n  CUSTOMER { string name }\n  ORDER { int id }\n"));
}

// ── Gantt ──

#[test]
fn snapshot_gantt_simple() {
    insta::assert_yaml_snapshot!(render("gantt\n  title Project\n  dateFormat YYYY-MM-DD\n  section Phase 1\n  Task A : a1, 2024-01-01, 7d\n"));
}

// ── Pie ──

#[test]
fn snapshot_pie_simple() {
    insta::assert_yaml_snapshot!(render("pie\n  title Usage\n  \"Chrome\" : 60\n  \"Firefox\" : 20\n"));
}

// ── Timeline ──

#[test]
fn snapshot_timeline_simple() {
    insta::assert_yaml_snapshot!(render("timeline\n  title History\n  2023 : Founded\n  2024 : Series A\n       : Launched\n"));
}

// ── Mindmap ──

#[test]
fn snapshot_mindmap_simple() {
    insta::assert_yaml_snapshot!(render("mindmap\n  root((Central))\n    Topic A\n      Detail A1\n    Topic B\n"));
}

#[test]
fn snapshot_mindmap_long_labels() {
    insta::assert_yaml_snapshot!(render(
        "mindmap\n  root((Project Plan))\n    Research and Analysis Phase\n      Market survey and competitor benchmarking\n      Technical feasibility assessment\n    Implementation and Development\n      Core module architecture design\n      Testing and quality assurance\n      Documentation and user guides\n    Deployment and Operations\n      Production infrastructure setup\n      Monitoring and alerting configuration\n"
    ));
}

// ── GitGraph ──

#[test]
fn snapshot_gitgraph_simple() {
    insta::assert_yaml_snapshot!(render("gitGraph\n  commit\n  branch develop\n  checkout develop\n  commit\n  checkout main\n  merge develop\n"));
}


#[test]
fn snapshot_flowchart_nested_subgraph() {
    insta::assert_yaml_snapshot!(render("graph TD
  subgraph Outer
  A-->B
  subgraph Inner
  C-->D
  end
  end
"));
}

#[test]
fn snapshot_gitgraph_merge() {
    insta::assert_yaml_snapshot!(render("gitGraph
  commit
  branch dev
  checkout dev
  commit
  checkout main
  merge dev
"));
}
