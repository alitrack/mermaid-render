# mermaid-render

**Pure Rust Mermaid diagram parser and SVG renderer — zero JavaScript, zero Node.js, one dependency.**

[![Crates.io](https://img.shields.io/crates/v/mermaid-render)](https://crates.io/crates/mermaid-render)
![License](https://img.shields.io/crates/l/mermaid-render)

Parses Mermaid diagram syntax and renders to SVG entirely in Rust, powered by **dagre-rs** for production-quality hierarchical graph layout (the same Sugiyama algorithm mermaid.js uses).

```rust
use mermaid_render::{render_diagram, EstimatedMeasure};

let src = "graph TD\n  A-->|label|B\n  B-->C";
let (svg, w, h) = render_diagram(src, &Default::default(), &mut EstimatedMeasure).unwrap();
```

## Architecture

```
.mmd → parser.rs → types.rs → layout.rs (dagre-rs) → render.rs → SVG
```

| Layer | Description |
|-------|-------------|
| **Parser** | Hand-written Mermaid syntax parser for 10 diagram types |
| **Layout** | [dagre-rs](https://github.com/kookyleo/dagre-rs) v0.1.1 — network simplex + barycenter + Brandes-Koepf |
| **Renderer** | Direct SVG generation with all 13 node shapes, edge styles, arrow types |
| **Text** | `TextMeasure` trait with `EstimatedMeasure` default; optional `FontMeasure` (ab_glyph) |

## Supported Diagram Types (10)

| Type | Features |
|------|----------|
| **Flowchart** | `graph TD/LR/RL/BT`, 13 node shapes, subgraphs (nested), self-loops, CSS classes, node styles |
| **Sequence** | participants, messages, notes, activations, loops, alt/else, autonumber |
| **Class** | classes, attributes, methods, inheritance, composition, annotations, generics (`Foo~T~`) |
| **ER** | entities, attributes (key/composite), relationships, cardinality |
| **State** | states, transitions, composite states, choice nodes, fork/join |
| **Gantt** | sections, tasks, dependencies, milestones |
| **Pie** | title, slices with labels and values |
| **Timeline** | periods, events |
| **Mindmap** | tree structure with colored nodes |
| **GitGraph** | branches, commits, merges, tags |

## Styling (v0.7+)

Full `classDef`, `style`, and `linkStyle` support — parsed and rendered:

```
classDef highlight fill:#f9f,stroke:#333,stroke-width:2px
class A,B highlight
style C fill:#bbf,stroke:#333
linkStyle 0 stroke:red,stroke-width:3px
A:::highlight --> B:::highlight
```

## Quality

- **58 tests** — 33 unit + 19 insta visual snapshots covering all 10 types
- Auto text wrapping for long labels
- Graceful error SVG fallback (never panics)

## Dependencies

| Crate | Purpose |
|-------|---------|
| `dagre` ^0.1 | Graph layout (Sugiyama) |
| `ab_glyph` (optional) | Accurate font measurement |

## TypePress Integration

mermaid-render is the default diagram engine for [TypePress](https://github.com/alitrack/typepress) (Markdown → PDF). Enable via:

```toml
mermaid-render = { version = "0.9", optional = true }
```

## License

MIT — extracted from [MarkieCli](https://github.com/lsj5031/MarkieCli)
