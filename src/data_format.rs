//! Canonical data-format/modality vocabulary shared across every consumer
//! that registers or requests a specific kind of model-served data — the
//! b00t-server model registry (Foundry Local epic Phase 2, in
//! `_b00t_`'s `b00t-mcp`) and ledgrrr's document-intelligence pipeline
//! (Phase 3+) both describe "what a model serves" in terms of this enum.
//!
//! Deliberately small and non-exhaustive: `Other { format: String }` is the
//! escape hatch for anything not yet enumerated, rather than growing this
//! list ahead of a real second consumer needing the specific variant.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A canonical data format or modality a model can consume or produce.
///
/// Deliberately small — grows only when a second real consumer needs a
/// variant not yet listed, not speculatively.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum DataFormat {
    /// Structured JSON text.
    Json,
    /// Open-form prose/paragraph text. Intentionally has no `Satisfies`
    /// sanity check — well-formedness of free text isn't a syntactic
    /// property the way JSON's is.
    PlainText,
    /// Comma-separated tabular text.
    Csv,
    /// Text already extracted from a PDF (not the PDF binary itself).
    PdfExtractedText,
    /// Raster or vector image data.
    Image,
    /// Anything not yet enumerated above. Carries a free-form label (e.g.
    /// a MIME type) so callers aren't blocked on this enum growing.
    ///
    /// A named-field struct variant, not a tuple variant: this enum uses
    /// serde's internally tagged representation (`tag = "type"`), which
    /// only supports newtype variants whose inner value serializes as a
    /// map — a bare `String` newtype variant would fail to serialize at
    /// runtime. `Other { format: String }` sidesteps that (matches
    /// `Disposition::Violated { reason: String }`'s exact shape in
    /// `src/satisfies.rs`).
    Other { format: String },
}

use crate::satisfies::{Constraint, Satisfies, SatisfiesResult};

/// Constraint: the subject text is well-formed JSON.
///
/// Follows the same `Satisfies<C>` shape as `crate::sysml::SysmlV2Syntax`
/// (see `src/sysml.rs`) — this is the first proof that pattern generalizes
/// to a non-SysML format.
pub struct JsonWellFormed;

impl Constraint for JsonWellFormed {}

impl Satisfies<JsonWellFormed> for str {
    fn satisfies(&self, _constraint: &JsonWellFormed) -> SatisfiesResult {
        validate_json_well_formed(self)
    }
}

impl Satisfies<JsonWellFormed> for String {
    fn satisfies(&self, constraint: &JsonWellFormed) -> SatisfiesResult {
        self.as_str().satisfies(constraint)
    }
}

/// Parse `text` as JSON and report the result as a [`SatisfiesResult`]:
/// `Satisfied` if it parses, `Violated` with `serde_json`'s error message
/// otherwise.
pub fn validate_json_well_formed(text: &str) -> SatisfiesResult {
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(_) => SatisfiesResult::satisfied(1.0, Vec::new()),
        Err(e) => SatisfiesResult::violated(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_format_roundtrips_through_json() {
        let formats = vec![
            DataFormat::Json,
            DataFormat::PlainText,
            DataFormat::Csv,
            DataFormat::PdfExtractedText,
            DataFormat::Image,
            DataFormat::Other { format: "application/x-custom".to_string() },
        ];
        for format in formats {
            let json = serde_json::to_string(&format).unwrap();
            let back: DataFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(format, back);
        }
    }

    #[test]
    fn data_format_wire_shape_is_tagged() {
        let json = serde_json::to_string(&DataFormat::Json).unwrap();
        assert_eq!(json, r#"{"type":"json"}"#);
        let json = serde_json::to_string(&DataFormat::Other { format: "foo".to_string() }).unwrap();
        assert_eq!(json, r#"{"type":"other","format":"foo"}"#);
    }

    #[test]
    fn well_formed_json_satisfies_constraint() {
        let result = validate_json_well_formed(r#"{"a": 1, "b": [true, null]}"#);
        assert!(result.disposition.is_satisfied(), "{:?}", result.disposition);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn malformed_json_violates_constraint() {
        let result = validate_json_well_formed(r#"{"a": 1,"#);
        assert!(!result.disposition.is_satisfied());
        assert!(matches!(result.disposition, crate::satisfies::Disposition::Violated { .. }));
    }

    #[test]
    fn satisfies_trait_usable_on_str_and_string() {
        let owned = String::from(r#"{"ok": true}"#);
        assert!(owned.satisfies(&JsonWellFormed).disposition.is_satisfied());
        assert!(
            owned
                .as_str()
                .satisfies(&JsonWellFormed)
                .disposition
                .is_satisfied()
        );
    }
}
