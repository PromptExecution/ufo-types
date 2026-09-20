//! dbt manifest.json -> ufo_types::sysgraph::SysGraph (kr0ki digital-thread box-1
//! front-end). See kr0ki's docs/superpowers/specs/2026-09-20-dbt-manifest-digital-
//! thread-design.md for the full design.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::dialect::{DialectError, DialectUrn, Upgrade};
use crate::sysgraph::{OntologicalNode, SysGraph};
use crate::sysml_model::ElementId;
use crate::stereotype::UfoStereotype;

#[derive(Debug, Clone, Deserialize)]
pub struct DbtManifestMetadata {
    pub dbt_schema_version: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DbtDependsOn {
    #[serde(default)]
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DbtNode {
    pub unique_id: String,
    pub resource_type: String,
    pub name: String,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub depends_on: DbtDependsOn,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DbtManifest {
    pub metadata: DbtManifestMetadata,
    #[serde(default)]
    pub nodes: BTreeMap<String, DbtNode>,
    #[serde(default)]
    pub sources: BTreeMap<String, DbtNode>,
}

#[derive(Debug, Clone, Default)]
pub struct DbtLiftConfig {}

fn dbt_element_id(unique_id: &str) -> ElementId {
    ElementId::new(format!("dbt:{unique_id}"))
}

fn lower_nodes(manifest: &DbtManifest, graph: &mut SysGraph) {
    let qualifying = manifest
        .sources
        .values()
        .chain(manifest.nodes.values().filter(|n| {
            matches!(n.resource_type.as_str(), "model" | "seed" | "snapshot")
        }));
    for node in qualifying {
        graph.push_node(OntologicalNode::with_label(
            dbt_element_id(&node.unique_id),
            UfoStereotype::Kind("DbtModel".into()),
            node.name.clone(),
        ));
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DbtLiftError {
    #[error("could not parse manifest.json: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error(transparent)]
    Dialect(#[from] DialectError),
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
        assert!(graph.nodes.is_empty(), "test resource_type must not lower to a node");
    }
}
