# mermaid-rs

**Pure Rust Mermaid diagram parser and SVG renderer.**

Zero JavaScript. Zero Node.js. Zero system dependencies. Parses Mermaid diagram syntax and renders to SVG entirely in Rust.

Extracted from [MarkieCli](https://github.com/lsj5031/MarkieCli) (MIT licensed).

## Supported Diagram Types

- **Flowchart** — `graph TD/LR/RL/BT`, nodes, edges, subgraphs, styles, click handlers
- **Sequence** — participants, messages, notes, activations, loops, alt/else
- **Class** — classes, attributes, methods, inheritance, composition, annotations
- **ER (Entity Relationship)** — entities, attributes, relationships, cardinality
- **State** — states, transitions, composite states, choice nodes, fork/join
- **Gantt** — sections, tasks, dependencies, milestones
- **Pie** — title, slices with labels and values

## Usage

```rust
use mermaid_rs::{parse_mermaid, render_diagram, DiagramStyle, EstimatedMeasure, Rect};

let input = "graph TD\n  A[Start] --> B[End]";
let diagram = parse_mermaid(input)?;
let style = DiagramStyle::default();
let mut measure = EstimatedMeasure { svg_size: Rect::new(0.0, 0.0, 800.0, 600.0) };
let svg = render_diagram(&diagram, &style, &mut measure)?;
std::fs::write("diagram.svg", svg)?;
```

## License

MIT — see [LICENSE](LICENSE).
