//! `ModelCapability` — a minimal "model card" seed describing what data
//! format(s) one model serves. No registry logic lives here (that's the
//! Foundry Local epic's Phase 2, in `_b00t_`'s `b00t-mcp/src/server_llm.rs`)
//! — this is only the shared type both the registry and its consumers
//! (ledgrrr's document-intelligence pipeline) describe capabilities with.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::data_format::DataFormat;

/// What one model serves: which [`DataFormat`]s it accepts/produces, plus
/// free-form model-card metadata (quantization, hardware requirements,
/// etc.) that doesn't need a fixed schema yet — this is the type
/// establishing the concept, not the full registry (Phase 2's job).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModelCapability {
    pub model_name: String,
    pub formats: Vec<DataFormat>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl ModelCapability {
    /// Create a capability descriptor with no metadata.
    pub fn new(model_name: impl Into<String>, formats: Vec<DataFormat>) -> Self {
        Self {
            model_name: model_name.into(),
            formats,
            metadata: HashMap::new(),
        }
    }

    /// True if this model claims to serve the given format.
    pub fn serves(&self, format: &DataFormat) -> bool {
        self.formats.contains(format)
    }

    /// Attach a metadata entry, builder-style.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_format::DataFormat;

    #[test]
    fn new_constructs_with_given_formats() {
        let cap = ModelCapability::new("phi-4-mini", vec![DataFormat::Json, DataFormat::PlainText]);
        assert_eq!(cap.model_name, "phi-4-mini");
        assert_eq!(cap.formats, vec![DataFormat::Json, DataFormat::PlainText]);
        assert!(cap.metadata.is_empty());
    }

    #[test]
    fn serves_checks_membership() {
        let cap = ModelCapability::new("phi-4-mini", vec![DataFormat::Json]);
        assert!(cap.serves(&DataFormat::Json));
        assert!(!cap.serves(&DataFormat::Image));
    }

    #[test]
    fn with_metadata_accumulates_entries() {
        let cap = ModelCapability::new("phi-4-mini", vec![DataFormat::Json])
            .with_metadata("quantization", "int4")
            .with_metadata("hardware", "npu");
        assert_eq!(cap.metadata.get("quantization"), Some(&"int4".to_string()));
        assert_eq!(cap.metadata.get("hardware"), Some(&"npu".to_string()));
    }

    #[test]
    fn model_capability_roundtrips_through_json() {
        let cap = ModelCapability::new("phi-4-mini", vec![DataFormat::Json, DataFormat::Csv])
            .with_metadata("quantization", "int4");
        let json = serde_json::to_string(&cap).unwrap();
        let back: ModelCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, back);
    }
}
