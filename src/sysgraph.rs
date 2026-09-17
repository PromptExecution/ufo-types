//! `SysGraph` — the box-2 typed envelope for the canonical UFO semantic graph.
//!
//! Sits directly on top of [`crate::ontology`]: a `SysGraph` is a full snapshot
//! of [`OntologicalNode`]s and [`crate::ontology::OntologicalEdge`]s produced by
//! a box-1 front-end (via a box-3 pattern recognizer), ready to be lifted into
//! [`crate::sysml_model`] viewpoints or handed to a renderer adapter.
//!
//! Per `PromptExecution/kr0ki` `docs/DESIGN-NOTE-typed-model-layer.md` §2.6, a
//! typed envelope is acceptable at this layer (unlike [`crate::iso_ir`]'s
//! deliberately untyped transport floor): serde JSON, round-trip tested, same
//! bar as `systhread-core`'s `PositionedGraph`. Per §2.8, it carries **no**
//! `SchemaVersion` field — stability is enforced by golden fixtures and content
//! hashes upstream (e.g. `kr0ki-sysmlv2-client::ModelSnapshot::content_hash`),
//! not by a version tag on this type.
//!
//! Pure data — no traits, no builder machinery. A producer (a recognizer)
//! constructs one by pushing nodes and edges as it walks its source.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ontology::OntologicalEdge;
use crate::stereotype::UfoStereotype;
use crate::sysml_model::ElementId;

/// A single UFO-stereotyped node of a [`SysGraph`].
///
/// Distinct from [`crate::iso_ir::Node`] — that type's `part_type` is a
/// free-form domain string; this one's `stereotype` is the closed
/// [`UfoStereotype`] vocabulary, the same typing discipline
/// [`crate::ontology::OntologicalEdge`] applies to edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OntologicalNode {
    /// This node's identity. Shared id space with [`OntologicalEdge`]
    /// endpoints — an edge's `source`/`target` refers to a node by this id.
    pub id: ElementId,
    /// The UFO stereotype this node is classified under.
    pub stereotype: UfoStereotype,
    /// An optional human-readable label, independent of `id`.
    pub label: Option<String>,
}

impl OntologicalNode {
    /// Construct a node with no label.
    pub fn new(id: ElementId, stereotype: UfoStereotype) -> Self {
        Self {
            id,
            stereotype,
            label: None,
        }
    }

    /// Construct a node with a human-readable label.
    pub fn with_label(id: ElementId, stereotype: UfoStereotype, label: impl Into<String>) -> Self {
        Self {
            id,
            stereotype,
            label: Some(label.into()),
        }
    }
}

/// A full snapshot of the canonical UFO semantic graph: UFO-stereotyped
/// nodes plus the typed edges between them.
///
/// No enforced referential integrity at construction time (a producer may
/// build nodes and edges in either order, or from independently-sourced
/// passes) — see [`SysGraph::dangling_edges`] for a caller-invoked check.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct SysGraph {
    /// Every node in this snapshot.
    pub nodes: Vec<OntologicalNode>,
    /// Every edge in this snapshot.
    pub edges: Vec<OntologicalEdge>,
}

impl SysGraph {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a node.
    pub fn push_node(&mut self, node: OntologicalNode) {
        self.nodes.push(node);
    }

    /// Append an edge.
    pub fn push_edge(&mut self, edge: OntologicalEdge) {
        self.edges.push(edge);
    }

    /// Look up a node by id.
    pub fn node(&self, id: &ElementId) -> Option<&OntologicalNode> {
        self.nodes.iter().find(|n| &n.id == id)
    }

    /// Edges whose `source` or `target` does not resolve to a node in this
    /// graph — a caller-invoked integrity check, not enforced on push.
    pub fn dangling_edges(&self) -> Vec<&OntologicalEdge> {
        self.edges
            .iter()
            .filter(|e| self.node(&e.source).is_none() || self.node(&e.target).is_none())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::UfoRelation;

    fn sample_graph() -> SysGraph {
        let mut g = SysGraph::new();
        g.push_node(OntologicalNode::with_label(
            ElementId::new("svc-a"),
            UfoStereotype::Kind("Service".into()),
            "Service A",
        ));
        g.push_node(OntologicalNode::new(
            ElementId::new("svc-b"),
            UfoStereotype::Kind("Service".into()),
        ));
        g.push_edge(OntologicalEdge::new(
            "svc-a_routes_to_svc-b",
            ElementId::new("svc-a"),
            ElementId::new("svc-b"),
            UfoRelation::RoutesTo,
        ));
        g
    }

    #[test]
    fn sysgraph_round_trips_through_json() {
        let graph = sample_graph();
        let json = serde_json::to_string(&graph).expect("serialize");
        let back: SysGraph = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(graph, back);
    }

    #[test]
    fn node_lookup_finds_pushed_nodes_by_id() {
        let graph = sample_graph();
        assert_eq!(
            graph
                .node(&ElementId::new("svc-a"))
                .map(|n| n.label.as_deref()),
            Some(Some("Service A"))
        );
        assert!(graph.node(&ElementId::new("nonexistent")).is_none());
    }

    #[test]
    fn dangling_edges_are_empty_for_a_well_formed_graph() {
        let graph = sample_graph();
        assert!(graph.dangling_edges().is_empty());
    }

    #[test]
    fn dangling_edges_reports_edges_with_unresolved_endpoints() {
        let mut graph = SysGraph::new();
        graph.push_node(OntologicalNode::new(
            ElementId::new("svc-a"),
            UfoStereotype::Kind("Service".into()),
        ));
        graph.push_edge(OntologicalEdge::new(
            "svc-a_requires_missing",
            ElementId::new("svc-a"),
            ElementId::new("svc-missing"),
            UfoRelation::Requires,
        ));
        let dangling = graph.dangling_edges();
        assert_eq!(dangling.len(), 1);
        assert_eq!(dangling[0].id, "svc-a_requires_missing");
    }

    #[test]
    fn empty_graph_round_trips_and_has_no_dangling_edges() {
        let graph = SysGraph::new();
        let json = serde_json::to_string(&graph).expect("serialize");
        let back: SysGraph = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(graph, back);
        assert!(graph.dangling_edges().is_empty());
    }
}
