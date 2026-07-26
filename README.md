# mermaid-rs

**Pure Rust Mermaid diagram parser and SVG renderer.**

Zero JavaScript. Zero Node.js. Minimal dependencies. Parses Mermaid diagram syntax and renders to SVG entirely in Rust — now powered by **dagre-rs** for production-quality hierarchical graph layout.

Extracted from [MarkieCli](https://github.com/lsj5031/MarkieCli) (MIT licensed).

## Architecture

```
.mmd → parser.rs → types.rs → layout.rs (dagre-rs) → render.rs → SVG
```

- **Parser**: Hand-written, zero-dependency Mermaid syntax parser (7 diagram types)
- **Layout**: [dagre-rs](https://github.com/kookyleo/dagre-rs) — same Sugiyama layout engine mermaid.js uses (network simplex + barycenter + Brandes-Koepf)
- **Renderer**: Direct SVG generation with all 13 node shapes, edge styles, and arrow types
- **Text measurement**: `TextMeasure` trait with `EstimatedMeasure` default; pluggable font-based backend

## Supported Diagram Types

- **Flowchart** — `graph TD/LR/RL/BT`, 13 node shapes, subgraphs, self-loops
- **Sequence** — participants, messages, notes, activations, loops, alt/else
- **Class** — classes, attributes, methods, inheritance, composition, annotations
- **ER (Entity Relationship)** — entities, attributes, relationships, cardinality
- **State** — states, transitions, composite states, choice nodes, fork/join
- **Gantt** — sections, tasks, dependencies, milestones
- **Pie** — title, slices with labels and values

## Styling

```
classDef highlight fill:#f9f,stroke:#333,stroke-width:2px
class A,B highlight
style C fill:#bbf,stroke:#333
linkStyle 0 stroke:red,stroke-width:3px
A:::highlight --> B:::highlight
```

Style definitions (`NodeStyle`, `ClassDef`, `LinkStyleDef`) are parsed and stored in the `Flowchart` struct. Renderer integration is in progress.

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

## Dependencies

| Crate | Purpose | Runtime deps |
|-------|---------|:---:|
| [dagre](https://crates.io/crates/dagre) | Hierarchical graph layout (Sugiyama) | 0 |
| `log` | Internal logging (dagre dep) | 1 |

**Total runtime dependencies: 1** (dagre itself has zero runtime deps beyond `log`).

## License

MIT — see [LICENSE](LICENSE).
