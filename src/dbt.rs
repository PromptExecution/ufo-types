//! dbt manifest.json -> ufo_types::sysgraph::SysGraph (kr0ki digital-thread box-1
//! front-end). See kr0ki's docs/superpowers/specs/2026-09-20-dbt-manifest-digital-
//! thread-design.md for the full design.
//!
//! Decisions a consumer needs to know, matching `reqif`'s bar for this kind of note:
//!
//! - **v12 only, for now.** [`Upgrade::upgrade`] rejects any manifest whose
//!   `metadata.dbt_schema_version` major doesn't match `DbtManifest::DIALECT`
//!   (currently `12.0.0`) with `DialectError::MajorMismatch` -- see
//!   `crate::dialect` for why that's a hard rejection rather than a best-effort
//!   decode.
//! - **`dbt:{unique_id}` identity.** Every dbt-derived `ElementId` is
//!   `format!("dbt:{unique_id}")` (see `dbt_element_id`), reusing dbt's own
//!   stable `unique_id` rather than deriving a content hash. This is the whole
//!   zero-drift mechanism (spec §5): re-ingesting the same manifest always
//!   produces the same `ElementId`s for the same dbt nodes, so a future
//!   reconciliation pass (spec §9, not built here) can diff/replace everything
//!   under the `dbt:` prefix without touching anything else in the graph --
//!   human-authored elements or another front-end's elements never carry that
//!   prefix.
//! - **`DanglingDependency` is a deliberate hard error in this first cut,** not
//!   skip-and-warn. A `depends_on.nodes` entry that doesn't resolve within the
//!   qualifying node set (see `qualifying_nodes`) fails the whole lift. Spec §6
//!   leaves the door open to downgrading this later, with evidence from a real
//!   fixture corpus -- that call is not made here.
//! - **`parse_and_lift` is acquisition-neutral.** It takes `bytes: &[u8]` and
//!   performs no I/O of its own, exactly like `reqif::parse_and_lift` --
//!   whoever calls it (kr0ki-core, a future CLI, a future MCP tool) owns
//!   however those bytes were read (spec §7).

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::dialect::{DialectError, DialectUrn, Upgrade};
use crate::ontology::{OntologicalEdge, UfoRelation};
use crate::stereotype::UfoStereotype;
use crate::sysgraph::{OntologicalNode, SysGraph};
use crate::sysml_model::ElementId;

/// The one field this module reads out of dbt's `metadata` block: the
/// schema-version URL [`peek_dbt_schema_version`] extracts a major version
/// from, and [`Upgrade::upgrade`] validates against. Real dbt manifests carry
/// many more `metadata.*` fields (`project_id`, `generated_at`, ...) -- out of
/// scope until a real second consumer needs them (spec §3).
#[derive(Debug, Clone, Deserialize)]
pub struct DbtManifestMetadata {
    pub dbt_schema_version: String,
}

/// A dbt node's `depends_on` block, narrowed to the one field this module
/// lowers: `nodes`, the list of other `unique_id`s the node references. Each
/// entry becomes an outgoing `Requires` edge (spec §4). Real dbt also carries
/// `depends_on.macros`; not modeled here.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DbtDependsOn {
    #[serde(default)]
    pub nodes: Vec<String>,
}

/// One entry from dbt's `nodes` or `sources` map -- deliberately narrow (spec
/// §3): only what's needed to produce a `SysGraph` node plus its lineage
/// edges, not a full dbt manifest reimplementation. Compiled/raw SQL,
/// descriptions, column-level metadata, and dbt's many other per-node fields
/// are out of scope for this first cut.
#[derive(Debug, Clone, Deserialize)]
pub struct DbtNode {
    /// dbt's own stable node identity, e.g. `model.jaffle_shop.stg_customers`.
    /// Becomes this node's `ElementId` via [`dbt_element_id`] (spec §5).
    pub unique_id: String,
    /// e.g. `model`, `source`, `seed`, `snapshot`, `test`. Only
    /// `model`/`seed`/`snapshot` (plus every `sources` entry, unconditionally)
    /// qualify for lowering -- see [`is_qualifying_node`].
    pub resource_type: String,
    /// dbt's bare node name, e.g. `customers`. Not unique across resource
    /// types -- a model and a source can share one, which is why
    /// [`dbt_node_label`] folds `schema` into the label.
    pub name: String,
    /// Folded into the node's label as a disambiguating suffix when present
    /// (see [`dbt_node_label`]) -- not otherwise used.
    #[serde(default)]
    pub schema: Option<String>,
    /// Parsed but intentionally unused for now: not needed to disambiguate
    /// labels, and no current consumer reads it (spec §3: extend only when a
    /// real second consumer needs a field).
    #[serde(default)]
    pub database: Option<String>,
    /// This node's outgoing lineage references; see [`DbtDependsOn`].
    #[serde(default)]
    pub depends_on: DbtDependsOn,
}

/// The root of a dbt `manifest.json`, narrowed to what
/// [`dbt_manifest_to_sysgraph`] needs (spec §3). `nodes` holds every
/// non-source resource type (models, seeds, snapshots, tests, ...); `sources`
/// is dbt's separate top-level map for `source` resource types. Lowering
/// treats every `sources` entry as qualifying unconditionally and filters
/// `nodes` by `resource_type` -- see [`qualifying_nodes`].
#[derive(Debug, Clone, Deserialize)]
pub struct DbtManifest {
    /// Carries the schema-version URL used to pick the right
    /// [`Upgrade::upgrade`] call.
    pub metadata: DbtManifestMetadata,
    /// Keyed by `unique_id`; see the struct doc for the nodes/sources split.
    #[serde(default)]
    pub nodes: BTreeMap<String, DbtNode>,
    /// Keyed by `unique_id`; dbt's separate top-level map for `source`
    /// resource types.
    #[serde(default)]
    pub sources: BTreeMap<String, DbtNode>,
}

/// Reserved for future lift-time options -- empty for this first cut (spec
/// leaves it unspecified; a future field might be something like a
/// `skip_missing_deps: bool` per spec §6's downgrade-later note). Present now
/// so [`dbt_manifest_to_sysgraph`]'s and [`parse_and_lift`]'s signatures don't
/// need to change when one is added.
#[derive(Debug, Clone, Default)]
pub struct DbtLiftConfig {}

fn dbt_element_id(unique_id: &str) -> ElementId {
    ElementId::new(format!("dbt:{unique_id}"))
}

/// Whether a `manifest.nodes` entry's `resource_type` is lowered to a
/// `SysGraph` node. Every `manifest.sources` entry qualifies unconditionally
/// regardless of this predicate -- see [`qualifying_nodes`].
fn is_qualifying_node(resource_type: &str) -> bool {
    matches!(resource_type, "model" | "seed" | "snapshot")
}

/// The single source of truth for "which dbt nodes actually make it into the
/// `SysGraph`", keyed by `unique_id`. Both [`lower_nodes`] and [`lower_edges`]
/// call this rather than each deriving their own filter -- that drift was
/// exactly the C1 bug this fixes: `lower_edges` used to walk
/// `manifest.nodes` unfiltered and check dependency existence against the
/// same unfiltered maps, so a `test.*` node's `depends_on` produced an edge
/// *from* a node `lower_nodes` never created, and a qualifying node
/// depending on a non-qualifying-but-present node (e.g. `analysis.*`)
/// produced an edge *to* one -- either way leaving `SysGraph::dangling_edges`
/// non-empty on any real-world manifest.
fn qualifying_nodes(manifest: &DbtManifest) -> BTreeMap<&str, &DbtNode> {
    manifest
        .sources
        .values()
        .chain(
            manifest
                .nodes
                .values()
                .filter(|n| is_qualifying_node(&n.resource_type)),
        )
        .map(|n| (n.unique_id.as_str(), n))
        .collect()
}

/// A node's display label: `name`, plus a debug-only `(schema)` suffix when
/// one is known (spec §4: "source vs. model distinction lives in the
/// `ElementId` prefix and a debug-only label suffix, not a second
/// stereotype"). Without the suffix, a model and a source that share a bare
/// `name` -- this module's own `FIXTURE` has `model.jaffle_shop.customers`
/// and `source.jaffle_shop.raw.customers`, both named `customers` -- would
/// render with identical labels despite having different `ElementId`s.
/// `database` is parsed but deliberately left out of the label; not needed to
/// disambiguate and not asked for by the spec.
fn dbt_node_label(node: &DbtNode) -> String {
    match &node.schema {
        Some(schema) => format!("{} ({})", node.name, schema),
        None => node.name.clone(),
    }
}

/// Lower every qualifying node (see [`qualifying_nodes`]) to an
/// [`OntologicalNode`]. One stereotype, `Kind("DbtModel")`, for all four
/// resource types in this first cut (spec §4) -- source vs. model is
/// recovered from the `ElementId` prefix and [`dbt_node_label`]'s suffix, not
/// a second stereotype.
fn lower_nodes(manifest: &DbtManifest, graph: &mut SysGraph) {
    for node in qualifying_nodes(manifest).values().copied() {
        graph.push_node(OntologicalNode::with_label(
            dbt_element_id(&node.unique_id),
            UfoStereotype::Kind("DbtModel".into()),
            dbt_node_label(node),
        ));
    }
}

/// Lower every qualifying node's `depends_on.nodes` entries to
/// [`OntologicalEdge`]s using [`UfoRelation::Requires`] (spec §4): `source`
/// is the node whose `depends_on` list this is, `target` is the referenced
/// upstream node. Both which nodes are walked as the "from" side and what
/// `dep_exists` is checked against come from the same [`qualifying_nodes`]
/// set [`lower_nodes`] used -- see that function's doc for why this must be
/// the *same* set rather than a fresh unfiltered walk (the C1 fix).
fn lower_edges(manifest: &DbtManifest, graph: &mut SysGraph) -> Result<(), DbtLiftError> {
    let qualifying = qualifying_nodes(manifest);
    for node in qualifying.values().copied() {
        for dep_id in &node.depends_on.nodes {
            let dep_exists = qualifying.contains_key(dep_id.as_str());
            if !dep_exists {
                return Err(DbtLiftError::DanglingDependency {
                    from: node.unique_id.clone(),
                    to: dep_id.clone(),
                });
            }
            graph.push_edge(OntologicalEdge {
                id: format!("dbt:{}->{}", node.unique_id, dep_id),
                source: dbt_element_id(&node.unique_id),
                target: dbt_element_id(dep_id),
                relation: UfoRelation::Requires,
                occurrence: None,
                provenance: vec![],
            });
        }
    }
    Ok(())
}

/// Everything that can go wrong turning manifest bytes into a `SysGraph`.
#[derive(Debug, thiserror::Error)]
pub enum DbtLiftError {
    /// `serde_json` failed to deserialize the manifest at the target version.
    #[error("could not parse manifest.json: {0}")]
    Malformed(#[from] serde_json::Error),
    /// Schema-version peek or [`Upgrade::upgrade`] failed -- see
    /// [`DialectError`].
    #[error(transparent)]
    Dialect(#[from] DialectError),
    /// A `depends_on.nodes` entry names a `unique_id` that
    /// [`qualifying_nodes`] didn't lower to a graph node. A hard error in
    /// this first cut, not skip-and-warn (spec §6 leaves the door open to
    /// downgrading this later, with evidence from a real fixture corpus --
    /// not revisited here). With the C1 fix, `from`/`to` are checked against
    /// the same qualifying set [`lower_nodes`] uses, so this should only
    /// fire for a genuinely unrepresented node type (e.g. a `depends_on`
    /// naming a macro), not for the scoping bug it used to mask.
    #[error("{from} depends on {to}, which is not present in this manifest")]
    DanglingDependency { from: String, to: String },
}

/// Lower a parsed [`DbtManifest`] into a [`SysGraph`]: every qualifying node
/// (spec §4), then every qualifying node's lineage edges. Fails with
/// [`DbtLiftError::DanglingDependency`] if any edge would reference a node
/// outside that same qualifying set.
pub fn dbt_manifest_to_sysgraph(
    manifest: &DbtManifest,
    _config: &DbtLiftConfig,
) -> Result<SysGraph, DbtLiftError> {
    let mut graph = SysGraph::new();
    lower_nodes(manifest, &mut graph);
    lower_edges(manifest, &mut graph)?;
    Ok(graph)
}

/// The acquisition-neutral entry point (spec §7): bytes in, [`SysGraph`] out,
/// no I/O of its own -- the caller (kr0ki-core's HTTP/MCP intake boundary, a
/// future CLI, ...) owns however those bytes got read. Peeks the schema
/// version, upgrades to the canonical [`DbtManifest`] shape, then lowers it.
pub fn parse_and_lift(bytes: &[u8], config: &DbtLiftConfig) -> Result<SysGraph, DbtLiftError> {
    let version = peek_dbt_schema_version(bytes)?;
    let manifest = DbtManifest::upgrade(bytes, &version)?;
    dbt_manifest_to_sysgraph(&manifest, config)
}

/// Extract the schema version dbt itself publishes in every manifest, e.g.
/// "https://schemas.getdbt.com/dbt/manifest/v12.json" -> 12.0.0. Doesn't fully
/// parse the manifest -- callers use this to know which `Upgrade::upgrade`
/// call to make before paying for the full deserialize.
pub fn peek_dbt_schema_version(bytes: &[u8]) -> Result<semver::Version, DbtLiftError> {
    #[derive(Deserialize)]
    struct MetaOnly {
        metadata: DbtManifestMetadata,
    }
    let meta: MetaOnly = serde_json::from_slice(bytes)?;
    let url = &meta.metadata.dbt_schema_version;
    let major: u64 = url
        .rsplit('/')
        .next()
        .and_then(|last| last.strip_prefix('v'))
        .and_then(|v| v.strip_suffix(".json"))
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| {
            DbtLiftError::Dialect(DialectError::Malformed {
                dialect: Box::new(DbtManifest::DIALECT),
                from_version: semver::Version::new(0, 0, 0),
                reason: format!("could not extract a version number from '{url}'"),
            })
        })?;
    Ok(semver::Version::new(major, 0, 0))
}

impl Upgrade for DbtManifest {
    const DIALECT: DialectUrn = DialectUrn {
        crate_name: "dbt",
        path: "manifest",
        version: semver::Version::new(12, 0, 0),
    };

    fn upgrade(bytes: &[u8], from_version: &semver::Version) -> Result<Self, DialectError> {
        if from_version.major != Self::DIALECT.version.major {
            return Err(DialectError::MajorMismatch {
                from_version: from_version.clone(),
                dialect: Box::new(Self::DIALECT),
                expected_major: Self::DIALECT.version.major,
            });
        }
        serde_json::from_slice(bytes).map_err(|error| DialectError::Malformed {
            dialect: Box::new(Self::DIALECT),
            from_version: from_version.clone(),
            reason: error.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::DialectError;

    const FIXTURE: &str = r#"{
        "metadata": {"dbt_schema_version": "https://schemas.getdbt.com/dbt/manifest/v12.json"},
        "nodes": {
            "model.jaffle_shop.stg_customers": {
                "unique_id": "model.jaffle_shop.stg_customers",
                "resource_type": "model",
                "name": "stg_customers",
                "schema": "staging",
                "database": "analytics",
                "depends_on": {"nodes": ["source.jaffle_shop.raw.customers"]}
            },
            "model.jaffle_shop.customers": {
                "unique_id": "model.jaffle_shop.customers",
                "resource_type": "model",
                "name": "customers",
                "schema": "marts",
                "database": "analytics",
                "depends_on": {"nodes": ["model.jaffle_shop.stg_customers"]}
            }
        },
        "sources": {
            "source.jaffle_shop.raw.customers": {
                "unique_id": "source.jaffle_shop.raw.customers",
                "resource_type": "source",
                "name": "customers",
                "schema": "raw",
                "database": "analytics"
            }
        }
    }"#;

    /// Shaped like real `dbt build` output -- the piece every fixture above
    /// lacked (the C1 review finding): a source, a model that depends on it,
    /// AND a `test.*` node whose `depends_on.nodes` points at the model.
    /// Regression fixture for C1: before the fix, `lower_edges` walked
    /// `manifest.nodes` unfiltered, so this test node's dependency edge was
    /// emitted from a node `lower_nodes` never created, leaving
    /// `SysGraph::dangling_edges()` non-empty.
    const REALISTIC_BUILD_FIXTURE: &str = r#"{
        "metadata": {"dbt_schema_version": "https://schemas.getdbt.com/dbt/manifest/v12.json"},
        "nodes": {
            "model.jaffle_shop.stg_customers": {
                "unique_id": "model.jaffle_shop.stg_customers",
                "resource_type": "model",
                "name": "stg_customers",
                "schema": "staging",
                "database": "analytics",
                "depends_on": {"nodes": ["source.jaffle_shop.raw.customers"]}
            },
            "test.jaffle_shop.not_null_stg_customers_id": {
                "unique_id": "test.jaffle_shop.not_null_stg_customers_id",
                "resource_type": "test",
                "name": "not_null_stg_customers_id",
                "depends_on": {"nodes": ["model.jaffle_shop.stg_customers"]}
            }
        },
        "sources": {
            "source.jaffle_shop.raw.customers": {
                "unique_id": "source.jaffle_shop.raw.customers",
                "resource_type": "source",
                "name": "customers",
                "schema": "raw",
                "database": "analytics"
            }
        }
    }"#;

    #[test]
    fn parses_metadata_nodes_and_sources() {
        let manifest: DbtManifest = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(
            manifest.metadata.dbt_schema_version,
            "https://schemas.getdbt.com/dbt/manifest/v12.json"
        );
        assert_eq!(manifest.nodes.len(), 2);
        assert_eq!(manifest.sources.len(), 1);

        let stg = &manifest.nodes["model.jaffle_shop.stg_customers"];
        assert_eq!(stg.resource_type, "model");
        assert_eq!(
            stg.depends_on.nodes,
            vec!["source.jaffle_shop.raw.customers"]
        );

        let src = &manifest.sources["source.jaffle_shop.raw.customers"];
        assert_eq!(src.resource_type, "source");
        assert!(src.depends_on.nodes.is_empty());
    }

    #[test]
    fn peeks_schema_version_from_metadata() {
        let version = peek_dbt_schema_version(FIXTURE.as_bytes()).unwrap();
        assert_eq!(version, semver::Version::new(12, 0, 0));
    }

    #[test]
    fn upgrade_succeeds_for_matching_major() {
        let version = semver::Version::new(12, 0, 0);
        let manifest = DbtManifest::upgrade(FIXTURE.as_bytes(), &version).unwrap();
        assert_eq!(manifest.nodes.len(), 2);
    }

    #[test]
    fn upgrade_rejects_major_mismatch_without_attempting_decode() {
        let version = semver::Version::new(9, 0, 0);
        let err = DbtManifest::upgrade(FIXTURE.as_bytes(), &version).unwrap_err();
        assert!(matches!(
            err,
            DialectError::MajorMismatch {
                expected_major: 12,
                ..
            }
        ));
    }

    #[test]
    fn lowers_models_and_sources_but_not_other_resource_types() {
        let manifest: DbtManifest = serde_json::from_str(FIXTURE).unwrap();
        let mut graph = SysGraph::new();
        lower_nodes(&manifest, &mut graph);

        assert_eq!(graph.nodes.len(), 3, "2 models + 1 source");
        let ids: Vec<&str> = graph.nodes.iter().map(|n| n.id.0.as_str()).collect();
        assert!(ids.contains(&"dbt:model.jaffle_shop.stg_customers"));
        assert!(ids.contains(&"dbt:model.jaffle_shop.customers"));
        assert!(ids.contains(&"dbt:source.jaffle_shop.raw.customers"));
    }

    #[test]
    fn excludes_non_qualifying_resource_types() {
        let json = r#"{
            "metadata": {"dbt_schema_version": "https://schemas.getdbt.com/dbt/manifest/v12.json"},
            "nodes": {
                "test.jaffle_shop.not_null_customers_id": {
                    "unique_id": "test.jaffle_shop.not_null_customers_id",
                    "resource_type": "test",
                    "name": "not_null_customers_id"
                }
            }
        }"#;
        let manifest: DbtManifest = serde_json::from_str(json).unwrap();
        let mut graph = SysGraph::new();
        lower_nodes(&manifest, &mut graph);
        assert!(
            graph.nodes.is_empty(),
            "test resource_type must not lower to a node"
        );
    }

    #[test]
    fn node_label_disambiguates_same_named_model_and_source_by_schema() {
        let manifest: DbtManifest = serde_json::from_str(FIXTURE).unwrap();
        let mut graph = SysGraph::new();
        lower_nodes(&manifest, &mut graph);

        let model = graph
            .node(&ElementId::new("dbt:model.jaffle_shop.customers"))
            .unwrap();
        let source = graph
            .node(&ElementId::new("dbt:source.jaffle_shop.raw.customers"))
            .unwrap();
        assert_eq!(model.label.as_deref(), Some("customers (marts)"));
        assert_eq!(source.label.as_deref(), Some("customers (raw)"));
        assert_ne!(model.label, source.label, "labels must disambiguate");
    }

    #[test]
    fn node_label_falls_back_to_bare_name_without_a_schema() {
        let json = r#"{
            "metadata": {"dbt_schema_version": "https://schemas.getdbt.com/dbt/manifest/v12.json"},
            "nodes": {
                "model.jaffle_shop.no_schema": {
                    "unique_id": "model.jaffle_shop.no_schema",
                    "resource_type": "model",
                    "name": "no_schema"
                }
            }
        }"#;
        let manifest: DbtManifest = serde_json::from_str(json).unwrap();
        let mut graph = SysGraph::new();
        lower_nodes(&manifest, &mut graph);
        let node = graph
            .node(&ElementId::new("dbt:model.jaffle_shop.no_schema"))
            .unwrap();
        assert_eq!(node.label.as_deref(), Some("no_schema"));
    }

    #[test]
    fn lowers_lineage_edges_with_requires_relation() {
        let manifest: DbtManifest = serde_json::from_str(FIXTURE).unwrap();
        let graph = dbt_manifest_to_sysgraph(&manifest, &DbtLiftConfig::default()).unwrap();

        assert_eq!(graph.edges.len(), 2);
        let stg_edge = graph
            .edges
            .iter()
            .find(|e| e.source.0 == "dbt:model.jaffle_shop.stg_customers")
            .unwrap();
        assert_eq!(stg_edge.target.0, "dbt:source.jaffle_shop.raw.customers");
        assert_eq!(stg_edge.relation, UfoRelation::Requires);
        assert!(graph.dangling_edges().is_empty());
    }

    #[test]
    fn dangling_dependency_is_a_typed_error_not_a_panic() {
        let json = r#"{
            "metadata": {"dbt_schema_version": "https://schemas.getdbt.com/dbt/manifest/v12.json"},
            "nodes": {
                "model.jaffle_shop.orphan": {
                    "unique_id": "model.jaffle_shop.orphan",
                    "resource_type": "model",
                    "name": "orphan",
                    "depends_on": {"nodes": ["model.jaffle_shop.does_not_exist"]}
                }
            }
        }"#;
        let manifest: DbtManifest = serde_json::from_str(json).unwrap();
        let err = dbt_manifest_to_sysgraph(&manifest, &DbtLiftConfig::default()).unwrap_err();
        assert!(matches!(err, DbtLiftError::DanglingDependency { .. }));
    }

    #[test]
    fn a_test_node_depending_on_a_model_does_not_produce_dangling_edges() {
        // Regression test for C1: a `test.*` node (never lowered to a graph
        // node) whose `depends_on.nodes` points at a qualifying model. Before
        // the fix, `lower_edges` walked `manifest.nodes` unfiltered and would
        // have emitted an edge sourced from the test node, which
        // `lower_nodes` never created -- failing `dangling_edges().is_empty()`.
        let graph = parse_and_lift(
            REALISTIC_BUILD_FIXTURE.as_bytes(),
            &DbtLiftConfig::default(),
        )
        .unwrap();

        assert_eq!(
            graph.nodes.len(),
            2,
            "1 model + 1 source; the test node itself does not qualify"
        );
        assert_eq!(
            graph.edges.len(),
            1,
            "only the model -> source edge; the test node is not a qualifying \
             'from' side so its depends_on is never walked"
        );
        assert!(graph.dangling_edges().is_empty());
    }

    #[test]
    fn parse_and_lift_round_trips_the_fixture_end_to_end() {
        let graph = parse_and_lift(FIXTURE.as_bytes(), &DbtLiftConfig::default()).unwrap();
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn parse_and_lift_rejects_malformed_json() {
        let err = parse_and_lift(b"{not json", &DbtLiftConfig::default()).unwrap_err();
        assert!(matches!(err, DbtLiftError::Malformed(_)));
    }

    #[test]
    fn parse_and_lift_on_zero_qualifying_nodes_is_an_empty_graph_not_an_error() {
        let json = r#"{
            "metadata": {"dbt_schema_version": "https://schemas.getdbt.com/dbt/manifest/v12.json"},
            "nodes": {
                "test.jaffle_shop.only_a_test": {
                    "unique_id": "test.jaffle_shop.only_a_test",
                    "resource_type": "test",
                    "name": "only_a_test"
                }
            }
        }"#;
        let graph = parse_and_lift(json.as_bytes(), &DbtLiftConfig::default()).unwrap();
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }
}
