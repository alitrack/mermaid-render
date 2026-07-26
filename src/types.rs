/// Node shapes in flowcharts
#[derive(Debug, Clone, PartialEq)]
pub enum NodeShape {
    Rect,
    RoundedRect,
    Stadium,
    Subroutine,
    Cylinder,
    Circle,
    DoubleCircle,
    Rhombus,
    Hexagon,
    Parallelogram,
    ParallelogramAlt,
    Trapezoid,
    TrapezoidAlt,
}

/// Edge styles
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeStyle {
    Solid,
    Dotted,
    Thick,
}

/// Arrow types
#[derive(Debug, Clone, PartialEq)]
pub enum ArrowType {
    Arrow,
    Circle,
    Cross,
    None,
}

/// A node in a flowchart
#[derive(Debug, Clone)]
pub struct FlowchartNode {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
    /// CSS class names applied via `:::className` syntax
    pub class_names: Vec<String>,
}

impl FlowchartNode {
    pub fn new(id: String, label: String, shape: NodeShape) -> Self {
        Self {
            id,
            label,
            shape,
            class_names: Vec::new(),
        }
    }
}

/// An edge connecting two nodes
#[derive(Debug, Clone)]
pub struct FlowchartEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub style: EdgeStyle,
    pub arrow_head: ArrowType,
    pub arrow_tail: ArrowType,
    pub min_length: usize,
}

/// A complete flowchart diagram
#[derive(Debug, Clone)]
pub struct Flowchart {
    pub direction: FlowDirection,
    pub nodes: Vec<FlowchartNode>,
    pub edges: Vec<FlowchartEdge>,
    pub subgraphs: Vec<Subgraph>,
    /// Class definitions: `classDef className fill:...`
    pub class_defs: Vec<ClassDef>,
    /// Inline node styles: `style nodeId fill:...`
    pub node_styles: Vec<(String, NodeStyle)>,
    /// Link styles: `linkStyle N stroke:...`
    pub link_styles: Vec<LinkStyleDef>,
}

impl Flowchart {
    pub fn new(direction: FlowDirection) -> Self {
        Self {
            direction,
            nodes: Vec::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            class_defs: Vec::new(),
            node_styles: Vec::new(),
            link_styles: Vec::new(),
        }
    }
}

/// A subgraph (grouped nodes)
#[derive(Debug, Clone)]
pub struct Subgraph {
    pub id: String,
    pub title: String,
    pub nodes: Vec<String>,
}

/// Direction of flowchart
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlowDirection {
    TopDown,
    BottomUp,
    LeftRight,
    RightLeft,
}

// ============================================
// Style Types
// ============================================

/// Inline style for a node: `style nodeId fill:#f9f,stroke:#333`
#[derive(Debug, Clone, Default)]
pub struct NodeStyle {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<String>,
    pub color: Option<String>,
}

/// Class definition: `classDef className fill:#f9f,stroke:#333`
#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: String,
    pub style: NodeStyle,
}

/// Link style: `linkStyle 0 stroke:#f00,stroke-width:3px`
#[derive(Debug, Clone)]
pub struct LinkStyleDef {
    pub index: usize,
    pub stroke: Option<String>,
    pub stroke_width: Option<String>,
}

// ============================================
// Sequence Diagram Types
// ============================================

#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    Solid,
    Dotted,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageKind {
    Sync,
    Async,
    Reply,
}

#[derive(Debug, Clone)]
pub struct SequenceMessage {
    pub from: String,
    pub to: String,
    pub label: String,
    pub msg_type: MessageType,
    pub kind: MessageKind,
}

#[derive(Debug, Clone)]
pub struct Activation {
    pub participant: String,
}

#[derive(Debug, Clone)]
pub enum SequenceBlockType {
    Alt,
    Opt,
    Loop,
    Par,
    Critical,
}

#[derive(Debug, Clone)]
pub struct SequenceBlock {
    pub block_type: SequenceBlockType,
    pub label: String,
    pub messages: Vec<SequenceElement>,
    pub else_branches: Vec<(String, Vec<SequenceElement>)>,
}

#[derive(Debug, Clone)]
pub enum SequenceElement {
    Message(SequenceMessage),
    Activation(Activation),
    Deactivation(Activation),
    Note {
        participant: String,
        position: String,
        text: String,
    },
    Block(SequenceBlock),
}

#[derive(Debug, Clone)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub elements: Vec<SequenceElement>,
}

// ============================================
// Class Diagram Types
// ============================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Package,
}

#[derive(Debug, Clone)]
pub struct ClassMember {
    pub visibility: Visibility,
    pub name: String,
    pub is_static: bool,
    pub is_abstract: bool,
}

#[derive(Debug, Clone)]
pub struct ClassAttribute {
    pub member: ClassMember,
    pub type_annotation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClassMethod {
    pub member: ClassMember,
    pub parameters: Vec<(String, Option<String>)>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassRelationType {
    Inheritance,
    Composition,
    Aggregation,
    Association,
    Dependency,
    Realization,
}

#[derive(Debug, Clone)]
pub struct ClassRelation {
    pub from: String,
    pub to: String,
    pub relation_type: ClassRelationType,
    pub label: Option<String>,
    pub multiplicity_from: Option<String>,
    pub multiplicity_to: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClassDefinition {
    pub name: String,
    pub stereotype: Option<String>,
    pub attributes: Vec<ClassAttribute>,
    pub methods: Vec<ClassMethod>,
    pub is_abstract: bool,
    pub is_interface: bool,
}

#[derive(Debug, Clone)]
pub struct ClassDiagram {
    pub classes: Vec<ClassDefinition>,
    pub relations: Vec<ClassRelation>,
}

// ============================================
// State Diagram Types
// ============================================

#[derive(Debug, Clone)]
pub struct State {
    pub id: String,
    pub label: String,
    pub is_start: bool,
    pub is_end: bool,
    pub is_composite: bool,
    pub children: Vec<StateElement>,
}

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub enum StateElement {
    State(State),
    Transition(StateTransition),
    Note { state: String, text: String },
}

#[derive(Debug, Clone)]
pub struct StateDiagram {
    pub states: Vec<State>,
    pub transitions: Vec<StateTransition>,
}

// ============================================
// ER Diagram Types
// ============================================

#[derive(Debug, Clone, PartialEq)]
pub enum ErCardinality {
    ZeroOrOne,
    ExactlyOne,
    ZeroOrMore,
    OneOrMore,
}

#[derive(Debug, Clone)]
pub struct ErAttribute {
    pub name: String,
    pub is_key: bool,
    pub is_composite: bool,
}

#[derive(Debug, Clone)]
pub struct ErEntity {
    pub name: String,
    pub attributes: Vec<ErAttribute>,
}

#[derive(Debug, Clone)]
pub struct ErRelationship {
    pub from: String,
    pub to: String,
    pub from_cardinality: ErCardinality,
    pub to_cardinality: ErCardinality,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ErDiagram {
    pub entities: Vec<ErEntity>,
    pub relationships: Vec<ErRelationship>,
}
