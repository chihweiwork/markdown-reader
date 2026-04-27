//! Mermaid diagram parsers.
//!
//! Supports `graph`/`flowchart`, `sequenceDiagram`, `stateDiagram` /
//! `stateDiagram-v2`, `erDiagram`, `classDiagram`, `journey`, `gantt`,
//! `timeline`, and `gitGraph` syntax.

pub mod class;
pub(crate) mod common;
pub mod er;
pub mod flowchart;
pub mod gantt;
pub mod git_graph;
pub mod journey;
pub mod pie;
pub mod sequence;
pub mod state;
pub mod timeline;

pub use flowchart::parse;
