//! Generic graph intermediate representation — `Node`/`Edge`, the shared
//! vocabulary a domain uses to describe "this thing connects to that thing"
//! before anything downstream decides how to lay it out or render it.
//!
//! Promoted from `nem-poweragent-lab/rust/systhread-core/src/iso_ir.rs` (the
//! b00t SysML v2 spine consolidation epic, `elasticdotventures/_b00t_#1177`)
//! — but narrower than that source file. `systhread-core`'s `iso_ir.rs`
//! conflates three concerns in one file: (1) this generic `Node`/`Edge`
//! shape, (2) domain-specific extraction functions that translate its own
//! lab-specific YAML instance types (power-grid buses/generators, digital-
//! thread agents/MCP servers) into that shape, and (3) layout/positioning
//! and JSON-spec assembly that bakes in `(x, y)` coordinates and visual
//! shapes for rendering. Only (1) is genuinely domain-agnostic "type
//! interface" — (2) is lab-specific ETL that has no business in a
//! cross-domain types crate, and (3) is visualization, explicitly out of
//! scope for `ufo-types` per this epic's direction. Both stay in
//! `systhread-core`; only the vocabulary moves here.
//!
//! A domain wanting positioned/rendered output (SVG, Cytoscape JSON, a
//! Mermaid diagram) still owns that translation itself, the same way
//! `systhread-core`'s `layout.rs`/`render.rs` do today — this module only
//! gives that translation a common starting shape to consume.

use serde::{Deserialize, Serialize};

/// A single node in a graph — identity, a human label, and a domain-defined
/// classification tag. `part_type` is a plain `String` here (not a
/// `&'static str` as in `systhread-core`'s original) since a promoted,
/// cross-domain type can't assume every consumer's classification set is
/// known at compile time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    /// Domain-defined classification (e.g. "Agent", "Bus", "MCPServer" in
    /// systhread-core's three lab tracks) — left as a free-form string
    /// rather than an enum, since the set of meaningful values is entirely
    /// the consuming domain's business, not this crate's.
    pub part_type: String,
}

/// A directed connection between two `Node`s by id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    /// Domain-defined edge classification (e.g. "attachment", "sequence",
    /// "branch") — same free-form-string reasoning as `Node::part_type`.
    pub edge_type: String,
    pub kind: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_and_edge_round_trip_through_json() {
        let node = Node {
            id: "a".into(),
            label: "Agent A".into(),
            part_type: "Agent".into(),
        };
        let edge = Edge {
            id: "a_attach".into(),
            from: "a".into(),
            to: "b".into(),
            edge_type: "attachment".into(),
            kind: None,
        };

        let node_json = serde_json::to_string(&node).unwrap();
        let edge_json = serde_json::to_string(&edge).unwrap();
        assert_eq!(serde_json::from_str::<Node>(&node_json).unwrap(), node);
        assert_eq!(serde_json::from_str::<Edge>(&edge_json).unwrap(), edge);
    }

    #[test]
    fn edge_kind_is_optional() {
        let edge = Edge {
            id: "l1".into(),
            from: "bus1".into(),
            to: "bus2".into(),
            edge_type: "branch".into(),
            kind: Some("transformer".into()),
        };
        let json = serde_json::to_string(&edge).unwrap();
        assert!(json.contains("transformer"));

        let no_kind = Edge { kind: None, ..edge };
        let json = serde_json::to_string(&no_kind).unwrap();
        assert!(json.contains("\"kind\":null"));
    }
}
