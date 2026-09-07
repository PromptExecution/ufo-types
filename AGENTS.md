# AGENTS.md — ufo-types

UFO-grounded domain types for the b00t ecosystem. One small, dependency-light
crate: UFO stereotypes, the `Satisfies<C>` constraint trait, generic
agent/OODA capability types, and the **semantic-graph layer** (`iso_ir` →
`ontology` → `sysml_model`/`view`). `iso` is the only domain-specific module
(finance/legal, for Tax-Lawyer / `ledgrrr`).

## Module map

| layer | module | what | traits? | feature |
|---|---|---|---|---|
| stereotypes | `stereotype` | `UfoStereotype`, `UfoCategory`, `Stereotyped` | yes | — |
| constraints | `satisfies` | `Satisfies<C>`, `SatisfiesResult`, `Disposition`, `IsoAuditable` | yes | — |
| capability | `capability`, `dare` | `Task`/`Attempt`/`Episode`/…, DARED proposal types, `OodaStateMachine` | — | — |
| **graph floor** | `iso_ir` | free-form `Node` / `Edge` transport | — | — |
| **graph middle** | `ontology` | `UfoRelation` (closed, 25), `OntologicalEdge`, `TemporalExtent`, `SourceAnchor` | — | — |
| **graph viewpoints** | `sysml_model`, `view` | `ElementKind`, `Relation`, `ElementId`, `SysmlViewKind` (all closed, data-level) | — | — |
| export | `mbse`, `statechart` | SysML v2 `part` render; W3C SCXML bridge | — | `statechart` |
| syntax check | `sysml` | `validate_sysml_v2` (wraps `sysml-v2-parser`) | — | `sysml` (also via `python`) |
| domain | `iso` | `Lei`, `Isin`, `Currency`, `BankAccount`, `FinancialInstrument` | — | — |
| infra | `coherence`, `multi_model`, `data_format`, `model_capability` | numeric-agreement + generic multi-model verifier | mixed | — |

The semantic-graph layer is domain-**neutral**. The Kubernetes verb table,
tax rules beyond `iso`, per-lab extraction — those live in the consumer
(`kr0ki`, `ledger-core`, …), never here. Need a new canonical `UfoRelation`
variant or `ElementKind`? Open an issue; do not fork the enum.

## Hard invariants (do not break)

- **IDs are never numeric.** `ElementId(pub String)`, `OntologicalEdge.id`,
  every id newtype — opaque strings. Content-addressed forms use a scheme
  prefix (`sha256:`, `blake3:`, `git:` — see `ElementId::HASH_SCHEMES`).
- **No wall-clock, no uuids** in `SourceAnchor` / `TemporalExtent` — modeled
  time only (caller-supplied ISO-8601 / logical tick / commit-ref).
- The **strict canonical `FromStr`** on the closed enums is the stable
  surface. The **lenient** `UfoRelation::from_synonym` / case-insensitive
  parsing MAY be re-tuned between `0.MINOR`s — never depend on a specific
  synonym mapping.
- `sysml` pulls `sysml-v2-parser` (not wasm32-safe) — keep it `optional`,
  never a default feature.

## Versioning & release

Pre-1.0. A `0.MINOR` bump has **no** SemVer guarantee and MAY reverse
capability. `CHANGELOG.md` is the compatibility contract until `1.0.0` —
update it in the same PR as any public-surface change, and keep
`Cargo.toml` `version` in lockstep with the newest `## [x.y.z]` heading.

Cut a release (maintainer):

```
# 1. CHANGELOG.md: date the pending heading; Cargo.toml version matches
# 2. merge to main, then from main:
git tag -a vX.Y.Z -m "ufo-types vX.Y.Z"
git push origin vX.Y.Z
gh release create vX.Y.Z --title "vX.Y.Z" --notes-from-tag   # or --notes-file
# 3. (optional, irreversible) cargo publish   — decide per release; git-tag
#    consumers do not need it
```

Downstream pins live in README § "Downstream consumers" — bump them there and
in each consumer after a release: `b00t` (`stereotype`/`satisfies`/`capability`/
`dare`), `ledgrrr` (`+iso`), `kr0ki` (`iso_ir`/`ontology`/`sysml_model`/`view`),
`m0ltis` (planned: provider→ufo-types lowering).

## Build / check

```
cargo test                          # default features
cargo test --all-features           # + sysml, python, statechart
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

Python binding (`python` feature): `maturin develop` + `pytest python/`.

# b00t:map v1
# summary: ufo-types agent guide — module map (stereotype/satisfies/capability + iso_ir→ontology→sysml_model/view semantic-graph layer + iso domain), hard invariants (non-numeric ids, no wall-clock, strict-FromStr-is-stable), pre-1.0 release procedure, downstream pins
# tags: ufo-types, ufo, semantic-graph, ontology, sysml-v2, kerml, stereotype, satisfies, pre-1.0, release, b00tyverse
# tier: frontier
# cmds: cargo test --all-features; cargo clippy --all-targets --all-features -- -D warnings; git tag -a vX.Y.Z && gh release create vX.Y.Z
# complexity: 4
