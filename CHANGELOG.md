# Changelog

All notable changes to `ufo-types` are documented here. This file starts
with the `v0.11.0` release; earlier tags (`v0.10.1`, `v0.10.2`) are listed
for reference without a full itemized history.

**Versioning:** `ufo-types` is pre-1.0. Under Cargo SemVer a `0.x.y` bump
carries *no* compatibility guarantee — a `0.MINOR` bump MAY remove, rename,
or otherwise reverse capability. Entries below noting "additive, non-breaking"
describe intent as a courtesy; they are not a contract until `1.0.0`. Pin an
exact version (`ufo-types = "=0.14.0"`) or a git tag, and read this file
before bumping. See README § "Versioning & stability".

## [0.16.0] - Unreleased

### Added
- Checkpoint dialect documentation and query validation (issue #34) distinguish the opaque
  model dialect (`SysML-v2`, for example) from the portable format URI; project
  binding and supported-model checks remain adapter obligations.
- Shared revision discovery request/response, exact/minimum/current selectors,
  immutable graph descriptors, explicit freshness/pending/unavailable outcomes
  and typed RDF SELECT/ASK results. Decode validates identities, graph byte
  digest consistency, term syntax, wire bounds and duplicate JSON keys. Pure
  request matching cannot prove accepted ancestry, grants or graph completeness;
  adapters remain responsible for those and evaluator cancellation. Optional
  `oxiri`/`oxilangtag` reuse mature IRI/language validation under `revision`.
  Schema selectors add `query-request`, `query-response`, `graph-descriptor`.
- Canonical `Relation::FeatureTyping` with independent feature/type endpoints and
  stable relation identity (issue #33). Portable validation checks usage/definition
  categories; adapters must additionally prove native subtype/library compatibility.
  Older wire consumers reject the new variant; negotiate `feature_typing` support.
- The `revision-schema` example exports bundle, model, changeset, conflict, merge,
  receipt, checkpoint, status, expected-head, fidelity and capability JSON schemas
  directly from the canonical Rust contracts for browser code generation.
- Native `VerificationCaseDefinition` / `VerificationCaseUsage` element-kind pair,
  canonical metaclass/wire names and definition/usage helpers (issue #32). Older
  consumers reject these new wire variants; negotiate adapter capabilities before
  publication. This type addition does not establish real-server retention.
- Opt-in `revision` portable model contract: distinct validated project/revision/operation identities,
  typed values and multiplicity, existing SysML relations and source anchors, deterministic
  SHA-256 bundles preserving original artifact bytes, and explicit fidelity/index status data.
  Model validation and bundle hydration reject dangling references, missing/corrupt artifacts,
  incompatible schemas and undeclared data loss. No storage or transport implementation.
- Revision `ChangeSet` and deterministic three-way merge: independently edited fields,
  properties and extension keys merge; competing edits return typed conflict snapshots.
  Whole-model revalidation rejects removed endpoints, multiple owners and owning cycles.
  Change sets bind base/result digests and reject stale or altered preconditions.
- Complete-bundle digest binds context and original artifact bytes separately from semantic
  equivalence. Adapter capability checks fail closed on unverified typed property and fact
  authority preservation, including older probe records without those claims.
- Real values validate as arbitrary-precision decimals while retaining exact original lexemes;
  valid values beyond binary floating-point range no longer fail hydration.
- `quantity`: `Dimension` (ISQ base exponents + `information` + `money`), `Unit`
  (positive scale, optional offset, currency identity), finite `Interval`
  arithmetic, `Quantity`, and `ExchangeRate`. Concept transferred from SysMD's
  quantities/constraint values (`tukcps/SysMD`, Apache-2.0); no code copied, no
  solver. **Currencies are distinct units, never implicitly convertible:**
  `Quantity::add`/`convert_to` reject mixed currencies
  (`QuantityError::CurrencyMismatch`); the only way across is an explicit
  `ExchangeRate` carrying caller-supplied `as_of` + `source` (no clock read).
  `°C`/`°F` convert but refuse arithmetic. Money is a *value*; attribution to a
  cost center is a separate upstream concern and is not modeled here.
- `verdict`: `RangeVerdict` (`Entailed` / `Narrowed` / `Inconsistent`),
  `QuantityBound`, `check_bound`, and `Satisfies<QuantityBound> for Quantity`.
  `Narrowed` maps to `Disposition::Unknown` (an over-approximation is not a
  proof); `Disposition` is unchanged.
- `view_definition`: `ViewDefinition` / `ViewUsage` as data with OMG-verbatim
  vocabulary: `Expose` (`membership`, `members_of` = `A::*`, `all_under` =
  `A::**`), conjunctive `ViewFilter { any_of }`, `RenderingChoice` over the four
  standard `RenderingUsage`s of the normative `Views` library, and
  `ModelIndex::select` -> `ViewSelection { elements, relations, unresolved }`
  (pure, deterministic, cycle-safe; relations feed an existing renderer).
- `mbse::assurance`: the versioned assurance-thread profile over `RequirementGraph` —
  `NodeKind` (source obligation, requirement, system element, control, verification case),
  `SourceKind` (binding obligation / organisational policy / guidance), `ProfileRequirement`
  (the nine required fields), `statement_issues` (deterministic statement lint),
  `EvidenceRecord` (revision-bound), `CurrentRevisions` (global model + implementation
  revision, **per-case** configuration digest), `Freshness`, and `analyze` -> `ThreadReport`
  with an `Assurance` state per requirement (`Unsatisfied`, `SatisfiedUntested`, `Verified`,
  `Failing`, `Stale`) and typed `GapKind`s. Keeps a satisfaction assertion (architectural)
  apart from a verification result (revision-specific evidence).
- `model_edit`: `ModelEdit::{HasA, IsA}` — qualified-name-addressed incremental
  edits (the SysMD notebook `hasA`/`isA` triple dialect), parse + print only.

## [0.15.0] - 2026-09-19

### Added
- `mbse::requirements`: `RequirementGraph`/`Requirement`/`RequirementRelation`
  and the five induced viewpoints (decomposition, traceability, impact,
  behaviour, verification coverage), vendored from kr0ki-core's
  `requirements` module (kr0ki M1 — the backend-neutral ReqIF/Flexo
  requirements semantic model, previously a local kr0ki-core integration
  seam). Preserves asserted/inferred/proposed relation authority and
  explicit `promote_relation()`; emits no renderer source. kr0ki-core now
  re-exports this module rather than defining its own copy.
  (`PromptExecution/kr0ki` `HANDOFF-2026-09-19-reqif-flexo.md`,
  lead-developer action #1)
- `sysgraph`: `SysGraph`/`OntologicalNode` — the box-2 typed envelope
  holding a full snapshot of UFO-stereotyped nodes plus `ontology::
  OntologicalEdge`s, per kr0ki `docs/TODO.md` box 2 and `DESIGN-NOTE-
  typed-model-layer.md` §2.6/§2.8 (no `SchemaVersion` field). serde JSON,
  round-trip tested; a `dangling_edges()` caller-invoked integrity check.
  (`PromptExecution/kr0ki#roadmap-box-2`)
- `pipeline_types`: `StageSpec`/`CapsuleProfile`/`StagePort`/`PipelineDag` and
  related types (port compatibility negotiation, error routing, DAG wiring
  with topological ordering and cycle detection), extracted from
  `b00t-cli`'s `pipeline_types.rs` (`elasticdotventures/_b00t_#1251`).
- `pipeline_flowctl`: `FlowStrategy`/`FlowControl`/`FlowGate`/
  `StageFlowConfig` for back-pressure and throttling between pipeline
  stages, extracted from `b00t-cli` (`elasticdotventures/_b00t_#1251`).
- `pipeline_secrets`: `SecretRef`/`SecretStore`/`SecureStageEnv` for
  resolving and injecting secrets into stage environments without exposing
  them in pipeline definitions, extracted from `b00t-cli`
  (`elasticdotventures/_b00t_#1251`).
- `keyring` (optional feature): OS keyring backend for
  `pipeline_secrets::SecretSource::Keyring`
  (`elasticdotventures/_b00t_#1251`).
- New mandatory dependencies `rpassword` and `shellexpand`, used by
  `pipeline_secrets`'s `Prompt` and `File` secret sources respectively
  (`elasticdotventures/_b00t_#1251`).

## [0.14.0] - 2026-09-07

First tag of the **semantic-graph layer** (`iso_ir` → `ontology` →
`sysml_model`/`view`). `v0.12.0`, `v0.13.0`, and `v0.14.0` are tagged
together on this date from `main`; the layer landed incrementally in
`#20`/`#21`/`#22` but was previously unreleased. Downstream projects
(`b00t`, `ledgrrr`, `kr0ki`, `m0ltis`) may now pin `=0.14.0`.

Additive, non-breaking release. No public API was removed or changed
incompatibly since `v0.13.0`.

Added:
- `sysml_model`: `ElementId` now documents and recognizes `blake3:` as a
  sanctioned content-hash scheme alongside `sha256:` and `git:`. The
  newtype itself is unchanged — still `pub struct ElementId(pub String)`,
  opaque; this is purely additive.
  - `ElementId::HASH_SCHEMES` — the `&'static [&'static str]` of
    recognized content-hash / VCS scheme prefixes (`"sha256:"`,
    `"blake3:"`, `"git:"`).
  - `ElementId::content_hash_scheme()` — the scheme prefix this id
    carries if it is content-addressed, else `None`.
  - `ElementId::is_content_addressed()` — whether this id is a content /
    VCS hash rather than a human-meaningful name.
  - The `ElementId` doc comment now lists `blake3:…` among the sanctioned
    content / VCS hash forms; the **never numeric** rule is unchanged.

## [0.13.0] - 2026-09-07

Additive, non-breaking release. No public API was removed or changed
incompatibly since `v0.12.0`.

Added:
- `ontology`: the canonical UFO-typed semantic-graph edge layer that sits
  **above** `iso_ir::{Node, Edge}` (the free-form transport floor) and
  **below** `sysml_model::{ElementKind, Relation}` (the SysML-v2 viewpoint
  layer). Pure data — no traits. Not feature-gated. Domain-neutral: the
  Kubernetes verb-classification table itself lives downstream in `kr0ki`
  docs, not here.
  - `UfoRelation` — a **closed** (`#[non_exhaustive]`), 25-variant
    vocabulary of canonical ontological relations (`Specializes`,
    `Instantiates`, `HasPart`, `MemberOf`, `ScopedBy`, `HostedBy`,
    `Controls`, `Observes`, `Selects`, `ResolvesTo`, `Binds`, `Provides`,
    `Requires`, `RoutesTo`, `ParticipatesIn`, `Initiates`, `Precedes`,
    `Causes`, `Transitions`, `FlowsTo`, `Satisfies`, `Verifies`,
    `GovernedBy`, `AuthorizedBy`, `TracesTo`) that normalizes overloaded
    domain verbs, Kubernetes especially. Carries `ALL`, `canonical_name()`,
    `category_signature()` (expected `(source, target)` UFO categories),
    `permits()`, `is_structural()` / `is_occurrence()`, a **strict**
    `FromStr` (canonical snake_case only, mirroring `sysml_model`), and a
    **separate lenient** `from_synonym()` mapping the documented synonyms
    (`"runs-on"` → `HostedBy`, `"lists/watches"` → `Observes`,
    `"forwards-to"` → `RoutesTo`, …) for raw k8s / OpenAPI verb
    normalization. Records the explicit non-conflations `Controls` ≠
    `HasPart`, `HostedBy` ≠ `MemberOf`, `Requires` ≠ temporal invocation,
    `RoutesTo` (configured topology) ≠ `FlowsTo` (occurrence).
  - `OntologicalEdge` — the typed UFO semantic-graph edge: opaque `id`
    (never numeric), `ElementId` source/target (reusing the `sysml_model`
    newtype), a `UfoRelation`, an optional `TemporalExtent`, and a
    `Vec<SourceAnchor>` of 0..n attestations. Helpers `endpoints()` and
    `is_attested()`. Distinct from the free-form `iso_ir::Edge`.
  - `TemporalExtent` — `#[non_exhaustive]` `Instant` / `Interval` /
    `Ongoing`, `String` bounds (caller picks ISO-8601 / logical tick /
    commit-ref). Modeled occurrence time, never a crate-generated
    timestamp.
  - `SourceAnchor` — `#[non_exhaustive]` deterministic provenance
    (`RustSpan`, `SymbolPath`, `KermlQualifiedName`, `SysmlFile`,
    `K8sObject`, `Vcs`, `Other`); no uuids, no wall-clock.

## [0.12.0] - 2026-09-07

Additive, non-breaking release. No public API was removed or changed
incompatibly since `v0.11.0`.

Added:
- `view`: `SysmlViewKind` — a **closed**, data-level enum of the standard
  SysML **v2** `ViewDefinition` kinds (`Tree`, `General`, `Interconnection`,
  `ActionFlow`, `StateTransition`, `Sequence`, `Case`, `Geometry`, `Grid`,
  `Browser`), mirroring the normative `Views` package of the SysML v2
  standard library. Carries `view_def_name()` (`sysml.library` PascalCase
  id), `ALL`, `implemented_by_syson()` (the four Eclipse SysON view
  providers), and a case/whitespace-insensitive `FromStr`. Not
  feature-gated.
- `sysml_model`: the closed, data-level SysML v2 / KerML abstract-syntax
  layer that sits above `iso_ir::{Node, Edge}` and below any renderer.
  Pure data — no traits, no viewpoint trait hierarchy (all explicitly
  rejected in kr0ki `DESIGN-NOTE-typed-model-layer.md` §2.5):
  - `ElementId` — opaque id newtype (`pub struct ElementId(pub String)`),
    with `From<String>`/`From<&str>`/`Display`/`AsRef<str>`; never numeric
    (§2.4).
  - `ElementKind` — `#[non_exhaustive]`, 24 KerML/SysML-v2 abstract-syntax
    kinds, def/usage paired (`is_definition`/`is_usage`/`usage_of`/
    `definition_of`), with `kerml_name()`, `ALL`, and a case-insensitive
    `FromStr`. No domain variants — those stay `iso_ir` `part_type` strings
    (§2.1).
  - `Relation` — `#[non_exhaustive]` struct-variant sum type over the
    KerML/SysML-v2 relationship set (`FeatureMembership`, `Specialization`,
    `Subsetting`, `Redefinition`, `Connection`, `Succession`, `Allocation`,
    `Satisfy`, `Verify`, `Refine`, `Dependency`) plus the single `Domain`
    escape hatch, with `endpoints()` and `kerml_name()` (§2.2).
- `view` + `sysml_model` together are the SysML **v2** replacement for the
  rejected SysML 1.x `DiagramKind` / `BehaviorDiagram` / `StructureDiagram`
  / `UmlRelation` taxonomy; v2 is not a UML 2 derivative, so there is
  deliberately no relationship-provenance enum and no behavior-vs-structure
  grouping.

## [0.11.0] - 2026-08-30

Additive, non-breaking release. No public API was removed or changed
incompatibly since `v0.10.2`.

Added:
- `iso_ir`: generic `Node`/`Edge` graph vocabulary, promoted from
  `systhread-core` (#5).
- `data_format`: canonical `DataFormat` enum (#12).
- `model_capability`: `ModelCapability` type (#12).
- `coherence`: `NumericAgreement` constraint for cross-model numeric
  agreement checks (#14).
- `multi_model`: `ModelClient` trait and `MultiModelVerifier` for
  generic multi-model propose/review workflows (#14).
- `sysml`/`mbse`: opt-in SysML v2 export, including real `Vec`/`Option`
  multiplicity and nested-struct parts; `validate_sysml_v2` exposed to
  Python via PyO3 + maturin, with a pytest gate in CI.
- `statechart` (feature-gated): SCXML export for state-machine-shaped
  types (starting with `dare`'s `OodaPhase`/`OodaEvent`).
- `stereotype`/`satisfies`/`iso`: consolidated onto `ledgrrr`'s
  real-usage shape as the single source of truth shared with the
  vendored copy in `ledgrrr`.

Chore:
- repo-wide `cargo fmt` pass, no logic changes (#7).

## [0.10.2] - 2026-08-02

Standalone-ized `Cargo.toml`, added `README`/`LICENSE`, added a
standalone CI quality gate (#1).

## [0.10.1] - 2026-07-26

Baseline standalone release of `ufo-types` split out of the monorepo.
