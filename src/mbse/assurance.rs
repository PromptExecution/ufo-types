//! Versioned assurance thread — one typed profile over
//! [`RequirementGraph`](super::requirements::RequirementGraph):
//!
//! ```text
//! source obligation → requirement → system element → enforcing control
//!                                 ↘ verification case → evidence
//! ```
//!
//! Every box except evidence is a node of the existing requirement graph,
//! distinguished by a typed `node_kind` attribute, so ReqIF export, Flexo sync
//! and the induced views keep working unchanged. (In ReqIF terms the system
//! element, control and verification case are *external element references*:
//! SpecObjects that carry an id and a revision-qualified locator, not content.)
//! Evidence is a revision-bound [`EvidenceRecord`] kept outside the graph; the
//! graph only holds an [`EvidenceRef`](super::requirements::EvidenceRef) to it.
//!
//! # Two different claims
//!
//! - A **satisfaction** edge (`element —satisfies→ requirement`) is an
//!   *architectural assertion*. It is true or false about the design and says
//!   nothing about whether anyone ran anything.
//! - A **verification result** is *evidence*: pass/fail for one verification
//!   case at one `(model, implementation, configuration)` revision triple. It
//!   goes stale the moment any of the three changes.
//!
//! [`Assurance`] keeps them apart: a requirement can be *satisfied but
//! untested*, *verified*, *failing*, or *stale*, and never silently
//! "verified" because an edge exists. The pre-existing
//! `ViewKind::VerificationCoverage` counts any `verifies` edge to evidence as
//! covered; this module is the revision-aware replacement for assurance use.
//!
//! # Edge direction
//!
//! Read an edge as `source <kind> target`. The profile uses the existing
//! [`RequirementRelationKind`] vocabulary:
//!
//! | edge | meaning |
//! |---|---|
//! | `source_obligation —derives→ requirement` | the requirement is derived from the obligation |
//! | `system_element —satisfies→ requirement` | architectural satisfaction assertion |
//! | `control —implements→ requirement` | the control enforces the requirement |
//! | `control —allocated_to→ system_element` | where the control lives |
//! | `verification_case —verifies→ requirement` | the case is the requirement's acceptance test |
//!
//! Only **asserted** edges count; inferred and proposed edges are review-only.
//!
//! Deterministic checks (schema, identifiers, links, revisions) live here.
//! Whether a statement is *ambiguous* is a judgment for the authoring agent;
//! [`statement_issues`] only flags what a string scan can know.

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::requirements::{
    RelationAuthority, Requirement, RequirementGraph, RequirementRelationKind,
};

/// Attribute key carrying a node's [`NodeKind`].
pub const NODE_KIND_ATTR: &str = "node_kind";
/// The closed vocabulary of a profile attribute, if it has one. These are the attributes an
/// exchange format should carry as enumerations rather than free text.
pub fn attribute_vocabulary(key: &str) -> Option<Vec<&'static str>> {
    match key {
        NODE_KIND_ATTR => Some(NodeKind::ALL.iter().map(|k| k.as_str()).collect()),
        "source_kind" => Some(SourceKind::ALL.iter().map(|k| k.as_str()).collect()),
        "status" => Some(RequirementStatus::ALL.iter().map(|k| k.as_str()).collect()),
        _ => None,
    }
}

/// Attribute keys of the required requirement fields (besides `id` and the
/// statement, which are the node's `id` and `text`).
pub const REQUIRED_ATTRS: [&str; 7] = [
    "source",
    "source_kind",
    "owner",
    "rationale",
    "verification_id",
    "acceptance",
    "status",
];

/// What a graph node is, in the assurance profile.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    SourceObligation,
    Requirement,
    SystemElement,
    Control,
    VerificationCase,
}

impl NodeKind {
    /// Every kind, in thread order.
    pub const ALL: [NodeKind; 5] = [
        NodeKind::SourceObligation,
        NodeKind::Requirement,
        NodeKind::SystemElement,
        NodeKind::Control,
        NodeKind::VerificationCase,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            NodeKind::SourceObligation => "source_obligation",
            NodeKind::Requirement => "requirement",
            NodeKind::SystemElement => "system_element",
            NodeKind::Control => "control",
            NodeKind::VerificationCase => "verification_case",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

/// How binding the source of a requirement is. Kept distinguishable on purpose:
/// guidance can be tailored away, a binding obligation cannot.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// A law, contract term or standard clause the system must meet.
    BindingObligation,
    /// The organisation's own policy.
    OrganisationalPolicy,
    /// Advice that may be tailored.
    Guidance,
}

impl SourceKind {
    /// Every kind.
    pub const ALL: [SourceKind; 3] = [
        SourceKind::BindingObligation,
        SourceKind::OrganisationalPolicy,
        SourceKind::Guidance,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::BindingObligation => "binding_obligation",
            SourceKind::OrganisationalPolicy => "organisational_policy",
            SourceKind::Guidance => "guidance",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

/// Requirement lifecycle (the repository's valigate stages).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RequirementStatus {
    Draft,
    Review,
    Validated,
    Gated,
    Implemented,
}

impl RequirementStatus {
    /// Every status, in lifecycle order.
    pub const ALL: [RequirementStatus; 5] = [
        RequirementStatus::Draft,
        RequirementStatus::Review,
        RequirementStatus::Validated,
        RequirementStatus::Gated,
        RequirementStatus::Implemented,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            RequirementStatus::Draft => "draft",
            RequirementStatus::Review => "review",
            RequirementStatus::Validated => "validated",
            RequirementStatus::Gated => "gated",
            RequirementStatus::Implemented => "implemented",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

/// Outcome of running one verification case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VerificationResult {
    Pass,
    Fail,
    /// The case could not run to a verdict (harness error, timeout).
    Error,
}

// ── Profile requirement ──────────────────────────────────────────────────────

/// A deterministic, structured problem with a requirement's profile fields.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileIssue {
    #[error("requirement `{id}` is missing required attribute `{attr}`")]
    MissingAttribute { id: String, attr: &'static str },
    #[error("requirement `{id}`: `{attr}` has invalid value `{value}`")]
    InvalidValue {
        id: String,
        attr: &'static str,
        value: String,
    },
    #[error("requirement `{id}` has an empty statement")]
    EmptyStatement { id: String },
}

/// The nine required requirement fields, typed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProfileRequirement {
    pub id: String,
    pub statement: String,
    /// Where the obligation comes from (a URI or locator).
    pub source: String,
    pub source_kind: SourceKind,
    pub owner: String,
    /// Why — kept apart from the statement.
    pub rationale: String,
    /// The verification case that accepts this requirement.
    pub verification_id: String,
    /// The measurable acceptance criterion.
    pub acceptance: String,
    pub status: RequirementStatus,
}

impl ProfileRequirement {
    /// Read the profile fields off a graph node, reporting *every* problem.
    pub fn from_node(node: &Requirement) -> Result<Self, Vec<ProfileIssue>> {
        let mut issues = Vec::new();
        if node.text.trim().is_empty() {
            issues.push(ProfileIssue::EmptyStatement {
                id: node.id.clone(),
            });
        }
        let mut get = |attr: &'static str| -> String {
            match node.attributes.get(attr).map(|v| v.trim()) {
                Some(v) if !v.is_empty() => v.to_string(),
                _ => {
                    issues.push(ProfileIssue::MissingAttribute {
                        id: node.id.clone(),
                        attr,
                    });
                    String::new()
                }
            }
        };
        let source = get("source");
        let source_kind_raw = get("source_kind");
        let owner = get("owner");
        let rationale = get("rationale");
        let verification_id = get("verification_id");
        let acceptance = get("acceptance");
        let status_raw = get("status");

        let source_kind = SourceKind::parse(&source_kind_raw);
        if source_kind.is_none() && !source_kind_raw.is_empty() {
            issues.push(ProfileIssue::InvalidValue {
                id: node.id.clone(),
                attr: "source_kind",
                value: source_kind_raw,
            });
        }
        let status = RequirementStatus::parse(&status_raw);
        if status.is_none() && !status_raw.is_empty() {
            issues.push(ProfileIssue::InvalidValue {
                id: node.id.clone(),
                attr: "status",
                value: status_raw,
            });
        }
        match (source_kind, status, issues.is_empty()) {
            (Some(source_kind), Some(status), true) => Ok(ProfileRequirement {
                id: node.id.clone(),
                statement: node.text.clone(),
                source,
                source_kind,
                owner,
                rationale,
                verification_id,
                acceptance,
                status,
            }),
            _ => Err(issues),
        }
    }

    /// The attribute map to store on a graph node (inverse of [`Self::from_node`]).
    pub fn to_attributes(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                NODE_KIND_ATTR.to_string(),
                NodeKind::Requirement.as_str().to_string(),
            ),
            ("source".to_string(), self.source.clone()),
            (
                "source_kind".to_string(),
                self.source_kind.as_str().to_string(),
            ),
            ("owner".to_string(), self.owner.clone()),
            ("rationale".to_string(), self.rationale.clone()),
            ("verification_id".to_string(), self.verification_id.clone()),
            ("acceptance".to_string(), self.acceptance.clone()),
            ("status".to_string(), self.status.as_str().to_string()),
        ])
    }
}

/// A lint a string scan can establish about a requirement statement.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StatementIssue {
    #[error("no `shall`: a binding requirement states one")]
    MissingShall,
    #[error("{0} occurrences of `shall`: write one observable obligation per requirement")]
    MultipleShall(usize),
    #[error("`shall` has no responsible component before it")]
    NoResponsibleComponent,
    #[error("vague or open term `{0}`")]
    VagueTerm(String),
}

const VAGUE_TERMS: [&str; 9] = [
    "tbd",
    "tbc",
    "etc",
    "and/or",
    "as appropriate",
    "user-friendly",
    "adequate",
    "if possible",
    "where practical",
];

/// Deterministic statement lint (NASA: singular, clear, verifiable). It cannot
/// judge ambiguity or whether a behaviour is observable; that stays with the
/// authoring agent and the reviewer.
pub fn statement_issues(statement: &str) -> Vec<StatementIssue> {
    let lower = statement.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric() && c != '/' && c != '-')
        .filter(|w| !w.is_empty())
        .collect();
    let shall_positions: Vec<usize> = words
        .iter()
        .enumerate()
        .filter(|(_, w)| **w == "shall")
        .map(|(i, _)| i)
        .collect();
    let mut issues = Vec::new();
    match shall_positions.len() {
        0 => issues.push(StatementIssue::MissingShall),
        1 => {
            if shall_positions[0] == 0 {
                issues.push(StatementIssue::NoResponsibleComponent);
            }
        }
        n => issues.push(StatementIssue::MultipleShall(n)),
    }
    for term in VAGUE_TERMS {
        let hit = if term.contains(' ') || term.contains('/') {
            lower.contains(term)
        } else {
            words.contains(&term)
        };
        if hit {
            issues.push(StatementIssue::VagueTerm(term.to_string()));
        }
    }
    issues
}

// ── Evidence ─────────────────────────────────────────────────────────────────

/// The three revisions a result is only valid for.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct RevisionSet {
    /// Revision (commit id) of the model the case was checked against.
    pub model_revision: String,
    /// Revision of the implementation under test.
    pub implementation_revision: String,
    /// Digest of the configuration the case ran with (`sha256:…`).
    pub configuration_digest: String,
}

/// What is current *now*.
///
/// The model and implementation revisions are global. The configuration digest is **per
/// verification case**, because it digests what *that case* checks (its command, acceptance
/// and the requirements it verifies); one digest for every case would mark all but one case's
/// evidence stale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CurrentRevisions {
    pub model_revision: String,
    pub implementation_revision: String,
    /// `verification_id` → the configuration digest a result for that case must carry to be
    /// current. A case missing from the map cannot be confirmed current, so its results are
    /// stale.
    pub configuration_digests: BTreeMap<String, String>,
}

impl CurrentRevisions {
    pub fn new(
        model_revision: impl Into<String>,
        implementation_revision: impl Into<String>,
    ) -> Self {
        Self {
            model_revision: model_revision.into(),
            implementation_revision: implementation_revision.into(),
            configuration_digests: BTreeMap::new(),
        }
    }

    pub fn with_case(
        mut self,
        verification_id: impl Into<String>,
        digest: impl Into<String>,
    ) -> Self {
        self.configuration_digests
            .insert(verification_id.into(), digest.into());
        self
    }

    /// The same configuration digest for each listed case (when they share one, or in tests).
    pub fn for_cases(
        revisions: &RevisionSet,
        cases: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let mut out = Self::new(
            &revisions.model_revision,
            &revisions.implementation_revision,
        );
        for c in cases {
            out.configuration_digests
                .insert(c.into(), revisions.configuration_digest.clone());
        }
        out
    }
}

/// Which revisions no longer match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Drift {
    pub model: bool,
    pub implementation: bool,
    pub configuration: bool,
}

/// Whether a result still speaks for the current revisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Freshness {
    Fresh,
    Stale { drift: Drift },
}

/// Why an evidence record is malformed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EvidenceError {
    #[error("evidence field `{0}` is empty")]
    Empty(&'static str),
    #[error("evidence field `{field}` must be `sha256:` + 64 hex digits, got `{value}`")]
    BadDigest { field: &'static str, value: String },
}

/// Revision-bound verification evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceRecord {
    pub requirement_id: String,
    pub verification_id: String,
    pub model_revision: String,
    pub implementation_revision: String,
    pub configuration_digest: String,
    pub result: VerificationResult,
    pub artifact_uri: String,
    pub artifact_digest: String,
}

fn is_sha256(s: &str) -> bool {
    s.strip_prefix("sha256:")
        .is_some_and(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
}

impl EvidenceRecord {
    pub fn validate(&self) -> Result<(), EvidenceError> {
        for (name, v) in [
            ("requirement_id", &self.requirement_id),
            ("verification_id", &self.verification_id),
            ("model_revision", &self.model_revision),
            ("implementation_revision", &self.implementation_revision),
            ("artifact_uri", &self.artifact_uri),
        ] {
            if v.trim().is_empty() {
                return Err(EvidenceError::Empty(name));
            }
        }
        for (name, v) in [
            ("configuration_digest", &self.configuration_digest),
            ("artifact_digest", &self.artifact_digest),
        ] {
            if !is_sha256(v) {
                return Err(EvidenceError::BadDigest {
                    field: name,
                    value: v.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn revisions(&self) -> RevisionSet {
        RevisionSet {
            model_revision: self.model_revision.clone(),
            implementation_revision: self.implementation_revision.clone(),
            configuration_digest: self.configuration_digest.clone(),
        }
    }

    /// A deterministic identity: the same case run at the same revisions is the
    /// same evidence.
    pub fn key(&self) -> String {
        format!(
            "ev:{}:{}:{}:{}",
            self.verification_id,
            self.model_revision,
            self.implementation_revision,
            self.configuration_digest
        )
    }

    /// Compare against the current revisions. Fresh only if all three match; the configuration
    /// is compared with this record's *own case's* current digest, and a case with no current
    /// digest cannot be confirmed, so it counts as drifted.
    pub fn freshness(&self, current: &CurrentRevisions) -> Freshness {
        let drift = Drift {
            model: self.model_revision != current.model_revision,
            implementation: self.implementation_revision != current.implementation_revision,
            configuration: current.configuration_digests.get(&self.verification_id)
                != Some(&self.configuration_digest),
        };
        if drift.model || drift.implementation || drift.configuration {
            Freshness::Stale { drift }
        } else {
            Freshness::Fresh
        }
    }
}

// ── The thread ───────────────────────────────────────────────────────────────

/// The graph could not be read as an assurance thread.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ThreadError {
    #[error("node `{0}` has no `node_kind` attribute")]
    MissingNodeKind(String),
    #[error("node `{id}` has unknown node_kind `{value}`")]
    UnknownNodeKind { id: String, value: String },
    #[error(transparent)]
    Graph(#[from] super::requirements::RequirementError),
}

/// What is wrong, or not yet right, with one requirement's thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "gap", rename_all = "snake_case")]
pub enum GapKind {
    /// No source obligation derives this requirement.
    NoSource,
    /// No system element satisfies it: an implementation gap.
    Unsatisfied,
    /// Its `verification_id` names no verification case in the graph.
    UnknownVerificationCase { verification_id: String },
    /// No verification case verifies it by an asserted edge.
    NoVerificationCase,
    /// No control is recorded as enforcing it.
    NoEnforcingControl,
    /// An enforcing control exists in the model but records no `implementation`
    /// (code path, deployment manifest, …): an implementation gap.
    ControlNotImplemented { control_id: String },
    /// Satisfied by design, but no evidence exists.
    SatisfiedUntested,
    /// Fresh evidence says the case failed.
    Failing,
    /// Evidence exists only for other revisions.
    Stale,
    /// A satisfying element does not exist in the model revision.
    DanglingElement { element_id: String },
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Gap {
    pub requirement_id: String,
    #[serde(flatten)]
    pub kind: GapKind,
}

/// The assurance state of one requirement. Satisfaction and verification are
/// separate axes; this is their combination, never a shortcut for one by the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Assurance {
    /// Nothing asserts the design satisfies it.
    Unsatisfied,
    /// Satisfied (asserted), never tested.
    SatisfiedUntested,
    /// At least one fresh pass and no fresh failure.
    Verified,
    /// A fresh failure.
    Failing,
    /// Only stale evidence.
    Stale,
}

/// Evidence considered for one requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceAssessment {
    pub key: String,
    pub verification_id: String,
    pub result: VerificationResult,
    pub freshness: Freshness,
}

/// One requirement's place in the thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RequirementThread {
    pub requirement_id: String,
    pub sources: Vec<String>,
    pub satisfied_by: Vec<String>,
    pub enforced_by: Vec<String>,
    pub verification_cases: Vec<String>,
    pub evidence: Vec<EvidenceAssessment>,
    pub assurance: Assurance,
}

/// The analysed thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ThreadReport {
    pub requirements: Vec<RequirementThread>,
    pub gaps: Vec<Gap>,
    /// Evidence that matches no requirement/case in the graph.
    pub orphan_evidence: Vec<String>,
}

impl ThreadReport {
    pub fn requirement(&self, id: &str) -> Option<&RequirementThread> {
        self.requirements.iter().find(|r| r.requirement_id == id)
    }

    pub fn gaps_for<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'a GapKind> + 'a {
        self.gaps
            .iter()
            .filter(move |g| g.requirement_id == id)
            .map(|g| &g.kind)
    }
}

fn node_kind(node: &Requirement) -> Result<NodeKind, ThreadError> {
    let raw = node
        .attributes
        .get(NODE_KIND_ATTR)
        .ok_or_else(|| ThreadError::MissingNodeKind(node.id.clone()))?;
    NodeKind::parse(raw).ok_or_else(|| ThreadError::UnknownNodeKind {
        id: node.id.clone(),
        value: raw.clone(),
    })
}

/// Analyse a graph as an assurance thread against `current` revisions.
///
/// `known_elements`, when given, is the set of element ids that exist in the
/// model revision being checked (KR-A02); a `system_element` node whose
/// `qualified_name` (or id, absent that) is not in it is reported as dangling.
pub fn analyze(
    graph: &RequirementGraph,
    evidence: &[EvidenceRecord],
    current: &CurrentRevisions,
    known_elements: Option<&BTreeSet<String>>,
) -> Result<ThreadReport, ThreadError> {
    graph.validate()?;
    let mut kinds: BTreeMap<&str, NodeKind> = BTreeMap::new();
    for node in &graph.requirements {
        kinds.insert(node.id.as_str(), node_kind(node)?);
    }
    let kind_of = |id: &str| kinds.get(id).copied();
    let by_id: BTreeMap<&str, &Requirement> = graph
        .requirements
        .iter()
        .map(|n| (n.id.as_str(), n))
        .collect();

    // asserted edges only
    let edges: Vec<_> = graph
        .relations
        .iter()
        .filter(|e| matches!(e.authority, RelationAuthority::Asserted))
        .collect();

    let mut requirements = Vec::new();
    let mut gaps = Vec::new();

    for node in graph
        .requirements
        .iter()
        .filter(|n| kind_of(&n.id) == Some(NodeKind::Requirement))
    {
        let sources = peers(&edges, RequirementRelationKind::Derives, &node.id, |s| {
            kind_of(s) == Some(NodeKind::SourceObligation)
        });
        let satisfied_by = peers(&edges, RequirementRelationKind::Satisfies, &node.id, |s| {
            kind_of(s) == Some(NodeKind::SystemElement)
        });
        let enforced_by = peers(&edges, RequirementRelationKind::Implements, &node.id, |s| {
            kind_of(s) == Some(NodeKind::Control)
        });
        let verification_cases = peers(&edges, RequirementRelationKind::Verifies, &node.id, |s| {
            kind_of(s) == Some(NodeKind::VerificationCase)
        });

        let mut push = |kind: GapKind| {
            gaps.push(Gap {
                requirement_id: node.id.clone(),
                kind,
            });
        };

        if sources.is_empty() {
            push(GapKind::NoSource);
        }
        if satisfied_by.is_empty() {
            push(GapKind::Unsatisfied);
        }
        if let Some(declared) = node.attributes.get("verification_id") {
            if kind_of(declared) != Some(NodeKind::VerificationCase) {
                push(GapKind::UnknownVerificationCase {
                    verification_id: declared.clone(),
                });
            }
        }
        if verification_cases.is_empty() {
            push(GapKind::NoVerificationCase);
        }
        if enforced_by.is_empty() {
            push(GapKind::NoEnforcingControl);
        }
        for control in &enforced_by {
            let implemented = by_id
                .get(control.as_str())
                .and_then(|n| n.attributes.get("implementation"))
                .is_some_and(|v| !v.trim().is_empty());
            if !implemented {
                push(GapKind::ControlNotImplemented {
                    control_id: control.clone(),
                });
            }
        }
        if let Some(known) = known_elements {
            for element in &satisfied_by {
                let locator = by_id
                    .get(element.as_str())
                    .and_then(|n| n.attributes.get("qualified_name"))
                    .unwrap_or(element);
                if !known.contains(locator) {
                    push(GapKind::DanglingElement {
                        element_id: element.clone(),
                    });
                }
            }
        }

        let assessed: Vec<EvidenceAssessment> = evidence
            .iter()
            .filter(|e| {
                e.requirement_id == node.id && verification_cases.contains(&e.verification_id)
            })
            .map(|e| EvidenceAssessment {
                key: e.key(),
                verification_id: e.verification_id.clone(),
                result: e.result,
                freshness: e.freshness(current),
            })
            .collect();
        let fresh = |a: &&EvidenceAssessment| a.freshness == Freshness::Fresh;
        let assurance = if satisfied_by.is_empty() {
            Assurance::Unsatisfied
        } else if assessed.is_empty() {
            Assurance::SatisfiedUntested
        } else if assessed
            .iter()
            .filter(fresh)
            .any(|a| a.result != VerificationResult::Pass)
        {
            Assurance::Failing
        } else if assessed.iter().any(|a| fresh(&a)) {
            Assurance::Verified
        } else {
            Assurance::Stale
        };
        match assurance {
            Assurance::SatisfiedUntested => push(GapKind::SatisfiedUntested),
            Assurance::Failing => push(GapKind::Failing),
            Assurance::Stale => push(GapKind::Stale),
            Assurance::Unsatisfied | Assurance::Verified => {}
        }

        requirements.push(RequirementThread {
            requirement_id: node.id.clone(),
            sources,
            satisfied_by,
            enforced_by,
            verification_cases,
            evidence: assessed,
            assurance,
        });
    }

    let orphan_evidence = evidence
        .iter()
        .filter(|e| {
            !requirements.iter().any(|r| {
                r.requirement_id == e.requirement_id
                    && r.verification_cases.contains(&e.verification_id)
            })
        })
        .map(EvidenceRecord::key)
        .collect();

    Ok(ThreadReport {
        requirements,
        gaps,
        orphan_evidence,
    })
}

/// Ids at the *source* end of asserted `kind` edges that target `target`,
/// accepted by `keep`; sorted and de-duplicated.
fn peers(
    edges: &[&super::requirements::RequirementRelation],
    kind: RequirementRelationKind,
    target: &str,
    keep: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut out: Vec<String> = edges
        .iter()
        .filter(|e| e.kind == kind && e.target == target && keep(&e.source))
        .map(|e| e.source.clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::super::requirements::{BaselineIdentity, Provenance, RequirementRelation};
    use super::*;

    const D1: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000001";
    const D2: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000002";

    fn baseline() -> BaselineIdentity {
        BaselineIdentity {
            id: "BL".into(),
            revision: "r1".into(),
            import_artifact_sha256: None,
            exported_baseline_sha256: None,
        }
    }

    fn prov() -> Provenance {
        Provenance {
            source_uri: "test".into(),
            artifact_sha256: None,
            locator: None,
        }
    }

    fn node(id: &str, kind: NodeKind, text: &str, extra: &[(&str, &str)]) -> Requirement {
        let mut attributes =
            BTreeMap::from([(NODE_KIND_ATTR.to_string(), kind.as_str().to_string())]);
        for (k, v) in extra {
            attributes.insert((*k).to_string(), (*v).to_string());
        }
        Requirement {
            id: id.into(),
            title: id.into(),
            text: text.into(),
            baseline: baseline(),
            provenance: prov(),
            attributes,
            evidence: vec![],
        }
    }

    fn edge(
        id: &str,
        source: &str,
        target: &str,
        kind: RequirementRelationKind,
    ) -> RequirementRelation {
        RequirementRelation {
            id: id.into(),
            source: source.into(),
            target: target.into(),
            kind,
            authority: RelationAuthority::Asserted,
            provenance: prov(),
            promotion: None,
        }
    }

    /// `current` for the single case `C1`.
    fn current(r: &RevisionSet) -> CurrentRevisions {
        CurrentRevisions::for_cases(r, ["C1"])
    }

    fn revs(model: &str, implementation: &str) -> RevisionSet {
        RevisionSet {
            model_revision: model.into(),
            implementation_revision: implementation.into(),
            configuration_digest: D1.into(),
        }
    }

    fn record(
        req: &str,
        case: &str,
        at: &RevisionSet,
        result: VerificationResult,
    ) -> EvidenceRecord {
        EvidenceRecord {
            requirement_id: req.into(),
            verification_id: case.into(),
            model_revision: at.model_revision.clone(),
            implementation_revision: at.implementation_revision.clone(),
            configuration_digest: at.configuration_digest.clone(),
            result,
            artifact_uri: format!("file:///evidence/{req}.json"),
            artifact_digest: D2.into(),
        }
    }

    /// OBL —derives→ R1, R2;  E —satisfies→ R1 (not R2);  C1 —verifies→ R1.
    fn graph() -> RequirementGraph {
        use RequirementRelationKind::*;
        RequirementGraph {
            baseline: baseline(),
            requirements: vec![
                node("OBL", NodeKind::SourceObligation, "Binding clause 4.2", &[]),
                node(
                    "R1",
                    NodeKind::Requirement,
                    "The gateway shall deny a write.",
                    &[("verification_id", "C1")],
                ),
                node(
                    "R2",
                    NodeKind::Requirement,
                    "The store shall keep evidence.",
                    &[("verification_id", "C-missing")],
                ),
                node(
                    "E",
                    NodeKind::SystemElement,
                    "ToolGateway",
                    &[("qualified_name", "Kr0ki::ToolGateway")],
                ),
                node(
                    "CTL",
                    NodeKind::Control,
                    "deny-by-default",
                    &[("implementation", "crates/gateway/src/deny.rs")],
                ),
                node("C1", NodeKind::VerificationCase, "gateway deny test", &[]),
            ],
            evidence: vec![],
            relations: vec![
                edge("d1", "OBL", "R1", Derives),
                edge("d2", "OBL", "R2", Derives),
                edge("s1", "E", "R1", Satisfies),
                edge("i1", "CTL", "R1", Implements),
                edge("v1", "C1", "R1", Verifies),
            ],
        }
    }

    #[test]
    fn satisfaction_alone_is_not_verification() {
        let now = revs("m1", "i1");
        let report = analyze(&graph(), &[], &current(&now), None).unwrap();
        let r1 = report.requirement("R1").unwrap();
        assert_eq!(r1.assurance, Assurance::SatisfiedUntested);
        assert!(
            report
                .gaps_for("R1")
                .any(|g| *g == GapKind::SatisfiedUntested)
        );
        assert_eq!(r1.satisfied_by, ["E"]);
        assert_eq!(r1.enforced_by, ["CTL"]);
        assert_eq!(r1.sources, ["OBL"]);
    }

    #[test]
    fn unsatisfied_unknown_case_and_missing_case_are_gaps() {
        let now = revs("m1", "i1");
        let report = analyze(&graph(), &[], &current(&now), None).unwrap();
        let r2 = report.requirement("R2").unwrap();
        assert_eq!(r2.assurance, Assurance::Unsatisfied);
        let kinds: Vec<_> = report.gaps_for("R2").cloned().collect();
        assert!(kinds.contains(&GapKind::Unsatisfied));
        assert!(kinds.contains(&GapKind::NoVerificationCase));
        assert!(kinds.contains(&GapKind::UnknownVerificationCase {
            verification_id: "C-missing".into()
        }));
    }

    #[test]
    fn missing_control_and_unimplemented_control_are_implementation_gaps() {
        let now = revs("m1", "i1");
        // R2 has no enforcing control at all.
        let report = analyze(&graph(), &[], &current(&now), None).unwrap();
        assert!(
            report
                .gaps_for("R2")
                .any(|g| *g == GapKind::NoEnforcingControl)
        );
        assert!(
            !report
                .gaps_for("R1")
                .any(|g| *g == GapKind::NoEnforcingControl)
        );
        // A control that records no implementation is a gap even though it is linked.
        let mut g = graph();
        g.requirements
            .iter_mut()
            .find(|n| n.id == "CTL")
            .unwrap()
            .attributes
            .remove("implementation");
        let report = analyze(&g, &[], &current(&now), None).unwrap();
        assert!(report.gaps_for("R1").any(|g| *g
            == GapKind::ControlNotImplemented {
                control_id: "CTL".into()
            }));
    }

    #[test]
    fn fresh_pass_is_verified() {
        let now = revs("m1", "i1");
        let report = analyze(
            &graph(),
            &[record("R1", "C1", &now, VerificationResult::Pass)],
            &current(&now),
            None,
        )
        .unwrap();
        assert_eq!(
            report.requirement("R1").unwrap().assurance,
            Assurance::Verified
        );
        assert_eq!(report.gaps_for("R1").count(), 0);
    }

    #[test]
    fn changing_any_revision_makes_the_previous_result_stale() {
        let then = revs("m1", "i1");
        let ev = [record("R1", "C1", &then, VerificationResult::Pass)];
        for (label, now) in [
            ("model", revs("m2", "i1")),
            ("implementation", revs("m1", "i2")),
            (
                "configuration",
                RevisionSet {
                    configuration_digest: D2.into(),
                    ..revs("m1", "i1")
                },
            ),
        ] {
            let report = analyze(&graph(), &ev, &current(&now), None).unwrap();
            assert_eq!(
                report.requirement("R1").unwrap().assurance,
                Assurance::Stale,
                "{label}"
            );
            assert!(
                report.gaps_for("R1").any(|g| *g == GapKind::Stale),
                "{label}"
            );
        }
        match ev[0].freshness(&current(&revs("m2", "i1"))) {
            Freshness::Stale { drift } => {
                assert!(drift.model && !drift.implementation && !drift.configuration);
            }
            Freshness::Fresh => panic!("expected stale"),
        }
    }

    #[test]
    fn fresh_failure_beats_fresh_pass_and_stale_pass_does_not_hide_it() {
        let then = revs("m1", "i1");
        let now = revs("m2", "i1");
        let ev = [
            record("R1", "C1", &then, VerificationResult::Pass),
            record("R1", "C1", &now, VerificationResult::Fail),
        ];
        let report = analyze(&graph(), &ev, &current(&now), None).unwrap();
        assert_eq!(
            report.requirement("R1").unwrap().assurance,
            Assurance::Failing
        );
        // An Error is not a pass either.
        let ev = [record("R1", "C1", &now, VerificationResult::Error)];
        let report = analyze(&graph(), &ev, &current(&now), None).unwrap();
        assert_eq!(
            report.requirement("R1").unwrap().assurance,
            Assurance::Failing
        );
    }

    #[test]
    fn evidence_for_an_unlinked_case_is_orphaned_not_counted() {
        let now = revs("m1", "i1");
        let stray = record("R1", "C-other", &now, VerificationResult::Pass);
        let report = analyze(&graph(), std::slice::from_ref(&stray), &current(&now), None).unwrap();
        assert_eq!(
            report.requirement("R1").unwrap().assurance,
            Assurance::SatisfiedUntested
        );
        assert_eq!(report.orphan_evidence, vec![stray.key()]);
    }

    #[test]
    fn dangling_element_is_reported_against_the_model_revision() {
        let now = revs("m1", "i1");
        let known: BTreeSet<String> = BTreeSet::from(["Kr0ki::Other".to_string()]);
        let report = analyze(&graph(), &[], &current(&now), Some(&known)).unwrap();
        assert!(report.gaps_for("R1").any(|g| *g
            == GapKind::DanglingElement {
                element_id: "E".into()
            }));
        let known: BTreeSet<String> = BTreeSet::from(["Kr0ki::ToolGateway".to_string()]);
        let report = analyze(&graph(), &[], &current(&now), Some(&known)).unwrap();
        assert!(
            !report
                .gaps_for("R1")
                .any(|g| matches!(g, GapKind::DanglingElement { .. }))
        );
    }

    #[test]
    fn only_asserted_edges_count() {
        let mut g = graph();
        for e in &mut g.relations {
            if e.id == "s1" {
                e.authority = RelationAuthority::Proposed(
                    super::super::requirements::NonAuthoritativeRelation {
                        confidence: 0.9,
                        rationale: "model guess".into(),
                        evidence: vec![],
                        model: super::super::requirements::ModelIdentity {
                            name: "m".into(),
                            version: "1".into(),
                        },
                    },
                );
            }
        }
        let report = analyze(&g, &[], &current(&revs("m1", "i1")), None).unwrap();
        assert_eq!(
            report.requirement("R1").unwrap().assurance,
            Assurance::Unsatisfied
        );
    }

    #[test]
    fn missing_or_unknown_node_kind_is_an_error() {
        let mut g = graph();
        g.requirements[1].attributes.remove(NODE_KIND_ATTR);
        assert_eq!(
            analyze(&g, &[], &current(&revs("m1", "i1")), None),
            Err(ThreadError::MissingNodeKind("R1".into()))
        );
        let mut g = graph();
        g.requirements[1]
            .attributes
            .insert(NODE_KIND_ATTR.into(), "widget".into());
        assert!(matches!(
            analyze(&g, &[], &current(&revs("m1", "i1")), None),
            Err(ThreadError::UnknownNodeKind { .. })
        ));
    }

    #[test]
    fn profile_requirement_round_trips_and_reports_every_problem() {
        let p = ProfileRequirement {
            id: "KR-X".into(),
            statement: "The adapter shall preserve identifiers.".into(),
            source: "https://example.test/spec#4".into(),
            source_kind: SourceKind::BindingObligation,
            owner: "team-a".into(),
            rationale: "needed for reimport".into(),
            verification_id: "VC-X".into(),
            acceptance: "round trip is lossless".into(),
            status: RequirementStatus::Draft,
        };
        let mut n = node("KR-X", NodeKind::Requirement, &p.statement, &[]);
        n.attributes = p.to_attributes();
        assert_eq!(ProfileRequirement::from_node(&n).unwrap(), p);

        let mut bad = n.clone();
        bad.attributes.remove("owner");
        bad.attributes.insert("status".into(), "done".into());
        bad.attributes.insert("source_kind".into(), "".into());
        bad.text = " ".into();
        let issues = ProfileRequirement::from_node(&bad).unwrap_err();
        assert!(issues.contains(&ProfileIssue::EmptyStatement { id: "KR-X".into() }));
        assert!(issues.contains(&ProfileIssue::MissingAttribute {
            id: "KR-X".into(),
            attr: "owner"
        }));
        assert!(issues.contains(&ProfileIssue::MissingAttribute {
            id: "KR-X".into(),
            attr: "source_kind"
        }));
        assert!(issues.contains(&ProfileIssue::InvalidValue {
            id: "KR-X".into(),
            attr: "status",
            value: "done".into()
        }));
    }

    #[test]
    fn statement_lint_checks_what_a_scan_can_know() {
        assert!(statement_issues("The gateway shall deny writes outside the grant.").is_empty());
        assert_eq!(
            statement_issues("The gateway denies writes."),
            [StatementIssue::MissingShall]
        );
        assert_eq!(
            statement_issues("The gateway shall deny writes and shall log them."),
            [StatementIssue::MultipleShall(2)]
        );
        assert_eq!(
            statement_issues("Shall deny writes."),
            [StatementIssue::NoResponsibleComponent]
        );
        let vague = statement_issues("The UI shall be user-friendly and/or fast, TBD.");
        assert!(vague.contains(&StatementIssue::VagueTerm("user-friendly".into())));
        assert!(vague.contains(&StatementIssue::VagueTerm("and/or".into())));
        assert!(vague.contains(&StatementIssue::VagueTerm("tbd".into())));
        // `shallow` and `etcetera` are not `shall` / `etc`.
        assert_eq!(
            statement_issues("The cache shall keep shallow copies."),
            Vec::<StatementIssue>::new()
        );
    }

    #[test]
    fn evidence_validation_rejects_empty_fields_and_bad_digests() {
        let now = revs("m1", "i1");
        let ok = record("R1", "C1", &now, VerificationResult::Pass);
        assert_eq!(ok.validate(), Ok(()));
        let mut bad = ok.clone();
        bad.artifact_uri = " ".into();
        assert_eq!(bad.validate(), Err(EvidenceError::Empty("artifact_uri")));
        let mut bad = ok.clone();
        bad.artifact_digest = "md5:abc".into();
        assert!(matches!(
            bad.validate(),
            Err(EvidenceError::BadDigest {
                field: "artifact_digest",
                ..
            })
        ));
        let mut bad = ok;
        bad.configuration_digest = "sha256:XYZ".into();
        assert!(matches!(
            bad.validate(),
            Err(EvidenceError::BadDigest {
                field: "configuration_digest",
                ..
            })
        ));
    }

    #[test]
    fn configuration_digests_are_per_case_so_two_cases_can_both_be_fresh() {
        use super::super::requirements::RequirementRelationKind as K;
        // R1 is verified by C1; add C2 verifying R2 (a different case, a different digest).
        let mut g = graph();
        g.requirements
            .push(node("C2", NodeKind::VerificationCase, "second case", &[]));
        g.relations.push(edge("v2", "C2", "R2", K::Verifies));
        let now = revs("m1", "i1");
        let d_c1 = format!("sha256:{}", "a".repeat(64));
        let d_c2 = format!("sha256:{}", "b".repeat(64));
        let at = |digest: &str| RevisionSet {
            configuration_digest: digest.into(),
            ..now.clone()
        };
        let ev = [
            record("R1", "C1", &at(&d_c1), VerificationResult::Pass),
            record("R2", "C2", &at(&d_c2), VerificationResult::Pass),
        ];
        let cur = CurrentRevisions::new("m1", "i1")
            .with_case("C1", &d_c1)
            .with_case("C2", &d_c2);
        let report = analyze(&g, &ev, &cur, None).unwrap();
        for id in ["R1", "R2"] {
            let t = report.requirement(id).unwrap();
            assert!(
                t.evidence.iter().all(|e| e.freshness == Freshness::Fresh),
                "{id}: each case's evidence is fresh against its own digest: {t:?}"
            );
        }
        // Change only C2's configuration: C1's evidence stays fresh, C2's goes stale.
        let cur = CurrentRevisions::new("m1", "i1")
            .with_case("C1", &d_c1)
            .with_case("C2", &d_c1);
        let report = analyze(&g, &ev, &cur, None).unwrap();
        assert_eq!(
            report.requirement("R1").unwrap().assurance,
            Assurance::Verified
        );
        assert!(
            report
                .requirement("R2")
                .unwrap()
                .evidence
                .iter()
                .all(|e| matches!(e.freshness, Freshness::Stale { .. }))
        );
    }

    #[test]
    fn a_case_with_no_current_digest_cannot_be_confirmed_so_its_results_are_stale() {
        let now = revs("m1", "i1");
        let ev = record("R1", "C1", &now, VerificationResult::Pass);
        let no_digest = CurrentRevisions::new("m1", "i1");
        match ev.freshness(&no_digest) {
            Freshness::Stale { drift } => {
                assert!(drift.configuration && !drift.model && !drift.implementation);
            }
            Freshness::Fresh => panic!("an unconfirmable configuration must not be fresh"),
        }
    }

    #[test]
    fn vocabularies_round_trip_and_are_exposed_for_exchange_formats() {
        for k in NodeKind::ALL {
            assert_eq!(NodeKind::parse(k.as_str()), Some(k));
        }
        for k in SourceKind::ALL {
            assert_eq!(SourceKind::parse(k.as_str()), Some(k));
        }
        for k in RequirementStatus::ALL {
            assert_eq!(RequirementStatus::parse(k.as_str()), Some(k));
        }
        assert_eq!(attribute_vocabulary("status").unwrap().len(), 5);
        assert_eq!(attribute_vocabulary("node_kind").unwrap().len(), 5);
        assert_eq!(attribute_vocabulary("source_kind").unwrap().len(), 3);
        assert_eq!(attribute_vocabulary("owner"), None);
    }

    #[test]
    fn evidence_key_is_deterministic_per_revision_triple() {
        let a = record("R1", "C1", &revs("m1", "i1"), VerificationResult::Pass);
        let b = record("R1", "C1", &revs("m1", "i1"), VerificationResult::Pass);
        let c = record("R1", "C1", &revs("m2", "i1"), VerificationResult::Pass);
        assert_eq!(a.key(), b.key());
        assert_ne!(a.key(), c.key());
    }
}
