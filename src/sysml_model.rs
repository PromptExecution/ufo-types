//! Closed, data-level SysML **v2** / KerML abstract-syntax types — **SysML v2 only**.
//!
//! This module is the typed classification layer that sits **above**
//! [`crate::iso_ir`]'s free-form `Node`/`Edge` transport floor and **below**
//! any renderer. It is **pure data**: plain enums and struct-variant sum
//! types, no traits, and deliberately **no sealed-trait viewpoint machinery**
//! (`BehavioralView` / `StructuralView` marker hierarchies, per-type
//! `accepts(element)` — all explicitly rejected; adding a viewpoint MUST be a
//! data change, not a recompile).
//!
//! It is the SysML v2 / KerML replacement for the rejected SysML 1.x
//! `DiagramKind` / `BehaviorDiagram` / `StructureDiagram` / `UmlRelation`
//! taxonomy. SysML v2 is a ground-up KerML-based language, **not a UML 2
//! derivative**, so there is deliberately no relationship-provenance enum
//! (`SameAsUml2` / `NewInSysml` and friends have nothing to compare against)
//! and no behavior-vs-structure grouping.
//!
//! Domain concepts (`Mission`, `Protocol`, `Agent`, `MCPServer`, …) are **NOT**
//! members of [`ElementKind`]. They stay [`crate::iso_ir::Node`] `part_type`
//! strings, or a downstream enum owned by the consuming crate — putting them
//! here would re-introduce exactly the domain↔SysML coupling `iso_ir` was
//! split out to remove.
//!
//! [`ElementKind`] and [`Relation`] are `#[non_exhaustive]`: the KerML /
//! SysML v2 abstract syntax is a fixed, spec-governed set, but that spec can
//! gain members, and consumers must not assume today's list is final.
//!
//! # Reference
//!
//! - `PromptExecution/kr0ki` — `docs/DESIGN-NOTE-typed-model-layer.md`
//!   §2.1 (`ElementKind`), §2.2 (`Relation`), §2.4 (identity / `ElementId`).
//! - OMG SysML v2 Beta / KerML specifications — the normative abstract syntax.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An opaque element identifier.
///
/// MAY be a human-meaningful name (`"bus-3"`, `"agent.foo"`), a KerML
/// qualified name (`Pkg::Sub::Item`), or a content / VCS hash
/// (`sha256:…`, `blake3:…`, `git:<blob-sha>`) where an element needs a stable derived
/// identity. It is **never** a numeric id — a random integer breaks the
/// byte-identical, git-diffable artifact model; a derived one is just a less
/// legible string (kr0ki `DESIGN-NOTE-typed-model-layer.md` §2.4).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ElementId(pub String);

impl ElementId {
    /// Wrap an already-formed id string.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Borrow the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The recognized content-hash / VCS scheme prefixes an `ElementId` may carry.
    pub const HASH_SCHEMES: &'static [&'static str] = &["sha256:", "blake3:", "git:"];

    /// If this id is content-addressed, the scheme prefix it uses
    /// (`"sha256:"`, `"blake3:"`, `"git:"`), else `None`.
    pub fn content_hash_scheme(&self) -> Option<&'static str> {
        Self::HASH_SCHEMES
            .iter()
            .copied()
            .find(|p| self.0.starts_with(p))
    }

    /// Whether this id is a content / VCS hash rather than a name.
    pub fn is_content_addressed(&self) -> bool {
        self.content_hash_scheme().is_some()
    }
}

impl std::fmt::Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for ElementId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ElementId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for ElementId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A KerML / SysML v2 abstract-syntax element kind.
///
/// Closed (modulo `#[non_exhaustive]`) and def/usage-paired: the KerML +
/// SysML v2 abstract syntax is a fixed, spec-governed set. Not every kind in
/// this set has both a definition and a usage member here — `Package` is a
/// pure namespace, and `AttributeUsage` / `ConnectionUsage` /
/// `AllocationUsage` / `RenderingUsage` / `ViewpointDefinition` are carried
/// without their unlisted counterpart. Use [`ElementKind::definition_of`] /
/// [`ElementKind::usage_of`] to walk the pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ElementKind {
    /// A namespace container. Neither a definition nor a usage.
    Package,
    /// `part def` — a definition of a system part.
    PartDefinition,
    /// `part` — a usage of a [`ElementKind::PartDefinition`].
    PartUsage,
    /// `attribute` — a usage of a value-typed feature.
    AttributeUsage,
    /// `port def` — a definition of an interaction point.
    PortDefinition,
    /// `port` — a usage of a [`ElementKind::PortDefinition`].
    PortUsage,
    /// `connection` — a usage connecting two or more ends.
    ConnectionUsage,
    /// `interface def` — a definition of a connectable interface.
    InterfaceDefinition,
    /// `interface` — a usage of an [`ElementKind::InterfaceDefinition`].
    InterfaceUsage,
    /// `item def` — a definition of a flowing/stored item.
    ItemDefinition,
    /// `item` — a usage of an [`ElementKind::ItemDefinition`].
    ItemUsage,
    /// `action def` — a definition of behavior.
    ActionDefinition,
    /// `action` — a usage of an [`ElementKind::ActionDefinition`].
    ActionUsage,
    /// `state def` — a definition of a state.
    StateDefinition,
    /// `state` — a usage of a [`ElementKind::StateDefinition`].
    StateUsage,
    /// `requirement def` — a definition of a requirement.
    RequirementDefinition,
    /// `requirement` — a usage of a [`ElementKind::RequirementDefinition`].
    RequirementUsage,
    /// `constraint def` — a definition of a boolean constraint.
    ConstraintDefinition,
    /// `constraint` — a usage of a [`ElementKind::ConstraintDefinition`].
    ConstraintUsage,
    /// `allocation` — a usage allocating one element to another.
    AllocationUsage,
    /// `view def` — a definition of a view (an `Expose` query + rendering).
    ViewDefinition,
    /// `view` — a usage of a [`ElementKind::ViewDefinition`].
    ViewUsage,
    /// `viewpoint def` — a definition of a stakeholder concern + conformance
    /// constraints.
    ViewpointDefinition,
    /// `render` — a usage of a rendering.
    RenderingUsage,
}

impl ElementKind {
    /// Every variant, in declaration order.
    pub const ALL: &'static [ElementKind] = &[
        ElementKind::Package,
        ElementKind::PartDefinition,
        ElementKind::PartUsage,
        ElementKind::AttributeUsage,
        ElementKind::PortDefinition,
        ElementKind::PortUsage,
        ElementKind::ConnectionUsage,
        ElementKind::InterfaceDefinition,
        ElementKind::InterfaceUsage,
        ElementKind::ItemDefinition,
        ElementKind::ItemUsage,
        ElementKind::ActionDefinition,
        ElementKind::ActionUsage,
        ElementKind::StateDefinition,
        ElementKind::StateUsage,
        ElementKind::RequirementDefinition,
        ElementKind::RequirementUsage,
        ElementKind::ConstraintDefinition,
        ElementKind::ConstraintUsage,
        ElementKind::AllocationUsage,
        ElementKind::ViewDefinition,
        ElementKind::ViewUsage,
        ElementKind::ViewpointDefinition,
        ElementKind::RenderingUsage,
    ];

    /// The KerML / SysML v2 abstract-syntax metaclass name, e.g.
    /// [`ElementKind::PartUsage`] → `"PartUsage"`.
    pub const fn kerml_name(self) -> &'static str {
        match self {
            ElementKind::Package => "Package",
            ElementKind::PartDefinition => "PartDefinition",
            ElementKind::PartUsage => "PartUsage",
            ElementKind::AttributeUsage => "AttributeUsage",
            ElementKind::PortDefinition => "PortDefinition",
            ElementKind::PortUsage => "PortUsage",
            ElementKind::ConnectionUsage => "ConnectionUsage",
            ElementKind::InterfaceDefinition => "InterfaceDefinition",
            ElementKind::InterfaceUsage => "InterfaceUsage",
            ElementKind::ItemDefinition => "ItemDefinition",
            ElementKind::ItemUsage => "ItemUsage",
            ElementKind::ActionDefinition => "ActionDefinition",
            ElementKind::ActionUsage => "ActionUsage",
            ElementKind::StateDefinition => "StateDefinition",
            ElementKind::StateUsage => "StateUsage",
            ElementKind::RequirementDefinition => "RequirementDefinition",
            ElementKind::RequirementUsage => "RequirementUsage",
            ElementKind::ConstraintDefinition => "ConstraintDefinition",
            ElementKind::ConstraintUsage => "ConstraintUsage",
            ElementKind::AllocationUsage => "AllocationUsage",
            ElementKind::ViewDefinition => "ViewDefinition",
            ElementKind::ViewUsage => "ViewUsage",
            ElementKind::ViewpointDefinition => "ViewpointDefinition",
            ElementKind::RenderingUsage => "RenderingUsage",
        }
    }

    /// The serde / `snake_case` wire name, e.g.
    /// [`ElementKind::PartUsage`] → `"part_usage"`.
    const fn serde_name(self) -> &'static str {
        match self {
            ElementKind::Package => "package",
            ElementKind::PartDefinition => "part_definition",
            ElementKind::PartUsage => "part_usage",
            ElementKind::AttributeUsage => "attribute_usage",
            ElementKind::PortDefinition => "port_definition",
            ElementKind::PortUsage => "port_usage",
            ElementKind::ConnectionUsage => "connection_usage",
            ElementKind::InterfaceDefinition => "interface_definition",
            ElementKind::InterfaceUsage => "interface_usage",
            ElementKind::ItemDefinition => "item_definition",
            ElementKind::ItemUsage => "item_usage",
            ElementKind::ActionDefinition => "action_definition",
            ElementKind::ActionUsage => "action_usage",
            ElementKind::StateDefinition => "state_definition",
            ElementKind::StateUsage => "state_usage",
            ElementKind::RequirementDefinition => "requirement_definition",
            ElementKind::RequirementUsage => "requirement_usage",
            ElementKind::ConstraintDefinition => "constraint_definition",
            ElementKind::ConstraintUsage => "constraint_usage",
            ElementKind::AllocationUsage => "allocation_usage",
            ElementKind::ViewDefinition => "view_definition",
            ElementKind::ViewUsage => "view_usage",
            ElementKind::ViewpointDefinition => "viewpoint_definition",
            ElementKind::RenderingUsage => "rendering_usage",
        }
    }

    /// Whether this kind is a `*Definition` metaclass.
    pub const fn is_definition(self) -> bool {
        matches!(
            self,
            ElementKind::PartDefinition
                | ElementKind::PortDefinition
                | ElementKind::InterfaceDefinition
                | ElementKind::ItemDefinition
                | ElementKind::ActionDefinition
                | ElementKind::StateDefinition
                | ElementKind::RequirementDefinition
                | ElementKind::ConstraintDefinition
                | ElementKind::ViewDefinition
                | ElementKind::ViewpointDefinition
        )
    }

    /// Whether this kind is a `*Usage` metaclass.
    pub const fn is_usage(self) -> bool {
        matches!(
            self,
            ElementKind::PartUsage
                | ElementKind::AttributeUsage
                | ElementKind::PortUsage
                | ElementKind::ConnectionUsage
                | ElementKind::InterfaceUsage
                | ElementKind::ItemUsage
                | ElementKind::ActionUsage
                | ElementKind::StateUsage
                | ElementKind::RequirementUsage
                | ElementKind::ConstraintUsage
                | ElementKind::AllocationUsage
                | ElementKind::ViewUsage
                | ElementKind::RenderingUsage
        )
    }

    /// The paired `*Usage` for a `*Definition`, where both are members of
    /// this enum. `None` for usages, for `Package`, and for
    /// `ViewpointDefinition` (no `ViewpointUsage` member).
    pub const fn usage_of(self) -> Option<ElementKind> {
        Some(match self {
            ElementKind::PartDefinition => ElementKind::PartUsage,
            ElementKind::PortDefinition => ElementKind::PortUsage,
            ElementKind::InterfaceDefinition => ElementKind::InterfaceUsage,
            ElementKind::ItemDefinition => ElementKind::ItemUsage,
            ElementKind::ActionDefinition => ElementKind::ActionUsage,
            ElementKind::StateDefinition => ElementKind::StateUsage,
            ElementKind::RequirementDefinition => ElementKind::RequirementUsage,
            ElementKind::ConstraintDefinition => ElementKind::ConstraintUsage,
            ElementKind::ViewDefinition => ElementKind::ViewUsage,
            _ => return None,
        })
    }

    /// The paired `*Definition` for a `*Usage`, where both are members of
    /// this enum. `None` for definitions, for `Package`, and for usages
    /// whose definition metaclass is not a member (`AttributeUsage`,
    /// `ConnectionUsage`, `AllocationUsage`, `RenderingUsage`).
    pub const fn definition_of(self) -> Option<ElementKind> {
        Some(match self {
            ElementKind::PartUsage => ElementKind::PartDefinition,
            ElementKind::PortUsage => ElementKind::PortDefinition,
            ElementKind::InterfaceUsage => ElementKind::InterfaceDefinition,
            ElementKind::ItemUsage => ElementKind::ItemDefinition,
            ElementKind::ActionUsage => ElementKind::ActionDefinition,
            ElementKind::StateUsage => ElementKind::StateDefinition,
            ElementKind::RequirementUsage => ElementKind::RequirementDefinition,
            ElementKind::ConstraintUsage => ElementKind::ConstraintDefinition,
            ElementKind::ViewUsage => ElementKind::ViewDefinition,
            _ => return None,
        })
    }
}

/// Error returned when a string does not name an [`ElementKind`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "unknown SysML v2 / KerML element kind {input:?}; expected a snake_case serde name \
     or a KerML metaclass name (e.g. \"part_usage\" / \"PartUsage\")"
)]
pub struct ParseElementKindError {
    /// The input string that failed to parse.
    pub input: String,
}

impl std::str::FromStr for ElementKind {
    type Err = ParseElementKindError;

    /// Case- and whitespace-insensitive. Accepts both the `snake_case` serde
    /// name (`"part_usage"`) and the KerML metaclass name (`"PartUsage"`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let norm: String = s
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect();
        Self::ALL
            .iter()
            .copied()
            .find(|k| norm == k.serde_name() || norm == k.kerml_name().to_lowercase())
            .ok_or(ParseElementKindError {
                input: s.to_string(),
            })
    }
}

/// A data-level sum type over the KerML / SysML v2 relationship set, plus a
/// single `Domain` escape hatch.
///
/// Struct variants, **not** traits. `#[non_exhaustive]`: the KerML abstract
/// syntax can gain relationship metaclasses.
///
/// `Domain` is the **only** escape hatch — for edges that are genuinely not a
/// KerML / SysML v2 relationship (a b00t digital-thread "attachment", a
/// pipeline "sequence" that is not a real `Succession`, …). Do not add a
/// free-form `edge_type` anywhere else; anything not expressible here stays
/// at the [`crate::iso_ir`] layer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Relation {
    /// A feature is owned by / a member of an owning element.
    FeatureMembership {
        /// The owning element.
        owner: ElementId,
        /// The owned member.
        member: ElementId,
    },
    /// The specific element specializes (is a subtype of) the general one.
    Specialization {
        /// The specializing (sub) element.
        specific: ElementId,
        /// The generalized (super) element.
        general: ElementId,
    },
    /// The subset feature subsets the superset feature.
    Subsetting {
        /// The subsetting feature.
        subset: ElementId,
        /// The subsetted feature.
        superset: ElementId,
    },
    /// The redefining feature redefines the redefined one.
    Redefinition {
        /// The redefining feature.
        redefining: ElementId,
        /// The redefined feature.
        redefined: ElementId,
    },
    /// A connection between two or more ends.
    Connection {
        /// The connected ends, in order.
        ends: Vec<ElementId>,
    },
    /// v2 control flow — replaces the v1 "Transition". Guard/trigger live on
    /// the `Succession` / `ActionUsage` and MUST NOT be dropped by consumers.
    Succession {
        /// The source of control.
        source: ElementId,
        /// The target of control.
        target: ElementId,
    },
    /// One element is allocated to another.
    Allocation {
        /// The allocated-from element.
        source: ElementId,
        /// The allocated-to element.
        target: ElementId,
    },
    /// A requirement is satisfied by a subject element.
    Satisfy {
        /// The requirement.
        requirement: ElementId,
        /// The satisfying subject.
        subject: ElementId,
    },
    /// A requirement is verified by an element (e.g. a test case).
    Verify {
        /// The requirement.
        requirement: ElementId,
        /// The verifying element.
        by: ElementId,
    },
    /// One element refines another (e.g. a requirement refines a need).
    Refine {
        /// The refined element.
        refined: ElementId,
        /// The refining element.
        refining: ElementId,
    },
    /// A client element depends on a supplier element.
    Dependency {
        /// The dependent client.
        client: ElementId,
        /// The depended-on supplier.
        supplier: ElementId,
    },
    /// Escape hatch: an edge that is not a KerML / SysML v2 relationship.
    Domain {
        /// The source element.
        source: ElementId,
        /// The target element.
        target: ElementId,
        /// A free-form classifier for this non-KerML edge.
        kind: String,
    },
}

impl Relation {
    /// The element endpoints this relation connects, in a stable order.
    ///
    /// Two for every variant except [`Relation::Connection`], which returns
    /// its `ends` verbatim. `Domain`'s `kind` classifier is not an endpoint.
    pub fn endpoints(&self) -> Vec<&ElementId> {
        match self {
            Relation::FeatureMembership { owner, member } => vec![owner, member],
            Relation::Specialization { specific, general } => vec![specific, general],
            Relation::Subsetting { subset, superset } => vec![subset, superset],
            Relation::Redefinition {
                redefining,
                redefined,
            } => vec![redefining, redefined],
            Relation::Connection { ends } => ends.iter().collect(),
            Relation::Succession { source, target } => vec![source, target],
            Relation::Allocation { source, target } => vec![source, target],
            Relation::Satisfy {
                requirement,
                subject,
            } => vec![requirement, subject],
            Relation::Verify { requirement, by } => vec![requirement, by],
            Relation::Refine { refined, refining } => vec![refined, refining],
            Relation::Dependency { client, supplier } => vec![client, supplier],
            Relation::Domain { source, target, .. } => vec![source, target],
        }
    }

    /// The KerML / SysML v2 relationship metaclass name, e.g.
    /// [`Relation::Succession`] → `"Succession"`. [`Relation::Domain`]
    /// returns `"Domain"` — it is the non-KerML escape hatch, named for
    /// symmetry only.
    pub const fn kerml_name(&self) -> &'static str {
        match self {
            Relation::FeatureMembership { .. } => "FeatureMembership",
            Relation::Specialization { .. } => "Specialization",
            Relation::Subsetting { .. } => "Subsetting",
            Relation::Redefinition { .. } => "Redefinition",
            Relation::Connection { .. } => "Connection",
            Relation::Succession { .. } => "Succession",
            Relation::Allocation { .. } => "Allocation",
            Relation::Satisfy { .. } => "Satisfy",
            Relation::Verify { .. } => "Verify",
            Relation::Refine { .. } => "Refine",
            Relation::Dependency { .. } => "Dependency",
            Relation::Domain { .. } => "Domain",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn eid(s: &str) -> ElementId {
        ElementId::from(s)
    }

    #[test]
    fn element_id_conversions_and_display() {
        assert_eq!(ElementId::from("bus-3").to_string(), "bus-3");
        assert_eq!(ElementId::from(String::from("git:abc")).as_str(), "git:abc");
        let id = ElementId::new("Pkg::Item");
        let s: &str = id.as_ref();
        assert_eq!(s, "Pkg::Item");
    }

    #[test]
    fn element_id_round_trips_as_bare_string() {
        let id = eid("sha256:deadbeef");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#""sha256:deadbeef""#);
        assert_eq!(serde_json::from_str::<ElementId>(&json).unwrap(), id);
    }

    #[test]
    fn element_id_content_hash_scheme_detection() {
        assert_eq!(eid("blake3:abc123").content_hash_scheme(), Some("blake3:"));
        assert!(eid("blake3:abc123").is_content_addressed());

        assert_eq!(
            eid("sha256:deadbeef").content_hash_scheme(),
            Some("sha256:")
        );
        assert!(eid("sha256:deadbeef").is_content_addressed());

        assert_eq!(eid("git:deadbeef").content_hash_scheme(), Some("git:"));
        assert!(eid("git:deadbeef").is_content_addressed());

        assert_eq!(eid("bus-3").content_hash_scheme(), None);
        assert!(!eid("bus-3").is_content_addressed());

        assert_eq!(ElementId::HASH_SCHEMES.len(), 3);
    }

    #[test]
    fn element_kind_all_is_complete_and_ordered() {
        // Exhaustive match: a new variant fails to compile until it is also
        // appended to ALL.
        for &k in ElementKind::ALL {
            match k {
                ElementKind::Package
                | ElementKind::PartDefinition
                | ElementKind::PartUsage
                | ElementKind::AttributeUsage
                | ElementKind::PortDefinition
                | ElementKind::PortUsage
                | ElementKind::ConnectionUsage
                | ElementKind::InterfaceDefinition
                | ElementKind::InterfaceUsage
                | ElementKind::ItemDefinition
                | ElementKind::ItemUsage
                | ElementKind::ActionDefinition
                | ElementKind::ActionUsage
                | ElementKind::StateDefinition
                | ElementKind::StateUsage
                | ElementKind::RequirementDefinition
                | ElementKind::RequirementUsage
                | ElementKind::ConstraintDefinition
                | ElementKind::ConstraintUsage
                | ElementKind::AllocationUsage
                | ElementKind::ViewDefinition
                | ElementKind::ViewUsage
                | ElementKind::ViewpointDefinition
                | ElementKind::RenderingUsage => {}
            }
        }
        assert_eq!(ElementKind::ALL.len(), 24);
    }

    #[test]
    fn element_kind_serde_round_trips_for_every_variant() {
        for &k in ElementKind::ALL {
            let json = serde_json::to_string(&k).unwrap();
            let back: ElementKind = serde_json::from_str(&json).unwrap();
            assert_eq!(k, back, "round-trip failed for {k:?} via {json}");
        }
    }

    #[test]
    fn element_kind_def_usage_partition() {
        let defs = ElementKind::ALL
            .iter()
            .filter(|k| k.is_definition())
            .count();
        let usages = ElementKind::ALL.iter().filter(|k| k.is_usage()).count();
        assert_eq!(defs, 10);
        assert_eq!(usages, 13);
        // Package is neither; nothing is both.
        assert_eq!(defs + usages + 1, ElementKind::ALL.len());
        for &k in ElementKind::ALL {
            assert!(!(k.is_definition() && k.is_usage()), "{k:?} is both");
        }
        assert!(!ElementKind::Package.is_definition());
        assert!(!ElementKind::Package.is_usage());
    }

    #[test]
    fn element_kind_pairing_is_symmetric_where_defined() {
        for &k in ElementKind::ALL {
            if let Some(u) = k.usage_of() {
                assert!(k.is_definition());
                assert!(u.is_usage());
                assert_eq!(
                    u.definition_of(),
                    Some(k),
                    "usage_of/definition_of asymmetric for {k:?}"
                );
            }
            if let Some(d) = k.definition_of() {
                assert!(k.is_usage());
                assert!(d.is_definition());
                assert_eq!(
                    d.usage_of(),
                    Some(k),
                    "definition_of/usage_of asymmetric for {k:?}"
                );
            }
        }
        // Spot-check the unpaired members.
        assert_eq!(ElementKind::ViewpointDefinition.usage_of(), None);
        assert_eq!(ElementKind::AttributeUsage.definition_of(), None);
        assert_eq!(ElementKind::ConnectionUsage.definition_of(), None);
        assert_eq!(ElementKind::AllocationUsage.definition_of(), None);
        assert_eq!(ElementKind::RenderingUsage.definition_of(), None);
        assert_eq!(ElementKind::Package.usage_of(), None);
        assert_eq!(ElementKind::Package.definition_of(), None);
        // A defined pair.
        assert_eq!(
            ElementKind::PartDefinition.usage_of(),
            Some(ElementKind::PartUsage)
        );
        assert_eq!(
            ElementKind::PartUsage.definition_of(),
            Some(ElementKind::PartDefinition)
        );
    }

    #[test]
    fn element_kind_from_str_is_inverse_of_both_names() {
        for &k in ElementKind::ALL {
            assert_eq!(ElementKind::from_str(k.kerml_name()), Ok(k));
            let serde_name = serde_json::to_string(&k)
                .unwrap()
                .trim_matches('"')
                .to_string();
            assert_eq!(ElementKind::from_str(&serde_name), Ok(k));
        }
    }

    #[test]
    fn element_kind_from_str_ignores_case_and_whitespace_and_rejects_unknown() {
        assert_eq!(
            ElementKind::from_str("  Part Usage "),
            Ok(ElementKind::PartUsage)
        );
        assert_eq!(
            ElementKind::from_str("VIEWPOINT_DEFINITION"),
            Ok(ElementKind::ViewpointDefinition)
        );
        assert_eq!(
            ElementKind::from_str("mission").unwrap_err().input,
            "mission"
        );
    }

    #[test]
    fn relation_serde_round_trips_for_a_sample_of_each_variant() {
        let samples = vec![
            Relation::FeatureMembership {
                owner: eid("pkg"),
                member: eid("part"),
            },
            Relation::Specialization {
                specific: eid("Car"),
                general: eid("Vehicle"),
            },
            Relation::Subsetting {
                subset: eid("frontWheels"),
                superset: eid("wheels"),
            },
            Relation::Redefinition {
                redefining: eid("mass"),
                redefined: eid("Vehicle::mass"),
            },
            Relation::Connection {
                ends: vec![eid("a"), eid("b"), eid("c")],
            },
            Relation::Succession {
                source: eid("act1"),
                target: eid("act2"),
            },
            Relation::Allocation {
                source: eid("logical"),
                target: eid("physical"),
            },
            Relation::Satisfy {
                requirement: eid("R1"),
                subject: eid("design"),
            },
            Relation::Verify {
                requirement: eid("R1"),
                by: eid("TC1"),
            },
            Relation::Refine {
                refined: eid("need"),
                refining: eid("R1"),
            },
            Relation::Dependency {
                client: eid("A"),
                supplier: eid("B"),
            },
            Relation::Domain {
                source: eid("doc"),
                target: eid("blob"),
                kind: "attachment".to_string(),
            },
        ];
        assert_eq!(samples.len(), 12);
        for r in &samples {
            let json = serde_json::to_string(r).unwrap();
            let back: Relation = serde_json::from_str(&json).unwrap();
            assert_eq!(*r, back, "round-trip failed via {json}");
        }
    }

    #[test]
    fn relation_endpoints_have_the_right_arity() {
        let two = [
            Relation::FeatureMembership {
                owner: eid("a"),
                member: eid("b"),
            },
            Relation::Specialization {
                specific: eid("a"),
                general: eid("b"),
            },
            Relation::Subsetting {
                subset: eid("a"),
                superset: eid("b"),
            },
            Relation::Redefinition {
                redefining: eid("a"),
                redefined: eid("b"),
            },
            Relation::Succession {
                source: eid("a"),
                target: eid("b"),
            },
            Relation::Allocation {
                source: eid("a"),
                target: eid("b"),
            },
            Relation::Satisfy {
                requirement: eid("a"),
                subject: eid("b"),
            },
            Relation::Verify {
                requirement: eid("a"),
                by: eid("b"),
            },
            Relation::Refine {
                refined: eid("a"),
                refining: eid("b"),
            },
            Relation::Dependency {
                client: eid("a"),
                supplier: eid("b"),
            },
            Relation::Domain {
                source: eid("a"),
                target: eid("b"),
                kind: "x".to_string(),
            },
        ];
        for r in &two {
            assert_eq!(r.endpoints().len(), 2, "{}", r.kerml_name());
        }
        let conn = Relation::Connection {
            ends: vec![eid("a"), eid("b"), eid("c"), eid("d")],
        };
        assert_eq!(conn.endpoints().len(), 4);
        assert_eq!(
            conn.endpoints(),
            vec![&eid("a"), &eid("b"), &eid("c"), &eid("d")]
        );
    }

    #[test]
    fn relation_kerml_name_matches_variant() {
        assert_eq!(
            Relation::Succession {
                source: eid("a"),
                target: eid("b")
            }
            .kerml_name(),
            "Succession"
        );
        assert_eq!(
            Relation::Domain {
                source: eid("a"),
                target: eid("b"),
                kind: "seq".to_string()
            }
            .kerml_name(),
            "Domain"
        );
    }
}
