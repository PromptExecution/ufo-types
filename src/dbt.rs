//! dbt manifest.json -> ufo_types::sysgraph::SysGraph (kr0ki digital-thread box-1
//! front-end). See kr0ki's docs/superpowers/specs/2026-09-20-dbt-manifest-digital-
//! thread-design.md for the full design.

use std::collections::BTreeMap;

use serde::Deserialize;

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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(stg.depends_on.nodes, vec!["source.jaffle_shop.raw.customers"]);

        let src = &manifest.sources["source.jaffle_shop.raw.customers"];
        assert_eq!(src.resource_type, "source");
        assert!(src.depends_on.nodes.is_empty());
    }
}
