# Foundry Local Epic — Phase 3: Generic Multi-Model Verifier + Numeric Coherence

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generalize `ledgrrr`'s existing rule-repair-specific `MultiModelVerifier`/`ModelClient` proposer/reviewer pattern into reusable `ufo-types` primitives, and add a numeric "coherence" (N-way agreement) sanity check via the established `Satisfies<C>` idiom — the two things the epic owner named as "trust": *"a process for reconciling"* (proposer/reviewer, already real, just not reusable) and *"two or more models agreeing"* (coherence, genuinely new).

**Architecture:** Two independent new `ufo-types` modules (Tasks 1-2, this repo) plus a consumer repoint in `ledgrrr` (Task 3, separate repo/PR) that removes local duplication in favor of the new shared primitives — following this epic's own established consolidation precedent (`elasticdotventures/_b00t_#1177` P0: "independently implemented twice... reconciled onto this crate"). Neither new module depends on Phase 1's `DataFormat`/`ModelCapability` types — this branch forks directly from `main`, independent of Phase 1's still-unmerged branch.

**Tech Stack:** Rust, `serde`/`serde_json` (already unconditional deps), `anyhow` (new unconditional dependency — small, ubiquitous, matches `verify.rs`'s existing `anyhow::Result` signatures exactly, needed for a faithful port).

**Spec:** https://github.com/PromptExecution/ufo-types/issues/13 (this repo's tracking issue), https://github.com/PromptExecution/ledgrrr/issues/218 (companion consumer issue, Task 3)

## Global Constraints

- No new Cargo feature for either new module — `anyhow` is the only new dependency, and it's small/ubiquitous like `serde_json`; add it unconditionally to `[dependencies]`, not behind a feature flag (matches this crate's convention: only heavy/optional deps like `sysml-v2-parser`/`scxml`/`pyo3` get feature-gated).
- `Satisfies<C>` idiom for Task 2: a zero-sized (or field-carrying, here `NumericAgreement { tolerance: f64 }` genuinely needs a field) constraint marker struct implementing `Constraint`, a `Satisfies<Marker>` impl on the subject type, and a plain free function doing the real work — mirrors `src/sysml.rs`/`src/data_format.rs`'s exact existing pattern.
- Task 1's `MultiModelVerifier<C>` does NOT build prompts internally (unlike the code it's generalized from) — it only orchestrates "call model, extract structured JSON, check a confidence threshold." Prompt construction and domain-specific payload types (`RepairProposal`, `ReviewResult`, etc.) are the CALLER's responsibility and stay in the consuming crate (`ledgrrr`, Task 3). This is the actual generalization: separating orchestration mechanics from domain-specific prompt text.
- Derive `Debug, Clone, Serialize, Deserialize` on `MultiModelConfig` (matches the original `verify.rs` derive list exactly — no `PartialEq`/`Eq`/`JsonSchema` needed here, this type isn't part of the crate's UFO/evidence-layer surface the way Phase 1's types were).
- Preserve `MultiModelConfig::default()`'s exact values from the code being generalized: `proposer_model: "claude-sonnet-4-5"`, `reviewer_model: "claude-haiku-4-5"`, `min_reviewer_confidence: 0.80` — a downstream test in `ledgrrr` (Task 3, out of scope for this repo's tasks but load-bearing) asserts these exact strings/value.
- Build and test entirely with plain `cargo build`/`cargo test` on this machine — this crate is plain cross-platform Rust with no Windows-only dependencies, and this session confirmed WSL genuinely cannot link anything (`error: linker 'cc' not found`, even for vanilla proc-macro build scripts). Work happens on the `D:\promptjects\ufo-types` clone (mirrored 1:1 at `/mnt/d/promptjects/ufo-types` — edit files there directly, no copying needed), and every `cargo` invocation MUST go through `pwsh.exe -NoProfile -Command "cd D:\promptjects\ufo-types; <cargo command> -j 2"`. This crate builds fast in this environment (Phase 1's full test suite ran in well under a minute once warm) — unlike `_b00t_`'s 30-crate monorepo, there is no need for `run_in_background`/long waits here; a plain foreground `cargo test` invocation is fine.
- Plain `git` commands (`add`/`commit`/`diff`/`log`) run directly from WSL against `/mnt/d/promptjects/ufo-types` — git needs no linker.

---

### Task 1: Generic `ModelClient` + `MultiModelVerifier`

**Files:**
- Create: `src/multi_model.rs`
- Modify: `src/lib.rs` (add module declaration + re-export)
- Modify: `Cargo.toml` (add `anyhow` dependency)
- Test: inline `#[cfg(test)] mod tests` in `src/multi_model.rs`

**Interfaces:**
- Consumes: nothing new from this crate (only `serde::de::DeserializeOwned`, already available).
- Produces: `pub trait ModelClient: Send + Sync { fn complete(&self, prompt: &str, max_tokens: usize) -> anyhow::Result<String>; fn extract<T: DeserializeOwned>(&self, prompt: &str) -> anyhow::Result<T>; }`, `pub struct MockModelClient` (with `MockModelClient::default()` and `.with_response(impl Into<String>)`), `pub struct MultiModelConfig { pub proposer_model: String, pub reviewer_model: String, pub min_reviewer_confidence: f32 }` (with `Default`, `MultiModelConfig::new(proposer, reviewer)`, `.with_threshold(f32)`), `pub struct MultiModelVerifier<C: ModelClient> { ... }` with `MultiModelVerifier::new(proposer: C, reviewer: C, config: MultiModelConfig) -> Self`, `.config(&self) -> &MultiModelConfig`, `.propose<P: DeserializeOwned>(&self, prompt: &str) -> anyhow::Result<P>`, `.review_raw<R: DeserializeOwned>(&self, prompt: &str) -> anyhow::Result<R>`, `.meets_confidence_threshold(&self, confidence: f32) -> bool`. Consumed by `ledgrrr`'s `ledger-core::verify` module (Task 3, separate repo/PR, tracked at https://github.com/PromptExecution/ledgrrr/issues/218).

- [ ] **Step 1: Add the anyhow dependency**

In `Cargo.toml`'s `[dependencies]` section, add (alongside the existing `thiserror = "2"` line, keeping the file's existing ordering — add it right after that line):

```toml
anyhow = "1"
```

- [ ] **Step 2: Write the failing test**

Create `src/multi_model.rs` with:

```rust
//! Generic multi-model proposer/reviewer orchestration — generalized from
//! `ledgrrr`'s `ledger-core::verify` module (rule-repair-specific), per this
//! crate's established consolidation precedent: independently-duplicated
//! logic gets reconciled onto one canonical shape here, the same way
//! `Satisfies`/`Stereotyped` were (see this crate's top-level doc comment).
//!
//! This module deliberately does NOT build prompts — it only orchestrates
//! "call a model, extract structured JSON, check a confidence threshold."
//! Prompt construction and domain-specific payload types (what "propose" and
//! "review" mean for a given problem) are the caller's responsibility.

use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

/// Trait for model invocation. Implementors substitute different LLM
/// providers, or a mock for testing.
pub trait ModelClient: Send + Sync {
    /// Generate a raw text completion from the model.
    fn complete(&self, prompt: &str, max_tokens: usize) -> anyhow::Result<String>;

    /// Extract structured output (JSON) from the model's response to `prompt`.
    fn extract<T: DeserializeOwned>(&self, prompt: &str) -> anyhow::Result<T>;
}

/// Mock model client for testing — always returns a fixed response.
pub struct MockModelClient {
    response: String,
}

impl MockModelClient {
    pub fn with_response(mut self, response: impl Into<String>) -> Self {
        self.response = response.into();
        self
    }
}

impl Default for MockModelClient {
    fn default() -> Self {
        Self {
            response: "mock response".to_string(),
        }
    }
}

impl ModelClient for MockModelClient {
    fn complete(&self, _prompt: &str, _max_tokens: usize) -> anyhow::Result<String> {
        Ok(self.response.clone())
    }

    fn extract<T: DeserializeOwned>(&self, _prompt: &str) -> anyhow::Result<T> {
        serde_json::from_str(&self.response).map_err(|e| anyhow::anyhow!(e))
    }
}

/// Configuration for a proposer/reviewer pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiModelConfig {
    pub proposer_model: String,
    pub reviewer_model: String,
    pub min_reviewer_confidence: f32,
}

impl Default for MultiModelConfig {
    fn default() -> Self {
        Self {
            proposer_model: "claude-sonnet-4-5".to_string(),
            reviewer_model: "claude-haiku-4-5".to_string(),
            min_reviewer_confidence: 0.80,
        }
    }
}

impl MultiModelConfig {
    pub fn new(proposer: impl Into<String>, reviewer: impl Into<String>) -> Self {
        Self {
            proposer_model: proposer.into(),
            reviewer_model: reviewer.into(),
            min_reviewer_confidence: 0.80,
        }
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.min_reviewer_confidence = threshold;
        self
    }
}

/// Orchestrates a proposer model and a reviewer model against a shared
/// confidence threshold. Does not know or care what "proposal" and "review"
/// mean domain-wise — the caller builds prompts and supplies payload types.
pub struct MultiModelVerifier<C: ModelClient> {
    proposer: C,
    reviewer: C,
    config: MultiModelConfig,
}

impl<C: ModelClient> MultiModelVerifier<C> {
    pub fn new(proposer: C, reviewer: C, config: MultiModelConfig) -> Self {
        Self {
            proposer,
            reviewer,
            config,
        }
    }

    pub fn config(&self) -> &MultiModelConfig {
        &self.config
    }

    /// Ask the proposer model to extract a structured proposal from `prompt`.
    pub fn propose<P: DeserializeOwned>(&self, prompt: &str) -> anyhow::Result<P> {
        self.proposer.extract::<P>(prompt)
    }

    /// Ask the reviewer model to extract a structured review from `prompt`,
    /// with no threshold logic applied — the caller decides what a "review"
    /// type's confidence field means and how to react to it, using
    /// `meets_confidence_threshold` below.
    pub fn review_raw<R: DeserializeOwned>(&self, prompt: &str) -> anyhow::Result<R> {
        self.reviewer.extract::<R>(prompt)
    }

    /// Whether a reported confidence value clears this verifier's configured
    /// threshold.
    pub fn meets_confidence_threshold(&self, confidence: f32) -> bool {
        confidence >= self.config.min_reviewer_confidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestProposal {
        value: String,
        confidence: f32,
    }

    #[test]
    fn mock_client_extracts_arbitrary_struct_type() {
        let json = r#"{"value":"hello","confidence":0.75}"#;
        let mock = MockModelClient::default().with_response(json);
        let result: TestProposal = mock.extract("any prompt").unwrap();
        assert_eq!(result, TestProposal { value: "hello".to_string(), confidence: 0.75 });
    }

    #[test]
    fn config_defaults_match_expected_values() {
        let config = MultiModelConfig::default();
        assert_eq!(config.proposer_model, "claude-sonnet-4-5");
        assert_eq!(config.reviewer_model, "claude-haiku-4-5");
        assert_eq!(config.min_reviewer_confidence, 0.80);
    }

    #[test]
    fn meets_confidence_threshold_checks_correctly() {
        let config = MultiModelConfig::default().with_threshold(0.80);
        let verifier = MultiModelVerifier::new(
            MockModelClient::default(),
            MockModelClient::default(),
            config,
        );
        assert!(verifier.meets_confidence_threshold(0.80));
        assert!(verifier.meets_confidence_threshold(0.95));
        assert!(!verifier.meets_confidence_threshold(0.79));
    }

    #[test]
    fn propose_and_review_raw_use_the_correct_client() {
        let proposer_json = r#"{"value":"proposed","confidence":0.9}"#;
        let reviewer_json = r#"{"value":"reviewed","confidence":0.6}"#;
        let verifier = MultiModelVerifier::new(
            MockModelClient::default().with_response(proposer_json),
            MockModelClient::default().with_response(reviewer_json),
            MultiModelConfig::default(),
        );

        let proposal: TestProposal = verifier.propose("propose prompt").unwrap();
        assert_eq!(proposal.value, "proposed");

        let review: TestProposal = verifier.review_raw("review prompt").unwrap();
        assert_eq!(review.value, "reviewed");
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `pwsh.exe -NoProfile -Command "cd D:\promptjects\ufo-types; cargo test --lib multi_model:: -j 2 -- --nocapture"`
Expected: FAIL to compile — `anyhow` crate not yet added as a dependency (before Step 1 is applied), or module not yet wired into `lib.rs` (after Step 1, before Step 4 below). Apply Step 1 first, then this test run should fail only on the missing `pub mod multi_model;` wiring.

- [ ] **Step 4: Wire into lib.rs**

**Note:** this branch forks from `main` directly (not from Phase 1's still-unmerged branch), so `main`'s current module list is `capability, dare, iso, iso_ir, mbse, satisfies, statechart, stereotype, sysml` — it does NOT include Phase 1's `data_format`/`model_capability` modules. Verify this yourself with `grep -E "^pub mod" src/lib.rs` before editing, in case `main` has moved since this plan was written.

In `src/lib.rs`, add the module declaration alphabetically (after `pub mod mbse;`, before `pub mod satisfies;` — `mbse` < `multi_model` < `satisfies`):

```rust
pub mod multi_model;
```

Add the re-export (same position — after `pub use mbse::{...};`, before `pub use satisfies::{...};`):

```rust
pub use multi_model::{ModelClient, MockModelClient, MultiModelConfig, MultiModelVerifier};
```

- [ ] **Step 5: Run test to verify it passes**

Run: `pwsh.exe -NoProfile -Command "cd D:\promptjects\ufo-types; cargo test --lib multi_model:: -j 2 -- --nocapture"`
Expected: PASS (4 tests)

- [ ] **Step 6: Run the full test suite to confirm no breakage**

Run: `pwsh.exe -NoProfile -Command "cd D:\promptjects\ufo-types; cargo test --lib -j 2"`
Expected: PASS, all existing tests plus the 4 new ones green.

- [ ] **Step 7: Commit**

```bash
cd /mnt/d/promptjects/ufo-types
git add Cargo.toml src/multi_model.rs src/lib.rs
git commit -m "feat(multi_model): add generic ModelClient + MultiModelVerifier"
```

---

### Task 2: Numeric coherence check

**Files:**
- Create: `src/coherence.rs`
- Modify: `src/lib.rs` (add module declaration + re-export)
- Test: inline `#[cfg(test)] mod tests` in `src/coherence.rs`

**Interfaces:**
- Consumes: `crate::satisfies::{Constraint, Satisfies, SatisfiesResult}` (already exported, no changes needed).
- Produces: `pub struct NumericAgreement { pub tolerance: f64 }`, `pub fn validate_numeric_agreement(values: &[f64], tolerance: f64) -> SatisfiesResult`, `impl Satisfies<NumericAgreement> for [f64]` and `for Vec<f64>`. This is independent of Task 1 — no shared code, can be implemented and tested without Task 1 present (though Task 1 will already be on this branch when this task starts).

- [ ] **Step 1: Write the failing test**

Create `src/coherence.rs`:

```rust
//! "Coherence" sanity checks: do multiple values (e.g. two different
//! extraction/inference passes' results) agree with each other closely
//! enough to be trusted. Follows this crate's established `Satisfies<C>`
//! idiom exactly (see `src/sysml.rs`/`src/data_format.rs`).

use crate::satisfies::{Constraint, Satisfies, SatisfiesResult};

/// Constraint: every value in the subject slice is within `tolerance` of
/// every other value (checked via max-minus-min, which is equivalent to
/// pairwise-within-tolerance for a single scalar tolerance).
pub struct NumericAgreement {
    pub tolerance: f64,
}

impl Constraint for NumericAgreement {}

impl Satisfies<NumericAgreement> for [f64] {
    fn satisfies(&self, constraint: &NumericAgreement) -> SatisfiesResult {
        validate_numeric_agreement(self, constraint.tolerance)
    }
}

impl Satisfies<NumericAgreement> for Vec<f64> {
    fn satisfies(&self, constraint: &NumericAgreement) -> SatisfiesResult {
        self.as_slice().satisfies(constraint)
    }
}

/// Checks whether `values` agree within `tolerance` (max - min <= tolerance).
/// Fewer than 2 values can't demonstrate agreement or disagreement between
/// independent sources, so this returns `Unknown` rather than a false
/// `Satisfied`.
pub fn validate_numeric_agreement(values: &[f64], tolerance: f64) -> SatisfiesResult {
    if values.len() < 2 {
        return SatisfiesResult::unknown();
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let spread = max - min;
    if spread <= tolerance {
        SatisfiesResult::satisfied(1.0, Vec::new())
    } else {
        SatisfiesResult::violated(format!(
            "values disagree by {spread} (min {min}, max {max}), exceeding tolerance {tolerance}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_within_tolerance_are_satisfied() {
        let values = vec![100.00, 100.02, 99.99];
        let result = validate_numeric_agreement(&values, 0.05);
        assert!(result.disposition.is_satisfied(), "{:?}", result.disposition);
    }

    #[test]
    fn values_outside_tolerance_are_violated() {
        let values = vec![100.00, 105.00];
        let result = validate_numeric_agreement(&values, 0.05);
        assert!(!result.disposition.is_satisfied());
        assert!(matches!(result.disposition, crate::satisfies::Disposition::Violated { .. }));
    }

    #[test]
    fn single_value_is_unknown_not_satisfied() {
        let values = vec![100.00];
        let result = validate_numeric_agreement(&values, 0.05);
        assert!(matches!(result.disposition, crate::satisfies::Disposition::Unknown));
    }

    #[test]
    fn empty_values_is_unknown() {
        let values: Vec<f64> = vec![];
        let result = validate_numeric_agreement(&values, 0.05);
        assert!(matches!(result.disposition, crate::satisfies::Disposition::Unknown));
    }

    #[test]
    fn satisfies_trait_usable_on_slice_and_vec() {
        let values = vec![50.0, 50.01];
        let constraint = NumericAgreement { tolerance: 0.05 };
        assert!(values.satisfies(&constraint).disposition.is_satisfied());
        assert!(values.as_slice().satisfies(&constraint).disposition.is_satisfied());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pwsh.exe -NoProfile -Command "cd D:\promptjects\ufo-types; cargo test --lib coherence:: -j 2 -- --nocapture"`
Expected: FAIL to compile — module not yet wired into `lib.rs`.

- [ ] **Step 3: Wire into lib.rs**

In `src/lib.rs`, add the module declaration alphabetically (after `pub mod capability;`, before `pub mod dare;` — `capability` < `coherence` < `dare`):

```rust
pub mod coherence;
```

Add the re-export (same position — after the `pub use capability::{...};` block, before `pub use dare::{...};`):

```rust
pub use coherence::{NumericAgreement, validate_numeric_agreement};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pwsh.exe -NoProfile -Command "cd D:\promptjects\ufo-types; cargo test --lib coherence:: -j 2 -- --nocapture"`
Expected: PASS (5 tests)

- [ ] **Step 5: Run the full test suite**

Run: `pwsh.exe -NoProfile -Command "cd D:\promptjects\ufo-types; cargo test --lib -j 2"`
Expected: PASS, every test in the crate green (existing + 4 from Task 1 + 5 from Task 2).

- [ ] **Step 6: Also build with every feature combination**

Run: `pwsh.exe -NoProfile -Command "cd D:\promptjects\ufo-types; cargo build --all-features -j 2; cargo test --all-features -j 2"`
Expected: PASS — confirms nothing in this phase accidentally depends on a feature-gated module.

- [ ] **Step 7: Commit**

```bash
cd /mnt/d/promptjects/ufo-types
git add src/coherence.rs src/lib.rs
git commit -m "feat(coherence): add NumericAgreement Satisfies constraint"
```

---

## Final Verification Checklist (for the validating/committing agent — Tasks 1-2, this repo only)

Run these from `/mnt/d/promptjects/ufo-types` on the feature branch, after both tasks are complete, before opening the PR:

- [ ] `pwsh.exe -NoProfile -Command "cd D:\promptjects\ufo-types; cargo test --lib -j 2"` — all tests pass, including the 9 new ones (4 in Task 1, 5 in Task 2).
- [ ] `pwsh.exe -NoProfile -Command "cd D:\promptjects\ufo-types; cargo test --all-features -j 2"` — passes.
- [ ] `pwsh.exe -NoProfile -Command "cd D:\promptjects\ufo-types; cargo build --all-features -j 2"` — no warnings from `multi_model.rs`/`coherence.rs`.
- [ ] `pwsh.exe -NoProfile -Command "cd D:\promptjects\ufo-types; cargo fmt --check"` — passes (this repo's CI enforces this).
- [ ] Confirm `ModelClient`, `MockModelClient`, `MultiModelConfig`, `MultiModelVerifier`, `NumericAgreement`, `validate_numeric_agreement` are all re-exported from the crate root (`grep "pub use" src/lib.rs`).
- [ ] Confirm `git diff main -- src/lib.rs` shows only additive lines (new `pub mod`/`pub use`), nothing changed or removed.
- [ ] Open the PR against `PromptExecution/ufo-types` `main`, referencing and closing issue #13 (`Closes #13` in the PR body). Mention in the body that `PromptExecution/ledgrrr#218` is the companion consumer repoint (separate repo, separate PR, not part of this one).
- [ ] Do not touch `ledgrrr` in this PR — that's Task 3, a separate repo and a separate PR (see "Task 3" section below, which is NOT part of this repo's plan execution — it's documented here for context but must be executed as its own dispatch against a `ledgrrr` checkout, after this PR's branch tip SHA is known).

---

## Task 3 (separate repo — `ledgrrr`, NOT executed as part of this plan's subagent-driven-development run)

This task lives in a different repository (`ledgrrr`, not `ufo-types`) and depends on this plan's Tasks 1-2 branch tip SHA, which isn't known until they're committed. It is documented here for continuity, but the controller must dispatch it as a **separate plan/dispatch** against a `ledgrrr` checkout, after Tasks 1-2 land on this branch (not necessarily merged — per this epic's established "pin to branch tip" convention).

**Files (in `ledgrrr`):**
- Modify: `crates/ledger-core/Cargo.toml` (bump the `ufo-types` rev pin from `1881d67` to this plan's branch tip)
- Modify: `crates/ledger-core/src/verify.rs` (remove local `ModelClient`/`MockModelClient`/`MultiModelConfig`, re-export the `ufo-types` versions, rewrite `MultiModelVerifier` to wrap `ufo_types::MultiModelVerifier<C>` internally)

**Interfaces:**
- Consumes: `ufo_types::{ModelClient, MockModelClient, MultiModelConfig, MultiModelVerifier}` (this plan's Task 1).
- Produces: no change to `ledger_core::verify`'s own public surface — `RepairProposal`, `ReviewResult`, `VerificationOutcome`, and `MultiModelVerifier::{new, propose_fix, review, verify}` (the domain-specific wrapper API) all keep their exact existing signatures. All 6 existing tests in `verify.rs` must pass byte-for-byte unchanged.

**Sketch of the rewritten `MultiModelVerifier` wrapper** (the controller dispatching this as a separate task should expand this into full brief detail, mirroring this plan's own level of completeness — this is context, not a copy-paste-ready spec):

```rust
pub use ufo_types::{ModelClient, MockModelClient, MultiModelConfig};

pub struct MultiModelVerifier<C: ModelClient> {
    inner: ufo_types::MultiModelVerifier<C>,
}

impl<C: ModelClient> MultiModelVerifier<C> {
    pub fn new(proposer: C, reviewer: C, config: MultiModelConfig) -> Self {
        Self { inner: ufo_types::MultiModelVerifier::new(proposer, reviewer, config) }
    }

    pub fn propose_fix(&self, rule_id: &str, issues_json: &str, context: &str) -> anyhow::Result<RepairProposal> {
        let prompt = format!(
            "Given these validation issues:\n{}\n\nContext: {}\n\nPropose a fix for rule {}. Return JSON: {{\"rule_id\": \"{}\", \"proposed_fix\": \"...\", \"reasoning\": \"...\", \"confidence\": 0.0-1.0}}",
            issues_json, context, rule_id, rule_id
        );
        self.inner.propose::<RepairProposal>(&prompt)
    }

    pub fn review(&self, proposal: &RepairProposal) -> anyhow::Result<ReviewResult> {
        let prompt = format!(
            "Review this proposed fix:\nRule: {}\nFix: {}\nReasoning: {}\nConfidence: {}\n\nReturn JSON: {{\"approved\": bool, \"concerns\": [], \"suggestions\": [], \"confidence\": 0.0-1.0}}",
            proposal.rule_id, proposal.proposed_fix, proposal.reasoning, proposal.confidence
        );
        let result = self.inner.review_raw::<ReviewResult>(&prompt)?;
        if !self.inner.meets_confidence_threshold(result.confidence) {
            return Ok(ReviewResult {
                approved: false,
                concerns: vec![format!(
                    "Reviewer confidence {} below threshold {}",
                    result.confidence, self.inner.config().min_reviewer_confidence
                )],
                ..result
            });
        }
        Ok(result)
    }

    pub fn verify(&self, rule_id: &str, issues_json: &str, context: &str) -> anyhow::Result<VerificationOutcome> {
        let proposal = self.propose_fix(rule_id, issues_json, context)?;
        let review = self.review(&proposal)?;
        let outcome = if review.approved && self.inner.meets_confidence_threshold(review.confidence) {
            VerificationOutcome::Approved { proposal, review }
        } else {
            VerificationOutcome::Rejected { proposal, review }
        };
        Ok(outcome)
    }
}
```

`RepairProposal`, `ReviewResult`, `VerificationOutcome` (and its `is_approved()` method) stay exactly as they already are in `verify.rs` — not shown here, not modified.
