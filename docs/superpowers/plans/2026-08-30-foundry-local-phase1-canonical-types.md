# Foundry Local Epic — Phase 1: Canonical DataFormat + ModelCapability Types

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a canonical `DataFormat` enum, a `JsonWellFormed` sanity-check constraint, and a minimal `ModelCapability` descriptor to `ufo-types` — the shared vocabulary later phases (b00t-server's model registry, ledgrrr's document-intelligence pipeline) use to describe what data a model serves.

**Architecture:** Two new modules following this crate's existing `Satisfies<C>` idiom exactly (see `src/sysml.rs` for the template this plan mirrors): `src/data_format.rs` holds the format vocabulary and its one real sanity check (JSON well-formedness); `src/model_capability.rs` holds the model-card-seed struct that references `DataFormat`. No registry, no concurrency, no Foundry Local wiring — those are separate, later phases in other repos.

**Tech Stack:** Rust 2024 edition, `serde`/`serde_json` (already unconditional deps), `schemars` (already unconditional dep, used for `JsonSchema` derives elsewhere in this crate).

**Spec:** https://github.com/PromptExecution/ufo-types/issues/11 (tracking issue; this epic is an extension of `elasticdotventures/_b00t_#1177`)

## Global Constraints

- No new Cargo feature and no new dependency. `serde_json` is already an unconditional dependency (see `Cargo.toml`) — `DataFormat`, `JsonWellFormed`, and `ModelCapability` land unconditionally in the crate, not behind a feature flag. This crate only feature-gates code that pulls in a *new* dependency (`sysml`, `statechart`, `python` all do; this phase doesn't).
- Follow `src/sysml.rs`'s exact `Satisfies<C>` pattern: a zero-sized constraint marker struct implementing `Constraint`, a `Satisfies<Marker>` impl for `str` (and `String`, delegating to `str`), and a plain free function doing the real work that the trait impl calls into.
- `SatisfiesResult` is never a bare bool — use `SatisfiesResult::satisfied(confidence, evidence_nodes)` / `SatisfiesResult::violated(reason)` exactly as `src/satisfies.rs` defines them. Do not construct `SatisfiesResult` via its struct literal directly (it has private-by-convention helper constructors for a reason — every other module in this crate uses them).
- Derive `Debug, Clone, PartialEq, Eq, Serialize, Deserialize` on every new public type. Add `JsonSchema` (from `schemars`) too — every other public data type in this crate (`Disposition`, `SatisfiesResult`, `NodeId`) does, and downstream consumers (MCP tool schemas) rely on it.
- Use `#[serde(rename_all = "snake_case", tag = "type")]` on the `DataFormat` enum, matching `Disposition`'s wire format in `src/satisfies.rs:62`.
- Build and test entirely with plain `cargo build` / `cargo test` — this crate has no Windows-only dependencies (unlike `ledgrrr`'s desktop-host crates), so no cross-compile or Windows VM step is needed anywhere in this plan. **Before starting Task 1, confirm `cargo check` succeeds in the environment you're running in** (`cd /home/brianh/promptexecution/ufo-types && cargo check --lib`) — a prior check in this WSL environment found no C linker installed at all (`error: linker 'cc' not found`, failing even on vanilla proc-macro build scripts unrelated to this plan's own code). If that reproduces, stop and report it rather than proceeding without test verification; do not attempt to install system packages yourself.
- `PlainText` (open-form prose/paragraph text) gets no sanity-check `Satisfies` impl in this phase, by design — this is documented, not a gap. Do not add one.
- Do not enumerate every conceivable file format. The `DataFormat` enum in this phase covers exactly: `Json`, `PlainText`, `Csv`, `PdfExtractedText`, `Image`, and `Other(String)` as an escape hatch. Do not add more variants.

---

### Task 1: `DataFormat` enum

**Files:**
- Create: `src/data_format.rs`
- Modify: `src/lib.rs` (add module declaration + re-export)
- Test: inline `#[cfg(test)] mod tests` in `src/data_format.rs`

**Interfaces:**
- Consumes: nothing new (only `serde::{Deserialize, Serialize}`, `schemars::JsonSchema`, both already crate dependencies).
- Produces: `pub enum DataFormat { Json, PlainText, Csv, PdfExtractedText, Image, Other(String) }`, later consumed by Task 2 (`ModelCapability`) and Task 3's `JsonWellFormed` module (same file).

- [ ] **Step 1: Write the failing test**

Create `src/data_format.rs` with just the test module (no enum yet):

```rust
//! Canonical data-format/modality vocabulary shared across every consumer
//! that registers or requests a specific kind of model-served data — the
//! b00t-server model registry (Foundry Local epic Phase 2, in
//! `_b00t_`'s `b00t-mcp`) and ledgrrr's document-intelligence pipeline
//! (Phase 3+) both describe "what a model serves" in terms of this enum.
//!
//! Deliberately small and non-exhaustive: `Other(String)` is the escape
//! hatch for anything not yet enumerated, rather than growing this list
//! ahead of a real second consumer needing the specific variant.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_format_roundtrips_through_json() {
        let formats = vec![
            DataFormat::Json,
            DataFormat::PlainText,
            DataFormat::Csv,
            DataFormat::PdfExtractedText,
            DataFormat::Image,
            DataFormat::Other { format: "application/x-custom".to_string() },
        ];
        for format in formats {
            let json = serde_json::to_string(&format).unwrap();
            let back: DataFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(format, back);
        }
    }

    #[test]
    fn data_format_wire_shape_is_tagged() {
        let json = serde_json::to_string(&DataFormat::Json).unwrap();
        assert_eq!(json, r#"{"type":"json"}"#);
        let json = serde_json::to_string(&DataFormat::Other { format: "foo".to_string() }).unwrap();
        assert_eq!(json, r#"{"type":"other","format":"foo"}"#);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/brianh/promptexecution/ufo-types && cargo test --lib data_format:: -- --nocapture`
Expected: FAIL to compile — `DataFormat` not found.

- [ ] **Step 3: Write minimal implementation**

Add above the test module in `src/data_format.rs`:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A canonical data format or modality a model can consume or produce.
///
/// Deliberately small — grows only when a second real consumer needs a
/// variant not yet listed, not speculatively.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum DataFormat {
    /// Structured JSON text.
    Json,
    /// Open-form prose/paragraph text. Intentionally has no `Satisfies`
    /// sanity check — well-formedness of free text isn't a syntactic
    /// property the way JSON's is.
    PlainText,
    /// Comma-separated tabular text.
    Csv,
    /// Text already extracted from a PDF (not the PDF binary itself).
    PdfExtractedText,
    /// Raster or vector image data.
    Image,
    /// Anything not yet enumerated above. Carries a free-form label (e.g.
    /// a MIME type) so callers aren't blocked on this enum growing.
    ///
    /// A named-field struct variant, not a tuple variant: this enum uses
    /// serde's internally tagged representation (`tag = "type"`), which
    /// only supports newtype variants whose inner value serializes as a
    /// map — a bare `String` newtype variant would fail to serialize at
    /// runtime. `Other { format: String }` sidesteps that (matches
    /// `Disposition::Violated { reason: String }`'s exact shape in
    /// `src/satisfies.rs`).
    Other { format: String },
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /home/brianh/promptexecution/ufo-types && cargo test --lib data_format:: -- --nocapture`
Expected: PASS (2 tests: `data_format_roundtrips_through_json`, `data_format_wire_shape_is_tagged`)

- [ ] **Step 5: Wire into lib.rs**

In `src/lib.rs`, add the module declaration alphabetically among the existing `pub mod` lines (after `pub mod dare;`, before `pub mod iso;` — matches existing alphabetical ordering):

```rust
pub mod data_format;
```

And add to the re-export block (after the `pub use dare::{...};` block, before `pub use iso::{...};`, keeping the existing alphabetical-by-module ordering):

```rust
pub use data_format::DataFormat;
```

- [ ] **Step 6: Run the full test suite to confirm no breakage**

Run: `cd /home/brianh/promptexecution/ufo-types && cargo test --lib`
Expected: PASS, all existing tests plus the 2 new ones green.

- [ ] **Step 7: Commit**

```bash
cd /home/brianh/promptexecution/ufo-types
git add src/data_format.rs src/lib.rs
git commit -m "feat(data_format): add canonical DataFormat enum"
```

---

### Task 2: `JsonWellFormed` sanity check

**Files:**
- Modify: `src/data_format.rs` (add constraint + `Satisfies` impls + helper function, below the `DataFormat` enum, above the existing test module)
- Test: add to the existing `#[cfg(test)] mod tests` in `src/data_format.rs`

**Interfaces:**
- Consumes: `crate::satisfies::{Constraint, Satisfies, SatisfiesResult}` (all already exported from `src/satisfies.rs`, no changes needed there).
- Produces: `pub struct JsonWellFormed;`, `pub fn validate_json_well_formed(text: &str) -> SatisfiesResult`, `impl Satisfies<JsonWellFormed> for str` and `for String`. Consumed by later phases (b00t-server, ledgrrr) as the first proof the `Satisfies`-per-`DataFormat` pattern generalizes beyond SysML v2.

- [ ] **Step 1: Write the failing test**

Add to `src/data_format.rs`'s test module (inside the existing `mod tests` block, after the two Task 1 tests):

```rust
    #[test]
    fn well_formed_json_satisfies_constraint() {
        let result = validate_json_well_formed(r#"{"a": 1, "b": [true, null]}"#);
        assert!(result.disposition.is_satisfied(), "{:?}", result.disposition);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn malformed_json_violates_constraint() {
        let result = validate_json_well_formed(r#"{"a": 1,"#);
        assert!(!result.disposition.is_satisfied());
        assert!(matches!(result.disposition, crate::satisfies::Disposition::Violated { .. }));
    }

    #[test]
    fn satisfies_trait_usable_on_str_and_string() {
        let owned = String::from(r#"{"ok": true}"#);
        assert!(owned.satisfies(&JsonWellFormed).disposition.is_satisfied());
        assert!(
            owned
                .as_str()
                .satisfies(&JsonWellFormed)
                .disposition
                .is_satisfied()
        );
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/brianh/promptexecution/ufo-types && cargo test --lib data_format:: -- --nocapture`
Expected: FAIL to compile — `JsonWellFormed`/`validate_json_well_formed` not found.

- [ ] **Step 3: Write minimal implementation**

Add to `src/data_format.rs`, below the `DataFormat` enum and above the `#[cfg(test)]` module:

```rust
use crate::satisfies::{Constraint, Satisfies, SatisfiesResult};

/// Constraint: the subject text is well-formed JSON.
///
/// Follows the same `Satisfies<C>` shape as `crate::sysml::SysmlV2Syntax`
/// (see `src/sysml.rs`) — this is the first proof that pattern generalizes
/// to a non-SysML format.
pub struct JsonWellFormed;

impl Constraint for JsonWellFormed {}

impl Satisfies<JsonWellFormed> for str {
    fn satisfies(&self, _constraint: &JsonWellFormed) -> SatisfiesResult {
        validate_json_well_formed(self)
    }
}

impl Satisfies<JsonWellFormed> for String {
    fn satisfies(&self, constraint: &JsonWellFormed) -> SatisfiesResult {
        self.as_str().satisfies(constraint)
    }
}

/// Parse `text` as JSON and report the result as a [`SatisfiesResult`]:
/// `Satisfied` if it parses, `Violated` with `serde_json`'s error message
/// otherwise.
pub fn validate_json_well_formed(text: &str) -> SatisfiesResult {
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(_) => SatisfiesResult::satisfied(1.0, Vec::new()),
        Err(e) => SatisfiesResult::violated(e.to_string()),
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /home/brianh/promptexecution/ufo-types && cargo test --lib data_format:: -- --nocapture`
Expected: PASS (5 tests total in this module now)

- [ ] **Step 5: Wire re-exports into lib.rs**

In `src/lib.rs`, change the Task 1 re-export line to also export the new items:

```rust
pub use data_format::{DataFormat, JsonWellFormed, validate_json_well_formed};
```

- [ ] **Step 6: Run the full test suite**

Run: `cd /home/brianh/promptexecution/ufo-types && cargo test --lib`
Expected: PASS, all tests green.

- [ ] **Step 7: Commit**

```bash
cd /home/brianh/promptexecution/ufo-types
git add src/data_format.rs src/lib.rs
git commit -m "feat(data_format): add JsonWellFormed Satisfies sanity check"
```

---

### Task 3: `ModelCapability` descriptor

**Files:**
- Create: `src/model_capability.rs`
- Modify: `src/lib.rs` (add module declaration + re-export)
- Test: inline `#[cfg(test)] mod tests` in `src/model_capability.rs`

**Interfaces:**
- Consumes: `crate::data_format::DataFormat` (from Task 1).
- Produces: `pub struct ModelCapability { pub model_name: String, pub formats: Vec<DataFormat>, pub metadata: std::collections::HashMap<String, String> }`, `impl ModelCapability { pub fn new(model_name: impl Into<String>, formats: Vec<DataFormat>) -> Self; pub fn serves(&self, format: &DataFormat) -> bool; pub fn with_metadata(self, key: impl Into<String>, value: impl Into<String>) -> Self }`. Consumed by later phases: `_b00t_`'s `SoulConfig`/`LocalBackend` (Phase 2, registers one `ModelCapability` per model) and ledgrrr's document-intelligence pipeline (Phase 3+, matches a document's required `DataFormat` against registered capabilities).

- [ ] **Step 1: Write the failing test**

Create `src/model_capability.rs`:

```rust
//! `ModelCapability` — a minimal "model card" seed describing what data
//! format(s) one model serves. No registry logic lives here (that's the
//! Foundry Local epic's Phase 2, in `_b00t_`'s `b00t-mcp/src/server_llm.rs`)
//! — this is only the shared type both the registry and its consumers
//! (ledgrrr's document-intelligence pipeline) describe capabilities with.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_format::DataFormat;

    #[test]
    fn new_constructs_with_given_formats() {
        let cap = ModelCapability::new("phi-4-mini", vec![DataFormat::Json, DataFormat::PlainText]);
        assert_eq!(cap.model_name, "phi-4-mini");
        assert_eq!(cap.formats, vec![DataFormat::Json, DataFormat::PlainText]);
        assert!(cap.metadata.is_empty());
    }

    #[test]
    fn serves_checks_membership() {
        let cap = ModelCapability::new("phi-4-mini", vec![DataFormat::Json]);
        assert!(cap.serves(&DataFormat::Json));
        assert!(!cap.serves(&DataFormat::Image));
    }

    #[test]
    fn with_metadata_accumulates_entries() {
        let cap = ModelCapability::new("phi-4-mini", vec![DataFormat::Json])
            .with_metadata("quantization", "int4")
            .with_metadata("hardware", "npu");
        assert_eq!(cap.metadata.get("quantization"), Some(&"int4".to_string()));
        assert_eq!(cap.metadata.get("hardware"), Some(&"npu".to_string()));
    }

    #[test]
    fn model_capability_roundtrips_through_json() {
        let cap = ModelCapability::new("phi-4-mini", vec![DataFormat::Json, DataFormat::Csv])
            .with_metadata("quantization", "int4");
        let json = serde_json::to_string(&cap).unwrap();
        let back: ModelCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, back);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/brianh/promptexecution/ufo-types && cargo test --lib model_capability:: -- --nocapture`
Expected: FAIL to compile — `ModelCapability` not found.

- [ ] **Step 3: Write minimal implementation**

Add above the test module in `src/model_capability.rs`:

```rust
use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::data_format::DataFormat;

/// What one model serves: which [`DataFormat`]s it accepts/produces, plus
/// free-form model-card metadata (quantization, hardware requirements,
/// etc.) that doesn't need a fixed schema yet — this is the type
/// establishing the concept, not the full registry (Phase 2's job).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModelCapability {
    pub model_name: String,
    pub formats: Vec<DataFormat>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl ModelCapability {
    /// Create a capability descriptor with no metadata.
    pub fn new(model_name: impl Into<String>, formats: Vec<DataFormat>) -> Self {
        Self {
            model_name: model_name.into(),
            formats,
            metadata: HashMap::new(),
        }
    }

    /// True if this model claims to serve the given format.
    pub fn serves(&self, format: &DataFormat) -> bool {
        self.formats.contains(format)
    }

    /// Attach a metadata entry, builder-style.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /home/brianh/promptexecution/ufo-types && cargo test --lib model_capability:: -- --nocapture`
Expected: PASS (4 tests)

- [ ] **Step 5: Wire into lib.rs**

In `src/lib.rs`, add the module declaration alphabetically (after `pub mod mbse;`, before `#[cfg(feature = "python")] mod python;`):

```rust
pub mod model_capability;
```

And add the re-export (after the `pub use mbse::{...};` line, before `pub use satisfies::{...};`):

```rust
pub use model_capability::ModelCapability;
```

- [ ] **Step 6: Run the full test suite**

Run: `cd /home/brianh/promptexecution/ufo-types && cargo test --lib`
Expected: PASS, every test in the crate green (existing tests + all 9 new ones across `data_format.rs` and `model_capability.rs`).

- [ ] **Step 7: Also build with every feature combination, matching this crate's CI**

Run: `cd /home/brianh/promptexecution/ufo-types && cargo build --all-features && cargo test --all-features`
Expected: PASS — confirms nothing in this phase accidentally depends on a feature-gated module.

- [ ] **Step 8: Commit**

```bash
cd /home/brianh/promptexecution/ufo-types
git add src/model_capability.rs src/lib.rs
git commit -m "feat(model_capability): add ModelCapability descriptor"
```

---

## Final Verification Checklist (for the validating/committing agent)

Run these from `/home/brianh/promptexecution/ufo-types` on the feature branch, after all 3 tasks are complete, before opening the PR:

- [ ] `cargo test --lib` — all tests pass, including the 9 new ones (2 in Task 1, 3 in Task 2, 4 in Task 3).
- [ ] `cargo test --all-features` — passes (proves no accidental feature-gate leakage).
- [ ] `cargo build --all-features` — no warnings introduced by the new code (`cargo build --all-features 2>&1 | grep -i "data_format\|model_capability"` should be empty).
- [ ] `cargo fmt --check` — passes (this repo's CI runs `cargo fmt --all`, per PR #7's history fixing a prior drift).
- [ ] Confirm `DataFormat`, `JsonWellFormed`, `validate_json_well_formed`, and `ModelCapability` are all re-exported from the crate root (`grep "pub use" src/lib.rs`) — later phases in other repos will import them as `ufo_types::DataFormat` etc., not via the submodule path.
- [ ] Confirm no existing test changed behavior — `git diff main -- src/lib.rs` should show only additive lines (new `pub mod` / `pub use`), no line changed or removed.
- [ ] Open the PR against `PromptExecution/ufo-types` `main`, referencing and closing issue #11 (`Closes #11` in the PR body).
- [ ] Do not touch `ledgrrr` or `_b00t_` in this PR — those pins/consumers are later phases.
