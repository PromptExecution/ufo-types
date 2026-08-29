//! `Satisfies<C>` — uniform constraint satisfaction pattern.
//!
//! Every domain predicate (R&D eligibility, crypto cost basis rules, gate
//! preconditions, etc.) implements the entity's own constraint type; every
//! domain entity implements `Satisfies<C>` against the constraint types it
//! must answer. Results are structured (`SatisfiesResult`) rather than a
//! bare bool, so evaluations carry confidence and an audit-trail evidence
//! chain.
//!
//! This is the crate's single canonical shape for this pattern — it was
//! independently implemented twice (once here, once in `ledgrrr`'s vendored
//! `crates/ufo-types`) against the same original spec (gh#511) and has since
//! been reconciled: this module's `SatisfiesResult`/`Disposition`/`NodeId`
//! shape and constructor signatures match what `ledger-core`/`ledgerr-mcp`
//! already depend on at ~60 real call sites, so consolidating onto this
//! module is a source change, not a behavior change, for that consumer.
//!
//! # Usage
//! ```ignore
//! use ufo_types::satisfies::{Satisfies, SatisfiesResult, Disposition};
//!
//! let activity = AuRdActivity { /* ... */ };
//! let eligibility = AuRdEligibility::new(2025);
//! let result = activity.satisfies(&eligibility);
//! assert!(matches!(result.disposition, Disposition::Satisfied));
//! ```

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::stereotype::{Stereotyped, UfoStereotype};

// ── Constraint marker ────────────────────────────────────────────────────────

/// Marker for types that can act as a `Satisfies<C>` constraint.
///
/// `Satisfies<C>` itself does not require `C: Constraint` (some consumers —
/// e.g. this crate's own `dare` module — evaluate constraints that don't
/// need a `Send + Sync` bound). Implement this marker on your constraint
/// type when you want that stronger guarantee (e.g. for use across an async
/// MCP server boundary); it costs nothing to add and nothing to omit.
pub trait Constraint: Send + Sync {}

// ── Satisfies trait ──────────────────────────────────────────────────────────

/// Evaluate whether `Self` satisfies the given constraint `C`.
///
/// Every domain type implements this for the constraints it must meet
/// (e.g., `AuRdActivity` satisfies `AuRdEligibility` under ITAA 1997 Div
/// 355). Implementors MUST produce deterministic results — the same inputs
/// always produce the same `SatisfiesResult`.
pub trait Satisfies<C> {
    /// Evaluate this entity against the constraint, returning a structured
    /// result with disposition, confidence, and evidence node IDs.
    fn satisfies(&self, constraint: &C) -> SatisfiesResult;
}

// ── Result types ─────────────────────────────────────────────────────────────

/// The outcome of a constraint evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Disposition {
    /// All criteria met.
    Satisfied,
    /// One or more criteria failed; reason is human-readable.
    Violated {
        /// Why the constraint was violated (e.g., "activity_type is not Core")
        reason: String,
    },
    /// Insufficient evidence to determine satisfaction.
    Unknown,
}

impl Disposition {
    pub fn is_satisfied(&self) -> bool {
        matches!(self, Disposition::Satisfied)
    }

    pub fn is_violated(&self) -> bool {
        matches!(self, Disposition::Violated { .. })
    }
}

impl std::fmt::Display for Disposition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Disposition::Satisfied => write!(f, "Satisfied"),
            Disposition::Violated { reason } => write!(f, "Violated({reason})"),
            Disposition::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Opaque evidence-graph node identifier.
///
/// Format matches arc-kit-au NodeId: `{type_prefix}:{blake3_hex}`. Using a
/// newtype here keeps ufo-types free of an arc-kit-au dependency; callers
/// convert between the two as needed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct NodeId(String);

impl NodeId {
    /// Create a new NodeId from an already-formatted evidence-graph id.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Return the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

fn default_ufo_category() -> UfoStereotype {
    UfoStereotype::Mode("SatisfiesResult".into())
}

/// Structured result of `Satisfies::satisfies`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SatisfiesResult {
    /// Pass / fail / unknown verdict.
    pub disposition: Disposition,
    /// Confidence in the verdict `[0.0, 1.0]`. `1.0` = completely certain;
    /// `0.0` = purely speculative or not evaluated.
    pub confidence: f64,
    /// Evidence graph nodes that support or contradict this verdict.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_nodes: Vec<NodeId>,
    /// UFO stereotype that best describes this satisfaction relation.
    /// Defaults to `Mode("SatisfiesResult")` — satisfying a constraint is
    /// ordinarily an intrinsic property of the entity, not a mediator
    /// between entities.
    #[serde(default = "default_ufo_category")]
    pub ufo_category: UfoStereotype,
}

impl SatisfiesResult {
    /// Create a satisfied result with the given confidence and evidence.
    pub fn satisfied(confidence: f64, evidence_nodes: Vec<NodeId>) -> Self {
        Self {
            disposition: Disposition::Satisfied,
            confidence,
            evidence_nodes,
            ufo_category: default_ufo_category(),
        }
    }

    /// Create a violated result with a reason. Confidence defaults to 0.0;
    /// use the struct literal directly if a violated result needs a
    /// non-zero confidence.
    pub fn violated(reason: impl Into<String>) -> Self {
        Self {
            disposition: Disposition::Violated {
                reason: reason.into(),
            },
            confidence: 0.0,
            evidence_nodes: Vec::new(),
            ufo_category: default_ufo_category(),
        }
    }

    /// Create an unknown result — evaluation could not be completed.
    pub fn unknown() -> Self {
        Self {
            disposition: Disposition::Unknown,
            confidence: 0.0,
            evidence_nodes: Vec::new(),
            ufo_category: default_ufo_category(),
        }
    }

    /// Attach evidence node IDs after construction.
    pub fn with_evidence(mut self, nodes: Vec<NodeId>) -> Self {
        self.evidence_nodes = nodes;
        self
    }

    /// Override the confidence after construction — useful for a partial
    /// `violated()`/`unknown()` result where the caller has a real,
    /// non-zero confidence score to report (e.g. "3 of 4 criteria met").
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    /// Returns true if the constraint is satisfied.
    pub fn is_satisfied(&self) -> bool {
        self.disposition.is_satisfied()
    }

    /// Returns true if the constraint is violated.
    pub fn is_violated(&self) -> bool {
        self.disposition.is_violated()
    }
}

// ── Evidence bridge traits ───────────────────────────────────────────────────

/// A domain constraint that can report which ISO standard(s) it embodies.
///
/// Implemented by constraint types (e.g., `AuRdEligibility`,
/// `UsRdcFourPartTest`) so that domain `Satisfies` impls can auto-call
/// `record_audited_by()` (NS-10).
pub trait IsoAuditable {
    /// ISO standard identifier(s) that govern this constraint.
    ///
    /// Examples:
    /// - `"ISO 17442"` for LEI validation
    /// - `"ISO 4217"` for currency handling
    /// - `"ITAA 1997 Div 355"` for AU R&D eligibility
    fn iso_standard_ids(&self) -> Vec<String>;
}

/// A helper that bridges `Satisfies<T>` results with the evidence layer.
///
/// Domain types implementing both `Satisfies<C>` and `Stereotyped` can use
/// this to produce a `SatisfiesResult` while also recording:
/// - NS-9: `record_is_a(subject, stereotype)` for ontological provenance
/// - NS-10: `record_audited_by(subject, iso_standard)` for compliance
pub struct EvidenceBridge;

impl EvidenceBridge {
    /// Evaluate `satisfies()` and produce the stereotype + ISO labels needed
    /// for evidence recording, without actually calling the evidence layer.
    ///
    /// Returns `(SatisfiesResult, ufo_stereotype, iso_standard_ids)` so the
    /// caller can pass them to `record_is_a()` and `record_audited_by()`.
    pub fn evaluate<E, C>(
        entity: &E,
        constraint: &C,
    ) -> (SatisfiesResult, UfoStereotype, Vec<String>)
    where
        E: Satisfies<C> + Stereotyped,
        C: IsoAuditable,
    {
        let result = entity.satisfies(constraint);
        let stereotype = entity.ufo_stereotype();
        let iso_ids = constraint.iso_standard_ids();
        (result, stereotype, iso_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test domain types ─────────────────────────────────────────────────

    struct AlwaysSatisfied;
    impl Constraint for AlwaysSatisfied {}

    struct AlwaysViolated;
    impl Constraint for AlwaysViolated {}

    struct Subject;
    impl Satisfies<AlwaysSatisfied> for Subject {
        fn satisfies(&self, _: &AlwaysSatisfied) -> SatisfiesResult {
            SatisfiesResult::satisfied(1.0, vec![NodeId::new("doc:abc")])
        }
    }
    impl Satisfies<AlwaysViolated> for Subject {
        fn satisfies(&self, _: &AlwaysViolated) -> SatisfiesResult {
            SatisfiesResult::violated("intentionally violated")
        }
    }

    #[derive(Debug)]
    struct LeiRequired;
    impl IsoAuditable for LeiRequired {
        fn iso_standard_ids(&self) -> Vec<String> {
            vec!["ISO 17442".into()]
        }
    }

    #[derive(Debug)]
    struct TestCompany {
        has_valid_lei: bool,
    }
    impl Satisfies<LeiRequired> for TestCompany {
        fn satisfies(&self, _c: &LeiRequired) -> SatisfiesResult {
            if self.has_valid_lei {
                SatisfiesResult::satisfied(0.99, vec![])
            } else {
                SatisfiesResult::violated("Missing or invalid LEI")
            }
        }
    }
    impl Stereotyped for TestCompany {
        fn ufo_stereotype(&self) -> UfoStereotype {
            UfoStereotype::Kind("Company".into())
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn satisfied_result() {
        let r = Subject.satisfies(&AlwaysSatisfied);
        assert!(r.disposition.is_satisfied());
        assert_eq!(r.confidence, 1.0);
        assert_eq!(r.evidence_nodes.len(), 1);
    }

    #[test]
    fn violated_result() {
        let r = Subject.satisfies(&AlwaysViolated);
        assert!(!r.disposition.is_satisfied());
        assert_eq!(r.confidence, 0.0);
        assert!(matches!(r.disposition, Disposition::Violated { .. }));
    }

    #[test]
    fn unknown_result_roundtrip() {
        let r = SatisfiesResult::unknown();
        let json = serde_json::to_string(&r).unwrap();
        let back: SatisfiesResult = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.disposition, Disposition::Unknown));
    }

    #[test]
    fn disposition_display() {
        assert_eq!(Disposition::Satisfied.to_string(), "Satisfied");
        assert_eq!(
            Disposition::Violated {
                reason: "no LEI".into()
            }
            .to_string(),
            "Violated(no LEI)"
        );
        assert_eq!(Disposition::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn disposition_internally_tagged_wire_format() {
        // ledger-core's already-deployed shape: {"type":"satisfied"} not "Satisfied".
        let json = serde_json::to_string(&Disposition::Satisfied).unwrap();
        assert_eq!(json, r#"{"type":"satisfied"}"#);
        let json = serde_json::to_string(&Disposition::Violated { reason: "x".into() }).unwrap();
        assert_eq!(json, r#"{"type":"violated","reason":"x"}"#);
    }

    #[test]
    fn satisfies_result_with_evidence() {
        let nodes = vec![NodeId::new("abc123"), NodeId::new("def456")];
        let r = SatisfiesResult::satisfied(0.8, vec![]).with_evidence(nodes.clone());
        assert_eq!(r.evidence_nodes, nodes);
    }

    #[test]
    fn node_id_roundtrips() {
        let n = NodeId::new("abc123def456");
        let json = serde_json::to_string(&n).unwrap();
        let back: NodeId = serde_json::from_str(&json).unwrap();
        assert_eq!(n, back);
        assert_eq!(n.as_str(), "abc123def456");
    }

    #[test]
    fn node_id_display_is_bare() {
        let n = NodeId::new("doc:abc123");
        assert_eq!(n.to_string(), "doc:abc123");
    }

    #[test]
    fn node_id_from_str() {
        let n: NodeId = "test".into();
        assert_eq!(n.as_str(), "test");
    }

    #[test]
    fn evidence_bridge_produces_labels() {
        let company = TestCompany {
            has_valid_lei: true,
        };
        let constraint = LeiRequired;
        let (result, stereotype, iso_ids) = EvidenceBridge::evaluate(&company, &constraint);
        assert!(result.is_satisfied());
        assert_eq!(stereotype.to_string(), "Kind:Company");
        assert_eq!(iso_ids, vec!["ISO 17442"]);
    }
}
