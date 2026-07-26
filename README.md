# ufo-types

UFO-grounded domain types for the b00t ecosystem.

This crate provides a domain-generic ontological foundation — grounded in
Guizzardi's Unified Foundational Ontology (UFO) — for any project that wants
ontologically-grounded types with deterministic, audit-ready constraint
evaluation. It defines:

- **UFO stereotypes** (`stereotype`): `UfoStereotype` enum grounding every
  domain type in Guizzardi's Unified Foundational Ontology. Domain-generic —
  nothing here is specific to any one consumer.
- **`Satisfies<C>` trait** (`satisfies`): The core constraint evaluation
  pattern — any domain type can implement `Satisfies<Constraint>` with
  deterministic, audit-ready results. Domain-generic.
- **Capability types** (`capability`): `Task`, `Attempt`, `ActionRecord`,
  `Episode`, `ReviewVerdict`, `Solution`, `TrainingCorpus`, `EnergyBudget`,
  etc. — generic agent-capability/OODA types. Domain-generic.
- **DARED proposal types** (`dare`): `Decision`, `Alternative`, `Risk`,
  `ExecutiveDecision`, `OodaStateMachine` — a generic OODA state-change
  proposal framework codified as Rust generics. Domain-generic.
- **ISO standard wrappers** (`iso`): `Lei` (ISO 17442 Legal Entity
  Identifier), `Iso4217` currency codes, and `Ifrs9Classification`
  financial-instrument types. These ARE domain-specific — they encode
  financial/legal-entity accounting standards and are only meaningful to
  consumers working in that space (e.g. Tax-Lawyer platforms). Not intended
  as a generic building block for unrelated domains.

Any project needing UFO-grounded domain types and the `Satisfies<T>` pattern
(e.g. `stereotype`, `satisfies`, `capability`, `dare`) can depend on this
crate directly. Only `iso` carries genuinely finance/tax-domain-specific
types.

## Architecture

```text
┌───────────┐     ┌───────────────┐     ┌────────────────┐
│ ufo-types │────→│ domain impls  │────→│  MCP actions   │
│ (traits)  │     │ (consumer)    │     │ (thin wrappers)│
└───────────┘     └───────────────┘     └────────────────┘
```

Consumers wire `ufo-types` into their own domain-impl layer — for example
`ledger-core` (part of [PromptExecution/ledgrrr](https://github.com/PromptExecution/ledgrrr))
implements `Stereotyped` for its own artifact-classification types.

## Integration with evidence layers

The `Stereotyped` trait (from `stereotype`) and `IsoAuditable` trait (from
`satisfies`) are designed to bridge domain types to an audit-evidence layer:

- `record_is_a(subject, ufo_stereotype)` — uses `Stereotyped::ufo_stereotype()`
- `record_audited_by(subject, iso_standard)` — uses `IsoAuditable::iso_standard_ids()`

The `EvidenceBridge::evaluate()` method on `SatisfiesResult` produces all
labels needed for both calls in one step.

## Origin

This crate was extracted from [elasticdotventures/_b00t_](https://github.com/elasticdotventures/_b00t_)
(`crates/ufo-types`), where it originated as part of the b00t governance
platform, with git history preserved. It is consumed both inside `_b00t_`
(`b00t-c0re-lib`, `b00t-lib-chat`) and by external projects (e.g.
`critter-keeper` in [app4dog](https://github.com/app4dog)) that only need
this small, dependency-light types crate without pulling in the full
`_b00t_` monorepo (34 git submodules).

## References

- Guizzardi, G. (2005). _Ontological Foundations for Structural Conceptual
  Models_. PhD Thesis, University of Twente.
- ISO 17442:2012 — Legal Entity Identifier (LEI) (`iso` module only)
- ISO 4217:2015 — Codes for the representation of currencies (`iso` module
  only)
- IFRS 9 — Financial Instruments (IASB, 2014) (`iso` module only)

## License

MIT — see [LICENSE](./LICENSE).
