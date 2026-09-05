//! The canonical UFO-typed semantic-graph edge layer.
//!
//! This module sits **below** [`crate::sysml_model`]'s SysML-v2 viewpoint
//! layer (`ElementKind` / `Relation`) and **above** [`crate::iso_ir`]'s
//! free-form `Node` / `Edge` transport floor. It is the *ontological*
//! middle: a fixed, closed relation vocabulary ([`UfoRelation`]) plus a
//! typed edge ([`OntologicalEdge`]) whose endpoints are checked against the
//! four UFO categories from [`crate::stereotype::UfoCategory`].
//!
//! # Where it fits in the consultant pipeline
//!
//! ```text
//! source ──▶ UFO semantic graph ──▶ pattern recognizers ──▶ SysML viewpoints ──▶ renderer adapters
//!  (iso_ir)      (ontology)            (downstream)           (sysml_model)          (downstream)
//! ```
//!
//! Raw sources (Rust ASTs, Kubernetes manifests, OpenAPI specs, KerML/SysML
//! files) are lifted into [`iso_ir`](crate::iso_ir) as untyped connectivity,
//! then normalized here into [`OntologicalEdge`]s carrying a canonical
//! [`UfoRelation`]. Pattern recognizers consume that typed graph; renderers
//! consume the SysML viewpoints derived from it. This crate owns only the
//! vocabulary — it is deliberately **domain-neutral**. The Kubernetes verb
//! classification table itself (which raw `kind`/verb maps to which
//! [`UfoRelation`]) lives downstream in `kr0ki` docs, not here.
//!
//! # Pure data
//!
//! Plain enums and one struct. No traits, no sealed hierarchies. Adding a
//! relation is a data change to [`UfoRelation`] (and its
//! [`UfoRelation::ALL`]); it is `#[non_exhaustive]` because the vocabulary
//! can grow, and consumers must not assume today's list is final.
//!
//! # Explicit non-conflations
//!
//! The vocabulary keeps apart distinctions that overloaded domain verbs
//! routinely blur:
//!
//! - [`UfoRelation::Controls`] is **not** [`UfoRelation::HasPart`] — a
//!   controller managing an object does not thereby *contain* it as a part.
//! - [`UfoRelation::HostedBy`] is **not** [`UfoRelation::MemberOf`] —
//!   placement on a node/cluster is not aggregation into a group.
//! - [`UfoRelation::Requires`] is **not** necessarily temporal invocation —
//!   it is a standing dependency on a capability, not a call that happened.
//! - [`UfoRelation::RoutesTo`] describes a *configured structural topology*
//!   (a proxy/route table); [`UfoRelation::FlowsTo`] describes an
//!   *occurrence* (a call/message that actually flowed). One can exist
//!   without the other.
//!
//! # Reference
//!
//! - `PromptExecution/kr0ki` — `docs/DESIGN-NOTE-typed-model-layer.md`
//!   §2.3 (provenance / [`SourceAnchor`]), §2.4 (identity — opaque, never
//!   numeric).
//! - Guizzardi, G. (2005). _Ontological Foundations for Structural
//!   Conceptual Models_ — the UFO categories the edge endpoints are typed
//!   against.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::stereotype::UfoCategory;
use crate::sysml_model::ElementId;

/// The canonical relation vocabulary of the UFO semantic graph.
///
/// A **closed** (`#[non_exhaustive]`) set of 25 relations that normalizes
/// the overloaded verbs of real domains — Kubernetes especially, where
/// "owns", "manages", "controls", "runs", "selects" and "routes" are used
/// interchangeably for ontologically distinct relationships. Each variant's
/// doc comment lists the common synonyms it absorbs and its ontological
/// reading (the UFO categories it relates).
///
/// Two parsing entry points, deliberately separate:
///
/// - [`FromStr`](std::str::FromStr) is **strict** — it accepts only the
///   canonical snake_case name ([`UfoRelation::canonical_name`]), modulo
///   case and surrounding whitespace.
/// - [`UfoRelation::from_synonym`] is **lenient** — it maps the documented
///   synonyms (and the canonical name), case- and separator-insensitive.
///   This is what pattern recognizers use to normalize raw k8s / OpenAPI
///   verbs.
///
/// See the module docs for the explicit non-conflations this vocabulary
/// preserves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UfoRelation {
    /// **Synonyms:** is-a, extends, subtype.
    ///
    /// **Reading:** universal → universal.
    Specializes,
    /// **Synonyms:** instance-of, object-of-kind.
    ///
    /// **Reading:** endurant → type.
    Instantiates,
    /// **Synonyms:** contains, comprises, composed-of.
    ///
    /// **Reading:** strong endurant composition.
    HasPart,
    /// **Synonyms:** belongs-to, grouped-in.
    ///
    /// **Reading:** endurant aggregation.
    MemberOf,
    /// **Synonyms:** in-namespace, namespace-of.
    ///
    /// **Reading:** object → namespace.
    ScopedBy,
    /// **Synonyms:** runs-on, placed-on, deployed-on.
    ///
    /// **Reading:** workload → node/cluster.
    HostedBy,
    /// **Synonyms:** owns, manages, reconciles.
    ///
    /// **Reading:** controller → managed object.
    Controls,
    /// **Synonyms:** watches, monitors, lists/watches.
    ///
    /// **Reading:** agent/controller → object.
    Observes,
    /// **Synonyms:** matches, targets.
    ///
    /// **Reading:** selector → candidate object.
    Selects,
    /// **Synonyms:** discovers, finds-endpoints.
    ///
    /// **Reading:** service/reference → endpoint.
    ResolvesTo,
    /// **Synonyms:** attaches, mounts, associates.
    ///
    /// **Reading:** relator mediating two endurants.
    Binds,
    /// **Synonyms:** offers, implements, exposes.
    ///
    /// **Reading:** provider → capability/interface.
    Provides,
    /// **Synonyms:** consumes, depends-on, needs.
    ///
    /// **Reading:** consumer → capability/interface.
    Requires,
    /// **Synonyms:** proxies-to, forwards-to.
    ///
    /// **Reading:** configured structural routing topology.
    RoutesTo,
    /// **Synonyms:** performs, executes, handles.
    ///
    /// **Reading:** endurant → perdurant.
    ParticipatesIn,
    /// **Synonyms:** triggers, starts, submits.
    ///
    /// **Reading:** endurant/event → perdurant.
    Initiates,
    /// **Synonyms:** before, followed-by.
    ///
    /// **Reading:** perdurant → perdurant.
    Precedes,
    /// **Synonyms:** produces, results-in.
    ///
    /// **Reading:** perdurant → event/state.
    Causes,
    /// **Synonyms:** changes-to, enters-state.
    ///
    /// **Reading:** event connecting two states.
    Transitions,
    /// **Synonyms:** calls, sends, publishes.
    ///
    /// **Reading:** occurrence / data flow.
    FlowsTo,
    /// **Synonyms:** meets, complies-with.
    ///
    /// **Reading:** element → requirement.
    Satisfies,
    /// **Synonyms:** tests, proves, demonstrates.
    ///
    /// **Reading:** evidence/activity → requirement.
    Verifies,
    /// **Synonyms:** constrained-by, policy-applies.
    ///
    /// **Reading:** element/process → policy.
    GovernedBy,
    /// **Synonyms:** approved-by, permitted-by.
    ///
    /// **Reading:** action → authority decision.
    AuthorizedBy,
    /// **Synonyms:** derived-from, corresponds-to.
    ///
    /// **Reading:** non-causal traceability.
    TracesTo,
}

impl UfoRelation {
    /// Every variant, in declaration order.
    ///
    /// The `tests` module matches exhaustively over this slice, so a new
    /// variant fails to compile until it is also appended here.
    pub const ALL: &'static [UfoRelation] = &[
        UfoRelation::Specializes,
        UfoRelation::Instantiates,
        UfoRelation::HasPart,
        UfoRelation::MemberOf,
        UfoRelation::ScopedBy,
        UfoRelation::HostedBy,
        UfoRelation::Controls,
        UfoRelation::Observes,
        UfoRelation::Selects,
        UfoRelation::ResolvesTo,
        UfoRelation::Binds,
        UfoRelation::Provides,
        UfoRelation::Requires,
        UfoRelation::RoutesTo,
        UfoRelation::ParticipatesIn,
        UfoRelation::Initiates,
        UfoRelation::Precedes,
        UfoRelation::Causes,
        UfoRelation::Transitions,
        UfoRelation::FlowsTo,
        UfoRelation::Satisfies,
        UfoRelation::Verifies,
        UfoRelation::GovernedBy,
        UfoRelation::AuthorizedBy,
        UfoRelation::TracesTo,
    ];

    /// The canonical snake_case name, identical to the serde wire form
    /// (e.g. [`UfoRelation::HasPart`] → `"has_part"`).
    pub const fn canonical_name(self) -> &'static str {
        match self {
            UfoRelation::Specializes => "specializes",
            UfoRelation::Instantiates => "instantiates",
            UfoRelation::HasPart => "has_part",
            UfoRelation::MemberOf => "member_of",
            UfoRelation::ScopedBy => "scoped_by",
            UfoRelation::HostedBy => "hosted_by",
            UfoRelation::Controls => "controls",
            UfoRelation::Observes => "observes",
            UfoRelation::Selects => "selects",
            UfoRelation::ResolvesTo => "resolves_to",
            UfoRelation::Binds => "binds",
            UfoRelation::Provides => "provides",
            UfoRelation::Requires => "requires",
            UfoRelation::RoutesTo => "routes_to",
            UfoRelation::ParticipatesIn => "participates_in",
            UfoRelation::Initiates => "initiates",
            UfoRelation::Precedes => "precedes",
            UfoRelation::Causes => "causes",
            UfoRelation::Transitions => "transitions",
            UfoRelation::FlowsTo => "flows_to",
            UfoRelation::Satisfies => "satisfies",
            UfoRelation::Verifies => "verifies",
            UfoRelation::GovernedBy => "governed_by",
            UfoRelation::AuthorizedBy => "authorized_by",
            UfoRelation::TracesTo => "traces_to",
        }
    }

    /// The expected `(source, target)` UFO categories for this relation.
    ///
    /// "Best ontological fit", not a hard schema — [`UfoRelation::permits`]
    /// is an *advisory* check callers may special-case. Judgement calls:
    ///
    /// - **Universals and types** ([`Specializes`](UfoRelation::Specializes)
    ///   source/target, [`Instantiates`](UfoRelation::Instantiates) target)
    ///   are modeled as [`UfoCategory::Abstract`] — UFO universals are not
    ///   individuals, and `UfoCategory` has no "universal" member.
    /// - **Requirements and policies**
    ///   ([`Satisfies`](UfoRelation::Satisfies),
    ///   [`Verifies`](UfoRelation::Verifies),
    ///   [`GovernedBy`](UfoRelation::GovernedBy),
    ///   [`AuthorizedBy`](UfoRelation::AuthorizedBy) targets) are
    ///   propositional — modeled as [`UfoCategory::Abstract`].
    /// - **[`Binds`](UfoRelation::Binds)** relates two endurants; the UFO
    ///   Relator (a Moment) it reifies is the *edge itself*, carried in
    ///   provenance, not an endpoint — hence `(Endurant, Endurant)`.
    /// - **[`Transitions`](UfoRelation::Transitions)** →
    ///   `(Perdurant, Perdurant)`: its source is really an Event and its
    ///   target really a *pair* of States, all of which are Perdurant
    ///   subtypes in this crate's [`UfoCategory`]. The pair-of-states
    ///   structure is not expressible in a single category signature.
    /// - **[`Initiates`](UfoRelation::Initiates)** source may also be an
    ///   Event (a Perdurant); `Endurant` is the common case.
    /// - **[`Causes`](UfoRelation::Causes)** target ("event/state") and
    ///   **[`Verifies`](UfoRelation::Verifies)** source ("evidence/activity")
    ///   likewise pick the dominant reading; the other is a subtype-compatible
    ///   alternative.
    /// - **[`TracesTo`](UfoRelation::TracesTo)** → `(Abstract, Abstract)`:
    ///   it is a meta-level correspondence between model artifacts, not a
    ///   relation between domain individuals, so `permits` rejects concrete
    ///   endurants by design.
    pub const fn category_signature(self) -> (UfoCategory, UfoCategory) {
        use UfoCategory::{Abstract, Endurant, Perdurant};
        match self {
            UfoRelation::Specializes => (Abstract, Abstract),
            UfoRelation::Instantiates => (Endurant, Abstract),
            UfoRelation::HasPart => (Endurant, Endurant),
            UfoRelation::MemberOf => (Endurant, Endurant),
            UfoRelation::ScopedBy => (Endurant, Endurant),
            UfoRelation::HostedBy => (Endurant, Endurant),
            UfoRelation::Controls => (Endurant, Endurant),
            UfoRelation::Observes => (Endurant, Endurant),
            UfoRelation::Selects => (Endurant, Endurant),
            UfoRelation::ResolvesTo => (Endurant, Endurant),
            UfoRelation::Binds => (Endurant, Endurant),
            UfoRelation::Provides => (Endurant, Endurant),
            UfoRelation::Requires => (Endurant, Endurant),
            UfoRelation::RoutesTo => (Endurant, Endurant),
            UfoRelation::ParticipatesIn => (Endurant, Perdurant),
            // Source may also be an Event (a Perdurant); Endurant is the common case.
            UfoRelation::Initiates => (Endurant, Perdurant),
            UfoRelation::Precedes => (Perdurant, Perdurant),
            // Target "event/state" — both are Perdurant subtypes here.
            UfoRelation::Causes => (Perdurant, Perdurant),
            // Source is really an Event; target is really a pair of States.
            UfoRelation::Transitions => (Perdurant, Perdurant),
            UfoRelation::FlowsTo => (Perdurant, Perdurant),
            UfoRelation::Satisfies => (Endurant, Abstract),
            // Source "evidence/activity" — activity (Perdurant) is the dominant reading.
            UfoRelation::Verifies => (Perdurant, Abstract),
            // Source "element/process" — element (Endurant) is the dominant reading.
            UfoRelation::GovernedBy => (Endurant, Abstract),
            UfoRelation::AuthorizedBy => (Perdurant, Abstract),
            UfoRelation::TracesTo => (Abstract, Abstract),
        }
    }

    /// Whether `(source, target)` exactly matches this relation's
    /// [`category_signature`](UfoRelation::category_signature).
    ///
    /// Exact match only — [`UfoCategory::Abstract`] on either side is **not**
    /// a wildcard. Callers that want tolerant checks (subtype compatibility,
    /// Event-for-Endurant on [`Initiates`](UfoRelation::Initiates), …)
    /// special-case on top of this.
    pub fn permits(self, source: UfoCategory, target: UfoCategory) -> bool {
        self.category_signature() == (source, target)
    }

    /// Whether this relation describes *configured static structure* of
    /// endurants — mereology, containment, placement, selection, binding,
    /// or routing topology.
    ///
    /// Mutually exclusive with [`is_occurrence`](UfoRelation::is_occurrence).
    /// Taxonomic ([`Specializes`](UfoRelation::Specializes),
    /// [`Instantiates`](UfoRelation::Instantiates)), capability/dependency
    /// ([`Provides`](UfoRelation::Provides),
    /// [`Requires`](UfoRelation::Requires)), management
    /// ([`Controls`](UfoRelation::Controls),
    /// [`Observes`](UfoRelation::Observes)), requirement, governance and
    /// traceability relations are **neither** — both predicates return
    /// `false` for them.
    pub const fn is_structural(self) -> bool {
        matches!(
            self,
            UfoRelation::HasPart
                | UfoRelation::MemberOf
                | UfoRelation::ScopedBy
                | UfoRelation::HostedBy
                | UfoRelation::Selects
                | UfoRelation::ResolvesTo
                | UfoRelation::Binds
                | UfoRelation::RoutesTo
        )
    }

    /// Whether this relation describes something *happening in time* — an
    /// endurant taking part in a perdurant, temporal/causal ordering, a
    /// state transition, or a data/control flow occurrence.
    ///
    /// Mutually exclusive with [`is_structural`](UfoRelation::is_structural);
    /// see it for the relations that are neither.
    pub const fn is_occurrence(self) -> bool {
        matches!(
            self,
            UfoRelation::ParticipatesIn
                | UfoRelation::Initiates
                | UfoRelation::Precedes
                | UfoRelation::Causes
                | UfoRelation::Transitions
                | UfoRelation::FlowsTo
        )
    }

    /// Lenient reverse lookup: map a raw domain verb — a documented synonym
    /// or the canonical name — to a variant.
    ///
    /// Case-insensitive and separator-insensitive (hyphen, underscore,
    /// space and slash are ignored, so `"lists/watches"`, `"runs-on"` and
    /// `"RUNS ON"` all resolve). This is **separate** from the strict
    /// [`FromStr`](std::str::FromStr) impl; it is what pattern recognizers
    /// call to normalize Kubernetes / OpenAPI vocabulary. Unknown input
    /// returns `None`.
    pub fn from_synonym(s: &str) -> Option<UfoRelation> {
        let n = normalize_loose(s);
        Some(match n.as_str() {
            "isa" | "extends" | "subtype" | "specializes" => UfoRelation::Specializes,
            "instanceof" | "objectofkind" | "instantiates" => UfoRelation::Instantiates,
            "contains" | "comprises" | "composedof" | "haspart" => UfoRelation::HasPart,
            "belongsto" | "groupedin" | "memberof" => UfoRelation::MemberOf,
            "innamespace" | "namespaceof" | "scopedby" => UfoRelation::ScopedBy,
            "runson" | "placedon" | "deployedon" | "hostedby" => UfoRelation::HostedBy,
            "owns" | "manages" | "reconciles" | "controls" => UfoRelation::Controls,
            "watches" | "monitors" | "listswatches" | "observes" => UfoRelation::Observes,
            "matches" | "targets" | "selects" => UfoRelation::Selects,
            "discovers" | "findsendpoints" | "resolvesto" => UfoRelation::ResolvesTo,
            "attaches" | "mounts" | "associates" | "binds" => UfoRelation::Binds,
            "offers" | "implements" | "exposes" | "provides" => UfoRelation::Provides,
            "consumes" | "dependson" | "needs" | "requires" => UfoRelation::Requires,
            "proxiesto" | "forwardsto" | "routesto" => UfoRelation::RoutesTo,
            "performs" | "executes" | "handles" | "participatesin" => UfoRelation::ParticipatesIn,
            "triggers" | "starts" | "submits" | "initiates" => UfoRelation::Initiates,
            "before" | "followedby" | "precedes" => UfoRelation::Precedes,
            "produces" | "resultsin" | "causes" => UfoRelation::Causes,
            "changesto" | "entersstate" | "transitions" => UfoRelation::Transitions,
            "calls" | "sends" | "publishes" | "flowsto" => UfoRelation::FlowsTo,
            "meets" | "complieswith" | "satisfies" => UfoRelation::Satisfies,
            "tests" | "proves" | "demonstrates" | "verifies" => UfoRelation::Verifies,
            "constrainedby" | "policyapplies" | "governedby" => UfoRelation::GovernedBy,
            "approvedby" | "permittedby" | "authorizedby" => UfoRelation::AuthorizedBy,
            "derivedfrom" | "correspondsto" | "tracesto" => UfoRelation::TracesTo,
            _ => return None,
        })
    }
}

/// Lowercase and drop every non-alphanumeric character (hyphen, underscore,
/// space, slash, `.`), so loosely-written verbs collapse to one key.
fn normalize_loose(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Error returned when a string does not name a [`UfoRelation`] in its
/// strict canonical form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "unknown UFO relation {input:?}; expected a canonical snake_case relation name \
     (e.g. \"has_part\", \"hosted_by\"); use UfoRelation::from_synonym for raw domain verbs"
)]
pub struct ParseUfoRelationError {
    /// The input string that failed to parse.
    pub input: String,
}

impl std::str::FromStr for UfoRelation {
    type Err = ParseUfoRelationError;

    /// Strict: the canonical snake_case name only
    /// ([`UfoRelation::canonical_name`]), case- and whitespace-insensitive.
    /// For synonym / domain-verb resolution use
    /// [`UfoRelation::from_synonym`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let norm: String = s
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect();
        Self::ALL
            .iter()
            .copied()
            .find(|r| norm == r.canonical_name())
            .ok_or(ParseUfoRelationError {
                input: s.to_string(),
            })
    }
}

/// A typed edge of the canonical UFO semantic graph.
///
/// Distinct from [`crate::iso_ir::Edge`], the free-form transport edge: this
/// one carries a canonical [`UfoRelation`], optional modeled occurrence
/// time, and structured provenance. Endpoints reuse
/// [`ElementId`](crate::sysml_model::ElementId) — the same opaque, never
/// numeric id newtype the SysML-v2 layer uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OntologicalEdge {
    /// Opaque edge identifier. A human-meaningful string, a content hash, or
    /// a derived key — **never** a numeric id (kr0ki
    /// `DESIGN-NOTE-typed-model-layer.md` §2.4).
    pub id: String,
    /// The source element.
    pub source: ElementId,
    /// The target element.
    pub target: ElementId,
    /// The canonical relation this edge asserts.
    pub relation: UfoRelation,
    /// Modeled occurrence time for perdurant / occurrence edges — `Some` for
    /// edges that ran at a modeled time (a rollout, a reconciliation),
    /// `None` for purely structural edges.
    pub occurrence: Option<TemporalExtent>,
    /// Zero or more attestations of where this edge came from. May be
    /// multi-sourced (the same edge recovered from a Rust AST *and* a
    /// Kubernetes manifest).
    pub provenance: Vec<SourceAnchor>,
}

impl OntologicalEdge {
    /// Construct an edge with no occurrence time and no provenance.
    pub fn new(
        id: impl Into<String>,
        source: ElementId,
        target: ElementId,
        relation: UfoRelation,
    ) -> Self {
        Self {
            id: id.into(),
            source,
            target,
            relation,
            occurrence: None,
            provenance: Vec::new(),
        }
    }

    /// The `(source, target)` endpoints, in order.
    pub fn endpoints(&self) -> (&ElementId, &ElementId) {
        (&self.source, &self.target)
    }

    /// Whether this edge carries at least one [`SourceAnchor`].
    pub fn is_attested(&self) -> bool {
        !self.provenance.is_empty()
    }
}

/// Modeled occurrence time — *when a modeled thing happened* (a rollout ran,
/// a reconciliation loop executed), expressed on whatever timeline the
/// caller chose.
///
/// Bounds are plain `String` on purpose: the caller picks ISO-8601, a
/// logical tick, a commit ref, a build number — anything monotonic and
/// meaningful in its model. Keeping it a string keeps model output
/// deterministic and free of wall-clock coupling.
///
/// This is **not** provenance (see [`SourceAnchor`]) and **not** a timestamp
/// this crate generates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TemporalExtent {
    /// A single modeled point in time.
    Instant(String),
    /// A bounded span; either end may be open.
    Interval {
        /// Start bound, if known.
        start: Option<String>,
        /// End bound, if known.
        end: Option<String>,
    },
    /// An open-ended span still in progress.
    Ongoing {
        /// Start bound, if known.
        since: Option<String>,
    },
}

/// A concrete, deterministic attestation of where an [`OntologicalEdge`]
/// came from — the realization of kr0ki `DESIGN-NOTE-typed-model-layer.md`
/// §2.3 provenance.
///
/// Every variant is deterministic and reproducible from the source
/// material: **no** uuids, **no** wall-clock timestamps. Two runs over the
/// same inputs produce byte-identical anchors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SourceAnchor {
    /// A span in a Rust source file.
    RustSpan {
        /// File path, as written in the source tree.
        file: String,
        /// 1-based line number.
        line: u32,
        /// 1-based column, if known.
        col: Option<u32>,
    },
    /// A fully-qualified Rust symbol path (`crate::module::Item`).
    SymbolPath(String),
    /// A KerML / SysML-v2 qualified name (`Pkg::Sub::Item`).
    KermlQualifiedName(String),
    /// A location in a `.sysml` / `.kerml` text file.
    SysmlFile {
        /// File path.
        path: String,
        /// 1-based line number, if known.
        line: Option<u32>,
    },
    /// A Kubernetes API object.
    K8sObject {
        /// `apiVersion`, e.g. `"apps/v1"`.
        api_version: String,
        /// `kind`, e.g. `"Deployment"`.
        kind: String,
        /// Namespace, for namespaced objects.
        namespace: Option<String>,
        /// Object name.
        name: String,
        /// `metadata.uid`, if captured. Deterministic per-object, assigned
        /// by the API server — not generated here.
        uid: Option<String>,
    },
    /// A version-control coordinate.
    Vcs {
        /// Repository URL or identifier, if known.
        repo: Option<String>,
        /// Commit hash — the deterministic anchor.
        commit: String,
        /// Path within the repo, if the anchor is file-scoped.
        path: Option<String>,
    },
    /// Any other provenance, as an opaque caller-defined string.
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn eid(s: &str) -> ElementId {
        ElementId::from(s)
    }

    #[test]
    fn ufo_relation_all_is_complete_and_ordered() {
        // Exhaustive match: a new variant fails to compile until it is also
        // appended to ALL.
        for &r in UfoRelation::ALL {
            match r {
                UfoRelation::Specializes
                | UfoRelation::Instantiates
                | UfoRelation::HasPart
                | UfoRelation::MemberOf
                | UfoRelation::ScopedBy
                | UfoRelation::HostedBy
                | UfoRelation::Controls
                | UfoRelation::Observes
                | UfoRelation::Selects
                | UfoRelation::ResolvesTo
                | UfoRelation::Binds
                | UfoRelation::Provides
                | UfoRelation::Requires
                | UfoRelation::RoutesTo
                | UfoRelation::ParticipatesIn
                | UfoRelation::Initiates
                | UfoRelation::Precedes
                | UfoRelation::Causes
                | UfoRelation::Transitions
                | UfoRelation::FlowsTo
                | UfoRelation::Satisfies
                | UfoRelation::Verifies
                | UfoRelation::GovernedBy
                | UfoRelation::AuthorizedBy
                | UfoRelation::TracesTo => {}
            }
        }
        assert_eq!(UfoRelation::ALL.len(), 25);
    }

    #[test]
    fn ufo_relation_serde_round_trips_and_wire_name_is_canonical() {
        for &r in UfoRelation::ALL {
            let json = serde_json::to_string(&r).unwrap();
            let back: UfoRelation = serde_json::from_str(&json).unwrap();
            assert_eq!(r, back, "round-trip failed for {r:?} via {json}");
            assert_eq!(
                json,
                format!("{:?}", r.canonical_name()),
                "serde wire name != canonical_name for {r:?}"
            );
        }
    }

    #[test]
    fn ufo_relation_canonical_name_is_inverse_of_from_str() {
        for &r in UfoRelation::ALL {
            assert_eq!(UfoRelation::from_str(r.canonical_name()), Ok(r));
        }
    }

    #[test]
    fn ufo_relation_from_str_is_strict_but_case_and_whitespace_insensitive() {
        assert_eq!(
            UfoRelation::from_str("  HAS_PART "),
            Ok(UfoRelation::HasPart)
        );
        assert_eq!(
            UfoRelation::from_str("resolves_to"),
            Ok(UfoRelation::ResolvesTo)
        );
        // Strict: a synonym is NOT accepted here.
        assert_eq!(
            UfoRelation::from_str("runs-on").unwrap_err().input,
            "runs-on"
        );
        // Strict: the space-separated form is not the canonical form.
        assert!(UfoRelation::from_str("has part").is_err());
    }

    #[test]
    fn ufo_relation_from_synonym_normalizes_domain_verbs() {
        assert_eq!(
            UfoRelation::from_synonym("is-a"),
            Some(UfoRelation::Specializes)
        );
        assert_eq!(
            UfoRelation::from_synonym("runs-on"),
            Some(UfoRelation::HostedBy)
        );
        assert_eq!(
            UfoRelation::from_synonym("lists/watches"),
            Some(UfoRelation::Observes)
        );
        assert_eq!(
            UfoRelation::from_synonym("depends-on"),
            Some(UfoRelation::Requires)
        );
        assert_eq!(
            UfoRelation::from_synonym("forwards-to"),
            Some(UfoRelation::RoutesTo)
        );
        // Canonical names are also accepted.
        assert_eq!(
            UfoRelation::from_synonym("has_part"),
            Some(UfoRelation::HasPart)
        );
        assert_eq!(
            UfoRelation::from_synonym("HAS PART"),
            Some(UfoRelation::HasPart)
        );
        // Every canonical name round-trips through from_synonym.
        for &r in UfoRelation::ALL {
            assert_eq!(UfoRelation::from_synonym(r.canonical_name()), Some(r));
        }
        // Unknown verbs are rejected.
        assert_eq!(UfoRelation::from_synonym("frobnicates"), None);
        assert_eq!(UfoRelation::from_synonym(""), None);
    }

    #[test]
    fn ufo_relation_non_conflations_are_preserved() {
        // Controls is not HasPart: distinct variants, and their structural
        // classification differs (HasPart is mereology; Controls is
        // management, neither structure nor occurrence).
        assert_ne!(UfoRelation::Controls, UfoRelation::HasPart);
        assert!(UfoRelation::HasPart.is_structural());
        assert!(!UfoRelation::Controls.is_structural());
        assert_ne!(
            UfoRelation::Controls.is_structural(),
            UfoRelation::HasPart.is_structural()
        );

        // HostedBy is not MemberOf.
        assert_ne!(UfoRelation::HostedBy, UfoRelation::MemberOf);

        // RoutesTo is configured topology (structural); FlowsTo is an
        // occurrence. Neither is the other.
        assert!(UfoRelation::RoutesTo.is_structural() && !UfoRelation::RoutesTo.is_occurrence());
        assert!(UfoRelation::FlowsTo.is_occurrence() && !UfoRelation::FlowsTo.is_structural());
        assert_ne!(UfoRelation::RoutesTo, UfoRelation::FlowsTo);
    }

    #[test]
    fn ufo_relation_structural_and_occurrence_are_mutually_exclusive() {
        for &r in UfoRelation::ALL {
            assert!(
                !(r.is_structural() && r.is_occurrence()),
                "{r:?} is both structural and occurrence"
            );
        }
        // Taxonomic / requirement / governance / traceability relations are
        // neither.
        for r in [
            UfoRelation::Specializes,
            UfoRelation::Instantiates,
            UfoRelation::Satisfies,
            UfoRelation::Verifies,
            UfoRelation::GovernedBy,
            UfoRelation::AuthorizedBy,
            UfoRelation::TracesTo,
        ] {
            assert!(!r.is_structural() && !r.is_occurrence(), "{r:?}");
        }
    }

    #[test]
    fn ufo_relation_permits_checks_category_signature_exactly() {
        use UfoCategory::{Endurant, Perdurant};

        assert!(UfoRelation::HasPart.permits(Endurant, Endurant));
        assert!(!UfoRelation::HasPart.permits(Endurant, Perdurant));
        assert!(UfoRelation::ParticipatesIn.permits(Endurant, Perdurant));
        assert!(UfoRelation::Precedes.permits(Perdurant, Perdurant));

        // Signature accessor agrees with permits.
        for &r in UfoRelation::ALL {
            let (s, t) = r.category_signature();
            assert!(r.permits(s, t), "{r:?} does not permit its own signature");
        }
    }

    #[test]
    fn ufo_relation_transitions_signature_is_perdurant_pair() {
        assert_eq!(
            UfoRelation::Transitions.category_signature(),
            (UfoCategory::Perdurant, UfoCategory::Perdurant)
        );
    }

    #[test]
    fn ontological_edge_round_trips_with_occurrence_and_multi_provenance() {
        let edge = OntologicalEdge {
            id: "edge:deploy-nginx--hosted_by--node-3".to_string(),
            source: eid("apps/v1/Deployment/default/nginx"),
            target: eid("core/v1/Node/node-3"),
            relation: UfoRelation::HostedBy,
            occurrence: Some(TemporalExtent::Interval {
                start: Some("2026-09-05T00:00:00Z".to_string()),
                end: None,
            }),
            provenance: vec![
                SourceAnchor::K8sObject {
                    api_version: "apps/v1".to_string(),
                    kind: "Deployment".to_string(),
                    namespace: Some("default".to_string()),
                    name: "nginx".to_string(),
                    uid: Some("abc-123".to_string()),
                },
                SourceAnchor::RustSpan {
                    file: "src/recognizers/k8s.rs".to_string(),
                    line: 42,
                    col: Some(9),
                },
            ],
        };

        assert!(edge.is_attested());
        assert_eq!(edge.endpoints(), (&edge.source, &edge.target));

        let json = serde_json::to_string(&edge).unwrap();
        let back: OntologicalEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(edge, back, "round-trip failed via {json}");
    }

    #[test]
    fn ontological_edge_new_is_unattested_and_structural() {
        let edge = OntologicalEdge::new("e1", eid("A"), eid("B"), UfoRelation::HasPart);
        assert!(!edge.is_attested());
        assert!(edge.occurrence.is_none());
        assert_eq!(edge.endpoints(), (&eid("A"), &eid("B")));
    }

    #[test]
    fn temporal_extent_round_trips_for_every_variant() {
        let samples = [
            TemporalExtent::Instant("t0".to_string()),
            TemporalExtent::Interval {
                start: Some("t0".to_string()),
                end: Some("t1".to_string()),
            },
            TemporalExtent::Interval {
                start: None,
                end: None,
            },
            TemporalExtent::Ongoing {
                since: Some("t0".to_string()),
            },
            TemporalExtent::Ongoing { since: None },
        ];
        for te in &samples {
            let json = serde_json::to_string(te).unwrap();
            let back: TemporalExtent = serde_json::from_str(&json).unwrap();
            assert_eq!(*te, back, "round-trip failed via {json}");
        }
    }

    #[test]
    fn source_anchor_round_trips_for_every_variant() {
        let samples = [
            SourceAnchor::RustSpan {
                file: "src/lib.rs".to_string(),
                line: 10,
                col: None,
            },
            SourceAnchor::SymbolPath("ufo_types::ontology::UfoRelation".to_string()),
            SourceAnchor::KermlQualifiedName("Pkg::Sub::Item".to_string()),
            SourceAnchor::SysmlFile {
                path: "model/system.sysml".to_string(),
                line: Some(3),
            },
            SourceAnchor::K8sObject {
                api_version: "v1".to_string(),
                kind: "Service".to_string(),
                namespace: None,
                name: "api".to_string(),
                uid: None,
            },
            SourceAnchor::Vcs {
                repo: Some("git@github.com:promptexecution/ufo-types.git".to_string()),
                commit: "deadbeef".to_string(),
                path: Some("src/ontology.rs".to_string()),
            },
            SourceAnchor::Other("hand-authored".to_string()),
        ];
        for sa in &samples {
            let json = serde_json::to_string(sa).unwrap();
            let back: SourceAnchor = serde_json::from_str(&json).unwrap();
            assert_eq!(*sa, back, "round-trip failed via {json}");
        }
    }
}
