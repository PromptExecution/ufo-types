# Changelog

All notable changes to `ufo-types` are documented here. This file starts
with the `v0.11.0` release; earlier tags (`v0.10.1`, `v0.10.2`) are listed
for reference without a full itemized history.

## [Unreleased]

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
