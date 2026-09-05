# Changelog

All notable changes to `ufo-types` are documented here. This file starts
with the `v0.11.0` release; earlier tags (`v0.10.1`, `v0.10.2`) are listed
for reference without a full itemized history.

## [Unreleased]

## [0.12.0] - pending tag

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

## [0.11.0] - pending tag

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
