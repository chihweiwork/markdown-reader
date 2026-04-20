//! Mermaid diagram parsers.
//!
//! Supports `graph`/`flowchart`, `sequenceDiagram`, and `stateDiagram` /
//! `stateDiagram-v2` syntax.

pub mod flowchart;
pub mod sequence;
pub mod state;

pub use flowchart::parse;
