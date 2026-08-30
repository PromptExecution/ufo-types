//! # ufo-types — UFO-grounded domain types for the b00t ecosystem
//!
//! This crate provides a domain-generic ontological foundation — grounded in
//! Guizzardi's Unified Foundational Ontology — for any b00t-ecosystem project
//! that wants ontologically-grounded types with deterministic, audit-ready
//! constraint evaluation. It defines:
//!
//! - **UFO stereotypes** (`stereotype`): `UfoStereotype` (every
//!   endurant/perdurant/moment substereotype from Guizzardi 2005, each
//!   carrying its own name) + `UfoCategory` (the top-level Endurant /
//!   Perdurant / Moment / Abstract classification, derivable from any
//!   stereotype via `.category()`). Domain-generic.
//! - **Satisfies<C> trait** (`satisfies`): The core constraint evaluation
//!   pattern — any domain type can implement `Satisfies<Constraint>` with
//!   deterministic, audit-ready results. Domain-generic.
//! - **Capability types** (`capability`): `Task`, `Attempt`, `ActionRecord`,
//!   `Episode`, `ReviewVerdict`, `Solution`, `TrainingCorpus`,
//!   `EnergyBudget`, etc. — generic agent-capability/OODA types.
//!   Domain-generic.
//! - **DARED proposal types** (`dare`): `Decision`, `Alternative`, `Risk`,
//!   `ExecutiveDecision`, `OodaStateMachine` — a generic OODA state-change
//!   proposal framework codified as Rust generics. Domain-generic.
//! - **Graph IR** (`iso_ir`): `Node`/`Edge` — the generic graph vocabulary a
//!   domain uses to describe connectivity before anything downstream lays
//!   it out or renders it. Domain-generic; promoted from `systhread-core`
//!   (narrower than that crate's own `iso_ir.rs` — layout/rendering and
//!   lab-specific extraction stay there).
//! - **MBSE export** (`mbse`): `MbseExport` — renders any `Stereotyped`
//!   type as a SysML v2 `part` usage, so evidence built from these types
//!   (a `DaredProposal`, a `Decision`) doubles as a systems-engineering
//!   model artifact. All of `dare`'s core types implement it; see
//!   `mbse`'s module docs for the one-line pattern to extend it.
//!   Domain-generic.
//! - **Statechart export** (`statechart`, feature-gated): bridges
//!   state-machine-shaped types (starting with `dare`'s `OodaPhase`/
//!   `OodaEvent`) to a W3C SCXML document via the `scxml` crate — one
//!   shared, standards-based, round-trippable representation instead of a
//!   bespoke per-domain renderer. `scxml` is a document model, not a
//!   runtime; it never replaces a domain's real transition-enforcement
//!   logic. Domain-generic.
//! - **ISO standard wrappers** (`iso`): `Lei` (ISO 17442), `Isin` (ISO
//!   6166), `Currency` (ISO 4217 + common crypto tickers), `BankAccount`
//!   (IBAN/BIC/LEI bundle), `FinancialInstrument` (IFRS 9). These ARE
//!   domain-specific — they encode financial/legal-entity accounting
//!   standards and are only meaningful to consumers working in that space
//!   (e.g. Tax-Lawyer, `ledgrrr`). Not intended as a generic building block
//!   for unrelated domains.
//!
//! Any b00t-ecosystem project needing UFO-grounded domain types and the
//! `Satisfies<T>` pattern (e.g. `stereotype`, `satisfies`, `capability`,
//! `dare`) can depend on it directly. Only `iso` carries genuinely
//! finance/tax-domain-specific types.
//!
//! ## Single source of truth
//!
//! `stereotype`/`satisfies`/`iso` were independently implemented twice
//! against the same original spec (gh#511) — once here (feeding `b00t-cli`,
//! `b00t-c0re-lib`, `b00t-lib-chat`, and cim-gridy's `mission-engine`) and
//! once in `ledgrrr`'s vendored `crates/ufo-types` (feeding `ledger-core`
//! and `ledgerr-mcp`). They have since been reconciled onto this crate:
//! `UfoStereotype` gained `ledgrrr`'s missing endurant/perdurant
//! substereotypes plus a `.category()` method (existing variants and
//! `Display` output unchanged — no consumer needed to change), and
//! `satisfies`/`iso` adopted `ledgrrr`'s exact `SatisfiesResult`/
//! `Disposition`/`NodeId`/`Lei`/`Currency`/`BankAccount`/
//! `FinancialInstrument` shapes (the ones with real production call sites),
//! superseding this crate's own former `Iso4217`/`Ifrs9Classification`
//! (zero external consumers, same ground now covered by
//! `Currency`/`FinancialInstrument`). `ledgrrr`'s vendored copy is retired
//! in favor of depending on this crate directly.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────┐     ┌───────────────┐     ┌────────────────┐
//! │ ufo-types │────→│  ledger-core  │────→│  MCP actions   │
//! │ (traits)  │     │ (domain impls)│     │ (thin wrappers)│
//! └──────────┘     └───────────────┘     └────────────────┘
//! ```
//!
//! (The above pipeline is Tax-Lawyer's own consumption path; other
//! consumers wire `ufo-types` into their own domain-impl layer instead of
//! `ledger-core`.)
//!
//! ## Integration with evidence layer (NS-9, NS-10)
//!
//! The `Stereotyped` trait (from `stereotype`) and `IsoAuditable` trait
//! (from `satisfies`) bridge domain types to `evidence.rs`:
//!
//! - **NS-9** `record_is_a(subject, ufo_stereotype)` — uses `Stereotyped::ufo_stereotype()`
//! - **NS-10** `record_audited_by(subject, iso_standard)` — uses `IsoAuditable::iso_standard_ids()`
//!
//! The `EvidenceBridge::evaluate()` method on `SatisfiesResult` produces
//! all labels needed for both calls in one step.
//!
//! ## References
//!
//! - Guizzardi, G. (2005). _Ontological Foundations for Structural
//!   Conceptual Models_. PhD Thesis, University of Twente.
//! - ISO 17442:2012 — Legal Entity Identifier (LEI) (`iso` module only)
//! - ISO 6166:2021 — International Securities Identification Number (ISIN) (`iso` module only)
//! - ISO 4217:2015 — Codes for the representation of currencies (`iso` module only)
//! - ISO 13616 / ISO 9362 — IBAN / BIC (`iso` module only)
//! - IFRS 9 — Financial Instruments (IASB, 2014) (`iso` module only)
//!
//! The following references are specific to the Tax-Lawyer consumer and its
//! use of the domain-generic types above — they are not properties of this
//! crate itself:
//! - ITAA 1997 Division 355 — AU R&D Tax Incentive
//! - IRC Sec 41 — US R&D Tax Credit
//! - IRS Rev. Proc. 2024-28 — Crypto cost basis safe harbor
//! - ATO QC 53725 — AU crypto CGT treatment

pub mod capability;
pub mod coherence;
pub mod dare;
pub mod iso;
pub mod iso_ir;
pub mod mbse;
pub mod multi_model;
#[cfg(feature = "python")]
mod python;
pub mod satisfies;
#[cfg(feature = "statechart")]
pub mod statechart;
pub mod stereotype;
#[cfg(feature = "sysml")]
pub mod sysml;

// Re-export key types for convenience
pub use capability::{
    ActionRecord, AgentCapability, Attempt, AttemptStatus, CapabilityDomain, CarmackSolution,
    EnergyBudget, Episode, History, ReviewVerdict, ReviewerType, Solution, StateObservation, Task,
    TaskStatus, TrainingCorpus,
};
pub use coherence::{NumericAgreement, validate_numeric_agreement};
pub use dare::{
    Alternative, DaredAcceptanceCriteria, DaredDocument, DaredProposal, DaredValidationError,
    Decision, ExecutiveDecision, OodaEvent, OodaGuards, OodaPhase, OodaStateMachine,
    OodaStateMachineError, OodaTransition, Risk, RiskSeverity,
};
pub use iso::{BankAccount, Currency, FinancialInstrument, Isin, IsoValidationError, Lei};
pub use iso_ir::{Edge, Node};
pub use mbse::{MbseExport, indent_block, mbse_field_dump, sanitize_ident};
pub use multi_model::{MockModelClient, ModelClient, MultiModelConfig, MultiModelVerifier};
pub use satisfies::{
    Constraint, Disposition, EvidenceBridge, IsoAuditable, NodeId, Satisfies, SatisfiesResult,
};
#[cfg(feature = "statechart")]
pub use statechart::{CANCELLED_STATE_ID, ooda_phases_to_statechart};
pub use stereotype::{Stereotyped, UfoCategory, UfoStereotype};
#[cfg(feature = "sysml")]
pub use sysml::{SysmlV2Syntax, validate_sysml_v2};
