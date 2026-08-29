//! UFO stereotype enums — grounding domain concepts in Unified Foundational Ontology.
//!
//! Based on Guizzardi (2005) UFO-A/B/C, _Ontological Foundations for Structural
//! Conceptual Models_, CTIT PhD Thesis Series No. 05-74. This is the crate's
//! single canonical stereotype model — it covers every endurant/perdurant/moment
//! substereotype from the reference taxonomy (not just the five originally
//! shipped here) while keeping every original variant's shape and `Display`
//! output unchanged, so existing callers (`dare`, `capability`, and downstream
//! consumers) don't need to change.
//!
//! # Usage
//! ```ignore
//! use ufo_types::stereotype::UfoStereotype;
//! let k = UfoStereotype::Kind("Company".into());
//! assert_eq!(k.as_str(), "Kind:Company");
//! assert_eq!(k.category(), ufo_types::stereotype::UfoCategory::Endurant);
//! record_is_a("company:5493001K", &k.to_string());
//! ```

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Top-level UFO ontological category (Guizzardi 2005, §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UfoCategory {
    /// Endurants: entities that exist wholly at each moment of time (individuals, universals).
    Endurant,
    /// Perdurants: entities that are "spread out" in time (events, processes).
    Perdurant,
    /// Moments: entities that are inherent to, or mediate between, other entities.
    Moment,
    /// Abstract: mathematical, logical, or formal entities with no spatio-temporal location.
    Abstract,
}

impl std::fmt::Display for UfoCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UfoCategory::Endurant => write!(f, "Endurant"),
            UfoCategory::Perdurant => write!(f, "Perdurant"),
            UfoCategory::Moment => write!(f, "Moment"),
            UfoCategory::Abstract => write!(f, "Abstract"),
        }
    }
}

/// UFO stereotype — classifies a domain entity according to Guizzardi's
/// Unified Foundational Ontology (UFO-A/B/C).
///
/// | Variant     | Category  | Rigidity   | Example                          |
/// |-------------|-----------|------------|-----------------------------------|
/// | `Kind`      | Endurant  | rigid      | Person, Company, Transaction     |
/// | `SubKind`   | Endurant  | rigid      | PtyLtd ⊆ Company, Sell ⊆ Trade   |
/// | `Role`      | Endurant  | anti-rigid | TaxCreditClaimant, RndConductor  |
/// | `Phase`     | Endurant  | anti-rigid | Draft ⊆ Document (phase of life) |
/// | `Category`  | Endurant  | rigid, non-sortal | PhysicalObject             |
/// | `Mixin`     | Endurant  | mixed, non-sortal | Insurable                  |
/// | `RoleMixin` | Endurant  | anti-rigid, non-sortal | Customer               |
/// | `Process`   | Perdurant | —          | ReviewProcess                    |
/// | `State`     | Perdurant | —          | Draft, UnderReview                |
/// | `Event`     | Perdurant | —          | DocumentSubmitted                 |
/// | `Scenario`  | Perdurant | —          | AuditEngagement                   |
/// | `Relator`   | Moment    | dependent  | Evidence, Proof, ConstraintCheck |
/// | `Mode`      | Moment    | inherent   | Eligibility, Compliance, Satisfied|
/// | `Abstract`  | Abstract  | —          | a purely formal/mathematical value|
///
/// All variants implement `Display` so they can be passed directly to
/// `record_is_a(datum_key, stereotype)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
pub enum UfoStereotype {
    /// UFO-A Endurant: rigid, essential type. An entity cannot cease to be a
    /// `Kind` without losing its identity. (Guizzardi 2005, §4.2.1)
    Kind(String),

    /// UFO-A Endurant: rigid subtype that inherits identity from its parent
    /// `Kind`. The extension of a `SubKind` is always a subset of its parent.
    /// (Guizzardi 2005, §4.2.2)
    SubKind {
        /// Name of this sub-kind
        name: String,
        /// Name of the parent Kind this sub-kind specialises
        parent: String,
    },

    /// UFO-A Endurant: anti-rigid, relationally dependent type. An entity
    /// can gain or lose a `Role` without changing its identity.
    /// (Guizzardi 2005, §4.3.1)
    Role(String),

    /// UFO-A Endurant: anti-rigid sortal whose instances change membership
    /// dynamically over time while keeping the same identity principle as
    /// their governing `Kind`. (Guizzardi 2005, §4.3.2)
    Phase(String),

    /// UFO-A Endurant: rigid, non-sortal universal that collects instances
    /// from multiple `Kind`s without supplying its own identity principle.
    /// (Guizzardi 2005, §4.4)
    Category(String),

    /// UFO-A Endurant: non-sortal universal contributed by multiple `Kind`s,
    /// mixing both rigid and anti-rigid properties. (Guizzardi 2005, §4.4)
    Mixin(String),

    /// UFO-A Endurant: non-sortal mixin that is also role-constrained.
    /// (Guizzardi 2005, §4.4)
    RoleMixin(String),

    /// UFO-B Perdurant: ongoing temporal entity with internal causal
    /// structure. (Guizzardi 2005, §6)
    Process(String),

    /// UFO-B Perdurant: temporally extended but homogeneous condition.
    /// (Guizzardi 2005, §6)
    State(String),

    /// UFO-B Perdurant: punctual change of state; atomic perdurant.
    /// (Guizzardi 2005, §6)
    Event(String),

    /// UFO-B Perdurant: complex perdurant composed of heterogeneous
    /// sub-parts in temporal order. (Guizzardi 2005, §6)
    Scenario(String),

    /// UFO-B Moment: mediates between two or more entities. A `Relator` is
    /// existentially dependent on its relata — if the entities it connects
    /// cease to exist, the relator ceases to exist.
    /// (Guizzardi 2005, §5.2)
    Relator(String),

    /// UFO-B Moment: intrinsic property that inheres in a single entity.
    /// Unlike `Relator` (which mediates between entities), `Mode` is a
    /// quality or state of a single bearer.
    /// (Guizzardi 2005, §5.1)
    Mode(String),

    /// Mathematical, logical, or formal entity with no spatio-temporal
    /// location (Guizzardi 2005, §3) — e.g. a number, a proposition, a
    /// purely formal constraint.
    Abstract(String),
}

impl UfoStereotype {
    /// Return the top-level UFO category this stereotype belongs to.
    pub fn category(&self) -> UfoCategory {
        use UfoStereotype::*;
        match self {
            Kind(_)
            | SubKind { .. }
            | Role(_)
            | Phase(_)
            | Category(_)
            | Mixin(_)
            | RoleMixin(_) => UfoCategory::Endurant,
            Process(_) | State(_) | Event(_) | Scenario(_) => UfoCategory::Perdurant,
            Relator(_) | Mode(_) => UfoCategory::Moment,
            Abstract(_) => UfoCategory::Abstract,
        }
    }

    /// Return the canonical string label for evidence logging.
    ///
    /// Prefer `Display`/`to_string()` — this exists only for callers that
    /// need a `&str` reference without allocating; use `to_label()` in new
    /// code.
    pub fn as_str(&self) -> &str {
        match self {
            UfoStereotype::Kind(_)
            | UfoStereotype::Role(_)
            | UfoStereotype::Phase(_)
            | UfoStereotype::Category(_)
            | UfoStereotype::Mixin(_)
            | UfoStereotype::RoleMixin(_)
            | UfoStereotype::Process(_)
            | UfoStereotype::State(_)
            | UfoStereotype::Event(_)
            | UfoStereotype::Scenario(_)
            | UfoStereotype::Relator(_)
            | UfoStereotype::Mode(_)
            | UfoStereotype::Abstract(_) => "",
            UfoStereotype::SubKind { .. } => "",
        }
    }

    /// Convert to an owned string suitable for `record_is_a()`.
    pub fn to_label(&self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for UfoStereotype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UfoStereotype::Kind(name) => write!(f, "Kind:{name}"),
            UfoStereotype::SubKind { name, parent } => write!(f, "SubKind:{name}<{parent}"),
            UfoStereotype::Role(name) => write!(f, "Role:{name}"),
            UfoStereotype::Phase(name) => write!(f, "Phase:{name}"),
            UfoStereotype::Category(name) => write!(f, "Category:{name}"),
            UfoStereotype::Mixin(name) => write!(f, "Mixin:{name}"),
            UfoStereotype::RoleMixin(name) => write!(f, "RoleMixin:{name}"),
            UfoStereotype::Process(name) => write!(f, "Process:{name}"),
            UfoStereotype::State(name) => write!(f, "State:{name}"),
            UfoStereotype::Event(name) => write!(f, "Event:{name}"),
            UfoStereotype::Scenario(name) => write!(f, "Scenario:{name}"),
            UfoStereotype::Relator(name) => write!(f, "Relator:{name}"),
            UfoStereotype::Mode(name) => write!(f, "Mode:{name}"),
            UfoStereotype::Abstract(name) => write!(f, "Abstract:{name}"),
        }
    }
}

/// A domain type that carries a UFO stereotype — used by `record_is_a` (NS-9)
/// to emit ontological provenance evidence.
///
/// # Example
/// ```ignore
/// impl Stereotyped for AuRdActivity {
///     fn ufo_stereotype(&self) -> UfoStereotype {
///         UfoStereotype::SubKind {
///             name: "AuRdActivity".into(),
///             parent: "Activity".into(),
///         }
///     }
/// }
/// ```
pub trait Stereotyped {
    /// The UFO stereotype for this domain type.
    fn ufo_stereotype(&self) -> UfoStereotype;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_display() {
        let k = UfoStereotype::Kind("Company".into());
        assert_eq!(k.to_string(), "Kind:Company");
    }

    #[test]
    fn subkind_display() {
        let sk = UfoStereotype::SubKind {
            name: "PtyLtd".into(),
            parent: "Company".into(),
        };
        assert_eq!(sk.to_string(), "SubKind:PtyLtd<Company");
    }

    #[test]
    fn role_display() {
        let r = UfoStereotype::Role("TaxCreditClaimant".into());
        assert_eq!(r.to_string(), "Role:TaxCreditClaimant");
    }

    #[test]
    fn relator_display() {
        let r = UfoStereotype::Relator("Evidence".into());
        assert_eq!(r.to_string(), "Relator:Evidence");
    }

    #[test]
    fn mode_display() {
        let m = UfoStereotype::Mode("Eligibility".into());
        assert_eq!(m.to_string(), "Mode:Eligibility");
    }

    #[test]
    fn new_endurant_substereotypes_display() {
        assert_eq!(
            UfoStereotype::Phase("Draft".into()).to_string(),
            "Phase:Draft"
        );
        assert_eq!(
            UfoStereotype::Category("PhysicalObject".into()).to_string(),
            "Category:PhysicalObject"
        );
        assert_eq!(
            UfoStereotype::Mixin("Insurable".into()).to_string(),
            "Mixin:Insurable"
        );
        assert_eq!(
            UfoStereotype::RoleMixin("Customer".into()).to_string(),
            "RoleMixin:Customer"
        );
    }

    #[test]
    fn perdurant_substereotypes_display() {
        assert_eq!(
            UfoStereotype::Process("Review".into()).to_string(),
            "Process:Review"
        );
        assert_eq!(
            UfoStereotype::State("UnderReview".into()).to_string(),
            "State:UnderReview"
        );
        assert_eq!(
            UfoStereotype::Event("Submitted".into()).to_string(),
            "Event:Submitted"
        );
        assert_eq!(
            UfoStereotype::Scenario("Audit".into()).to_string(),
            "Scenario:Audit"
        );
    }

    #[test]
    fn abstract_display() {
        assert_eq!(
            UfoStereotype::Abstract("Pi".into()).to_string(),
            "Abstract:Pi"
        );
    }

    #[test]
    fn category_classifies_every_variant_correctly() {
        assert_eq!(
            UfoStereotype::Kind("x".into()).category(),
            UfoCategory::Endurant
        );
        assert_eq!(
            UfoStereotype::SubKind {
                name: "x".into(),
                parent: "y".into()
            }
            .category(),
            UfoCategory::Endurant
        );
        assert_eq!(
            UfoStereotype::Role("x".into()).category(),
            UfoCategory::Endurant
        );
        assert_eq!(
            UfoStereotype::Phase("x".into()).category(),
            UfoCategory::Endurant
        );
        assert_eq!(
            UfoStereotype::Category("x".into()).category(),
            UfoCategory::Endurant
        );
        assert_eq!(
            UfoStereotype::Mixin("x".into()).category(),
            UfoCategory::Endurant
        );
        assert_eq!(
            UfoStereotype::RoleMixin("x".into()).category(),
            UfoCategory::Endurant
        );
        assert_eq!(
            UfoStereotype::Process("x".into()).category(),
            UfoCategory::Perdurant
        );
        assert_eq!(
            UfoStereotype::State("x".into()).category(),
            UfoCategory::Perdurant
        );
        assert_eq!(
            UfoStereotype::Event("x".into()).category(),
            UfoCategory::Perdurant
        );
        assert_eq!(
            UfoStereotype::Scenario("x".into()).category(),
            UfoCategory::Perdurant
        );
        assert_eq!(
            UfoStereotype::Relator("x".into()).category(),
            UfoCategory::Moment
        );
        assert_eq!(
            UfoStereotype::Mode("x".into()).category(),
            UfoCategory::Moment
        );
        assert_eq!(
            UfoStereotype::Abstract("x".into()).category(),
            UfoCategory::Abstract
        );
    }

    #[test]
    fn ufo_category_roundtrip() {
        for cat in [
            UfoCategory::Endurant,
            UfoCategory::Perdurant,
            UfoCategory::Moment,
            UfoCategory::Abstract,
        ] {
            let json = serde_json::to_string(&cat).unwrap();
            let back: UfoCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(cat, back);
        }
    }

    #[test]
    fn stereotype_roundtrips_json() {
        let stereotypes = vec![
            UfoStereotype::Kind("Company".into()),
            UfoStereotype::SubKind {
                name: "PtyLtd".into(),
                parent: "Company".into(),
            },
            UfoStereotype::Role("Claimant".into()),
            UfoStereotype::Relator("Proof".into()),
            UfoStereotype::Mode("Eligibility".into()),
            UfoStereotype::Phase("Draft".into()),
            UfoStereotype::Process("Review".into()),
            UfoStereotype::Abstract("Pi".into()),
        ];
        for s in &stereotypes {
            let json = serde_json::to_string(s).unwrap();
            let back: UfoStereotype = serde_json::from_str(&json).unwrap();
            assert_eq!(s, &back);
        }
    }

    #[test]
    fn stereotyped_trait_can_be_implemented() {
        struct TestEntity;
        impl Stereotyped for TestEntity {
            fn ufo_stereotype(&self) -> UfoStereotype {
                UfoStereotype::Kind("TestEntity".into())
            }
        }
        assert_eq!(TestEntity.ufo_stereotype().to_string(), "Kind:TestEntity");
    }
}
