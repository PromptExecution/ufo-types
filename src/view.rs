//! Standard SysML v2 view-definition kinds — **SysML v2 only**.
//!
//! [`SysmlViewKind`] is a **closed**, data-level enum of the standard *view*
//! kinds a SysML v2 model can expose. Each variant names a `ViewDefinition`
//! kind drawn from the normative `Views` package of the SysML v2 standard
//! library (`Systems-Modeling/SysML-v2-Release`, `sysml.library`), i.e. the
//! `view def` / rendering vocabulary the language itself defines for
//! presenting a model.
//!
//! This is **NOT** the SysML 1.x nine-diagram taxonomy (block-definition
//! diagram, internal-block diagram, activity, sequence, state-machine, use-case,
//! package, requirement, parametric). SysML v2 is a ground-up KerML-based
//! language, **not a UML 2 derivative**, so there is deliberately:
//!
//! - **no `UmlRelation`** (or any UML-metamodel relationship enum), and
//! - **no behavior-diagram vs. structure-diagram grouping**.
//!
//! In SysML v2 a "diagram" is just a rendering of a `ViewDefinition`; the
//! kinds below are those views, nothing more. Any earlier `DiagramKind` /
//! `BehaviorDiagram` / `StructureDiagram` / `UmlRelation` taxonomy is SysML
//! 1.x thinking and is not represented here.
//!
//! # Reference
//!
//! - `Systems-Modeling/SysML-v2-Release`, `sysml.library` — the normative
//!   `Views` package and the SysML v2 specification's graphical-notation
//!   clause, which enumerate these standard view kinds.
//! - Eclipse SysON implements four of them today (see
//!   [`SysmlViewKind::implemented_by_syson`]).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A standard SysML v2 view-definition kind.
///
/// Closed by design: these mirror the fixed set of standard view kinds the
/// SysML v2 language defines. A model-specific custom view is not one of
/// these and is out of scope for this enum.
///
/// The serde wire form is `snake_case` (matching [`crate::UfoCategory`]);
/// [`SysmlViewKind::view_def_name`] gives the `sysml.library` PascalCase
/// identifier (e.g. `"InterconnectionView"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SysmlViewKind {
    /// Containment / specialization tree of the exposed elements.
    Tree,
    /// General-purpose graphical view (the default SysML v2 graphical view).
    General,
    /// Structure view: parts and the connections between them.
    Interconnection,
    /// Behavior view: action usages and the flow of control/data between them.
    ActionFlow,
    /// Behavior view: states and the transitions between them.
    StateTransition,
    /// Behavior view: lifelines and the time-ordered messages between them.
    Sequence,
    /// Use-case view: the subject, its use cases, and the actors involved.
    Case,
    /// Spatial / geometric view of the modeled system.
    Geometry,
    /// Tabular grid view of exposed elements and their features.
    Grid,
    /// Model-browser / repository-explorer view.
    Browser,
}

impl SysmlViewKind {
    /// Every variant, in declaration order.
    pub const ALL: &'static [SysmlViewKind] = &[
        SysmlViewKind::Tree,
        SysmlViewKind::General,
        SysmlViewKind::Interconnection,
        SysmlViewKind::ActionFlow,
        SysmlViewKind::StateTransition,
        SysmlViewKind::Sequence,
        SysmlViewKind::Case,
        SysmlViewKind::Geometry,
        SysmlViewKind::Grid,
        SysmlViewKind::Browser,
    ];

    /// The `sysml.library` `ViewDefinition` identifier for this kind, e.g.
    /// [`SysmlViewKind::Interconnection`] → `"InterconnectionView"`.
    pub const fn view_def_name(self) -> &'static str {
        match self {
            SysmlViewKind::Tree => "TreeView",
            SysmlViewKind::General => "GeneralView",
            SysmlViewKind::Interconnection => "InterconnectionView",
            SysmlViewKind::ActionFlow => "ActionFlowView",
            SysmlViewKind::StateTransition => "StateTransitionView",
            SysmlViewKind::Sequence => "SequenceView",
            SysmlViewKind::Case => "CaseView",
            SysmlViewKind::Geometry => "GeometryView",
            SysmlViewKind::Grid => "GridView",
            SysmlViewKind::Browser => "BrowserView",
        }
    }

    /// The serde/`snake_case` wire name for this kind, e.g.
    /// [`SysmlViewKind::ActionFlow`] → `"action_flow"`.
    const fn serde_name(self) -> &'static str {
        match self {
            SysmlViewKind::Tree => "tree",
            SysmlViewKind::General => "general",
            SysmlViewKind::Interconnection => "interconnection",
            SysmlViewKind::ActionFlow => "action_flow",
            SysmlViewKind::StateTransition => "state_transition",
            SysmlViewKind::Sequence => "sequence",
            SysmlViewKind::Case => "case",
            SysmlViewKind::Geometry => "geometry",
            SysmlViewKind::Grid => "grid",
            SysmlViewKind::Browser => "browser",
        }
    }

    /// Whether Eclipse SysON ships a diagram implementation for this view
    /// kind today.
    ///
    /// `true` only for [`SysmlViewKind::General`],
    /// [`SysmlViewKind::Interconnection`], [`SysmlViewKind::ActionFlow`] and
    /// [`SysmlViewKind::StateTransition`] — SysON's four
    /// `*ViewDiagramDescriptionProvider`s. The remaining kinds are defined by
    /// the language but have no SysON diagram provider yet.
    pub const fn implemented_by_syson(self) -> bool {
        matches!(
            self,
            SysmlViewKind::General
                | SysmlViewKind::Interconnection
                | SysmlViewKind::ActionFlow
                | SysmlViewKind::StateTransition
        )
    }
}

/// Error returned when a string does not name a [`SysmlViewKind`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "unknown SysML v2 view kind {input:?}; expected one of the snake_case serde names \
     or a `sysml.library` view-def identifier (e.g. \"interconnection\" / \"InterconnectionView\")"
)]
pub struct ParseSysmlViewKindError {
    /// The input string that failed to parse.
    pub input: String,
}

impl std::str::FromStr for SysmlViewKind {
    type Err = ParseSysmlViewKindError;

    /// Case- and whitespace-insensitive. Accepts both the `snake_case` serde
    /// name (`"action_flow"`) and the `sysml.library` view-def identifier
    /// (`"ActionFlowView"`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let norm: String = s
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect();
        Self::ALL
            .iter()
            .copied()
            .find(|v| norm == v.serde_name() || norm == v.view_def_name().to_lowercase())
            .ok_or(ParseSysmlViewKindError {
                input: s.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn serde_round_trips_for_every_variant() {
        for &kind in SysmlViewKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            let back: SysmlViewKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back, "round-trip failed for {kind:?} via {json}");
        }
    }

    #[test]
    fn serde_wire_form_is_snake_case() {
        assert_eq!(
            serde_json::to_string(&SysmlViewKind::ActionFlow).unwrap(),
            r#""action_flow""#
        );
        assert_eq!(
            serde_json::to_string(&SysmlViewKind::Interconnection).unwrap(),
            r#""interconnection""#
        );
    }

    #[test]
    fn from_str_is_inverse_of_view_def_name() {
        for &kind in SysmlViewKind::ALL {
            assert_eq!(SysmlViewKind::from_str(kind.view_def_name()), Ok(kind));
        }
    }

    #[test]
    fn from_str_is_inverse_of_serde_name() {
        for &kind in SysmlViewKind::ALL {
            let serde_name = serde_json::to_string(&kind)
                .unwrap()
                .trim_matches('"')
                .to_string();
            assert_eq!(SysmlViewKind::from_str(&serde_name), Ok(kind));
        }
    }

    #[test]
    fn from_str_ignores_case_and_whitespace() {
        assert_eq!(
            SysmlViewKind::from_str("  Action Flow View "),
            Ok(SysmlViewKind::ActionFlow)
        );
        assert_eq!(
            SysmlViewKind::from_str("STATE_TRANSITION"),
            Ok(SysmlViewKind::StateTransition)
        );
        assert_eq!(
            SysmlViewKind::from_str("interconnectionview"),
            Ok(SysmlViewKind::Interconnection)
        );
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = SysmlViewKind::from_str("bdd").unwrap_err();
        assert_eq!(err.input, "bdd");
    }

    #[test]
    fn all_covers_every_variant() {
        // Exhaustive match: if a variant is added, this fails to compile
        // unless it is also appended to `ALL`.
        for &kind in SysmlViewKind::ALL {
            match kind {
                SysmlViewKind::Tree
                | SysmlViewKind::General
                | SysmlViewKind::Interconnection
                | SysmlViewKind::ActionFlow
                | SysmlViewKind::StateTransition
                | SysmlViewKind::Sequence
                | SysmlViewKind::Case
                | SysmlViewKind::Geometry
                | SysmlViewKind::Grid
                | SysmlViewKind::Browser => {}
            }
        }
        assert_eq!(SysmlViewKind::ALL.len(), 10);
    }

    #[test]
    fn exactly_four_variants_implemented_by_syson() {
        let n = SysmlViewKind::ALL
            .iter()
            .filter(|v| v.implemented_by_syson())
            .count();
        assert_eq!(n, 4);
        for kind in [
            SysmlViewKind::General,
            SysmlViewKind::Interconnection,
            SysmlViewKind::ActionFlow,
            SysmlViewKind::StateTransition,
        ] {
            assert!(
                kind.implemented_by_syson(),
                "{kind:?} should be SysON-backed"
            );
        }
    }

    #[test]
    fn view_def_names_are_distinct_pascal_case_with_view_suffix() {
        let mut seen = std::collections::HashSet::new();
        for &kind in SysmlViewKind::ALL {
            let name = kind.view_def_name();
            assert!(name.ends_with("View"), "{name} should end with `View`");
            assert!(seen.insert(name), "duplicate view_def_name {name}");
        }
    }
}
