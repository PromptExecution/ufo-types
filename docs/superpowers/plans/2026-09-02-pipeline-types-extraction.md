# Pipeline Types Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fold `b00t-cli`'s `pipeline_types.rs`, `pipeline_flowctl.rs`, and `pipeline_secrets.rs` into this crate (`ufo-types`) as new modules, so a lightweight pipeline-engine consumer can depend on `ufo-types` for `StageSpec`/`CapsuleProfile`/flow-control/secret-injection types instead of the full 34-submodule `_b00t_` monorepo.

**Architecture:** Three modules land at the crate root (`src/pipeline_secrets.rs`, `src/pipeline_types.rs`, `src/pipeline_flowctl.rs`), mirroring the flat top-level module convention `ufo-types` already uses for `satisfies`, `stereotype`, `dare`, etc. `pipeline_types.rs` and `pipeline_flowctl.rs` reference each other (`StageSpec` carries an `Option<StageFlowConfig>`; `auto_strategy` takes a `&StageSpec`) — this is fine within one crate, it only mattered for the earlier idea of a standalone types-only crate that excluded flow control. The one real code change (not just a copy) is `HostResources`'s `Satisfies<ResourceRequirements>` impl: the original targets `b00t_c0re_lib::satisfies::Satisfies` (`fn satisfies(&self, ...) -> anyhow::Result<EvidenceReport>`); this plan ports it to `ufo-types`'s own `crate::satisfies::Satisfies` (`fn satisfies(&self, ...) -> SatisfiesResult`, infallible), since the whole point of landing this code here is to use this crate's own constraint-satisfaction shape instead of depending on `b00t_c0re_lib`.

**Tech Stack:** Rust 2024 edition, serde/serde_json, anyhow, `keyring` (new optional dependency), `rpassword`, `shellexpand`, `tempfile`/`toml` (dev-dependencies).

**Spec:** No formal spec document. This plan implements `elasticdotventures/_b00t_#1251` (comment: https://github.com/elasticdotventures/_b00t_/issues/1251#issuecomment-5510645087), which itself was filed as the outcome of `app4dog`'s `docs/superpowers/specs/2026-09-01-create-a-critter-pipeline-orchestration.md` (workspace#109) needing `StageSpec` without the full `_b00t_` monorepo as a dependency. The user's explicit direction, given after this plan's author found the two `Satisfies<T>` shapes are NOT a re-export of each other (verified by reading both trait definitions directly), was: "Port it to ufo_types::satisfies" — i.e. rewrite the impl against this crate's `SatisfiesResult`/`Disposition` shape rather than keeping the `b00t_c0re_lib` dependency or re-implementing `EvidenceReport` here.

## Global Constraints

- This plan's workspace is a **separate git repository** from the `app4dog` monorepo this plan was authored inside: `github.com/PromptExecution/ufo-types`, cloned to `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types` on branch `feat/1251-pipeline-types-extraction` (already created, forked from `main` at the commit `git clone` fetched). All git/cargo commands in this plan's tasks run against **that** path.
- **Sandbox git constraint:** this session runs inside a worktree-isolated sandbox that refuses any `git` invocation whose target path is computed at runtime (a shell variable, or a `cd` before the git command) — only a *literal* absolute path typed directly into the command is accepted. Every git command in every task step below must spell out the full path `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types` literally (e.g. `git -C /home/.../ufo-types status`) — never `cd` there first, and never build the path from a variable. Plain `cargo`/file-editing commands are not restricted this way, but running them requires actually being in that directory, so prefer `cargo <cmd> --manifest-path /home/.../ufo-types/Cargo.toml` over `cd`.
- Source-of-truth copies of the three original files already exist, fetched verbatim from `elasticdotventures/_b00t_` at their current `main` content, at these literal paths (do not re-fetch — read them directly):
  - `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-b00t-extraction/pipeline_secrets.rs`
  - `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-b00t-extraction/pipeline_types.rs`
  - `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-b00t-extraction/pipeline_flowctl.rs`
  These are temporary scratch files — this plan's tasks read them once each; nothing later in this session depends on them surviving past task completion.
- Every new module's public API (struct/enum names, field names, method names, and signatures) is copied **verbatim** from the source files above, with exactly one deliberate exception: the `Satisfies<ResourceRequirements>` impl block in `pipeline_types.rs`, ported from `b00t_c0re_lib::satisfies` to `crate::satisfies` per Task 2's exact instructions. No other logic changes.
- `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings` must both pass (this is what the repo's own CI runs — see `.github/workflows/ci.yml`), and `cargo test --all-features` must pass, after every task.
- New Cargo dependencies added by this plan (`rpassword`, `shellexpand`, `keyring` (optional), `tempfile`/`toml` (dev)) must use the exact version constraints given in each task — do not "helpfully" bump them.
- The `keyring` feature is a **real, working** optional dependency in this crate, unlike `b00t-cli`'s own same-named `keyring = []` feature (a no-op stub with no crate behind it — confirmed by reading `b00t-cli`'s `Cargo.toml`, which declares the feature but never declares a `keyring` dependency at all, meaning that code path has never actually compiled there). This is a deliberate, called-out correctness improvement over the source, not an accidental scope change — do not "fix" it back to a stub.
- Commit after each task with a message describing what landed; do not squash across tasks until the final whole-branch review (per `subagent-driven-development`).

---

### Task 1: Add `pipeline_secrets` module

**Files:**
- Modify: `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/Cargo.toml`
- Create: `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/src/pipeline_secrets.rs`
- Modify: `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/src/lib.rs`

**Interfaces:**
- Produces: `SecretSource` (enum: `File{path}`, `EnvVar{name}`, `Keyring{service,account}`, `Prompt{description}`, `AzureKeyVault{vault,name}`), `SecretRef{key,env_var,source}`, `SecretStore` (`resolve`, `inject_to_env`, `get`, `len`, `is_empty`), `SecureStageEnv{stage_name,secret_refs,store}` (`new`, `resolve`, `inject_to_env`, `get`), `load_secret(&SecretRef) -> anyhow::Result<String>`, `list_azure_secret_names(vault, prefix) -> anyhow::Result<Vec<String>>`. Task 2's `pipeline_types.rs` consumes `SecretRef` by name (`crate::pipeline_secrets::SecretRef`).

- [ ] **Step 1: Add new dependencies to `Cargo.toml`**

Read the current file first, then apply this exact edit — insert three new dependency lines immediately after the existing `schemars = { version = "0.8", features = ["derive"] }` line (before the `# \`sysml-v2-parser\` is not wasm32-compatible...` comment):

Old:
```toml
schemars = { version = "0.8", features = ["derive"] }
# `sysml-v2-parser` is not wasm32-compatible, so `sysml` is opt-in rather
```

New:
```toml
schemars = { version = "0.8", features = ["derive"] }
rpassword = "7"
shellexpand = "3.1"
# Optional OS keyring backend for `pipeline_secrets::SecretSource::Keyring`
# (elasticdotventures/_b00t_#1251 extraction) — mirrors b00t-cli's existing
# `keyring` feature name/gate, but as a real dependency: b00t-cli's own
# `keyring = []` feature is a no-op stub with no crate behind it, so that
# code path has never actually compiled there. This makes it real.
keyring = { version = "3", optional = true }
# `sysml-v2-parser` is not wasm32-compatible, so `sysml` is opt-in rather
```

Then, in the `[features]` section, apply this edit:

Old:
```toml
[features]
sysml = ["dep:sysml-v2-parser"]
python = ["dep:pyo3", "sysml"]
statechart = ["dep:scxml"]
```

New:
```toml
[features]
sysml = ["dep:sysml-v2-parser"]
python = ["dep:pyo3", "sysml"]
statechart = ["dep:scxml"]
keyring = ["dep:keyring"]
```

Then, in `[dev-dependencies]`, apply this edit:

Old:
```toml
[dev-dependencies]
serde_json = "1.0"
```

New:
```toml
[dev-dependencies]
serde_json = "1.0"
tempfile = "3"
```

- [ ] **Step 2: Create `src/pipeline_secrets.rs`**

Read the file at `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-b00t-extraction/pipeline_secrets.rs` in full (it is 683 lines). Write its exact content to `src/pipeline_secrets.rs`, with exactly one change: in the `#[cfg(test)] mod tests` block, replace this line:

Old:
```rust
    use crate::test_env::ENV_LOCK;
```

New:
```rust
    use std::sync::{LazyLock, Mutex};

    /// Crate-wide test lock for tests that mutate process-wide env vars.
    /// The source file (`b00t-cli`) has a crate-wide `test_env::ENV_LOCK`
    /// used by many modules; this crate only needs it here, so it is
    /// defined locally rather than adding a new crate-wide module for one
    /// caller.
    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
```

No other line in the file changes — the module has no other reference to anything outside itself (verified: it uses only `anyhow`, `serde`, `std::collections::HashMap`, `std::fmt`, plus `keyring`/`rpassword`/`shellexpand` at call sites, all of which Step 1 just added as dependencies).

- [ ] **Step 3: Wire the module into `src/lib.rs`**

Read the current file first. Apply this edit to the `pub mod` block:

Old:
```rust
pub mod model_capability;
pub mod multi_model;
#[cfg(feature = "python")]
mod python;
```

New:
```rust
pub mod model_capability;
pub mod multi_model;
pub mod pipeline_secrets;
#[cfg(feature = "python")]
mod python;
```

Then apply this edit to the re-export block:

Old:
```rust
pub use model_capability::ModelCapability;
pub use multi_model::{MockModelClient, ModelClient, MultiModelConfig, MultiModelVerifier};
pub use satisfies::{
```

New:
```rust
pub use model_capability::ModelCapability;
pub use multi_model::{MockModelClient, ModelClient, MultiModelConfig, MultiModelVerifier};
pub use pipeline_secrets::{
    SecretRef, SecretSource, SecretStore, SecureStageEnv, list_azure_secret_names, load_secret,
};
pub use satisfies::{
```

- [ ] **Step 4: Run the test suite for this module**

Run: `cargo test --manifest-path /home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/Cargo.toml --features keyring pipeline_secrets`

Expected: all `pipeline_secrets::tests::*` tests pass (17 tests: `file_source_reads_contents`, `file_source_trims_whitespace`, `file_source_missing_file_errors`, `envvar_source_reads_environment`, `envvar_source_missing_var_errors`, `inject_to_env_merges_correctly`, `inject_to_env_overwrites_existing_keys`, `debug_does_not_print_secret_values`, `secret_ref_debug_does_not_contain_values`, `empty_refs_resolves_to_empty_store`, `secure_stage_env_resolve_and_inject`, `secure_stage_env_debug_redacts`, `secret_store_get_returns_none_for_missing_key`, `secret_ref_serialize_round_trip`, `azure_secret_list_jmespath_filter_scopes_by_prefix`, `azure_secret_list_jmespath_filter_is_client_side_only`, `secret_source_serialize_round_trip_all_variants` — 17 tests total). `--features keyring` is required for a full build sanity check (the `Keyring` source arm is `#[cfg(feature = "keyring")]`-gated and must compile); also run once WITHOUT `--features keyring` to confirm the crate still compiles with the feature off (the source arm falls back to `anyhow::bail!("keyring secret source requires the 'keyring' feature")`).

- [ ] **Step 5: Format, lint, commit**

Run:
```
cargo fmt --manifest-path /home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/Cargo.toml --all
cargo clippy --manifest-path /home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/Cargo.toml --all-targets --all-features -- -D warnings
```
Fix any clippy warnings before committing (the source file was clippy-clean in its origin repo, so any warning here is either an edition-2024-vs-2021 lint difference or introduced by the `test_env` edit — read the warning and fix the actual code, don't `#[allow]` it away without understanding why).

Then commit, using the literal path (never `cd`):
```
git -C /home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types add Cargo.toml src/pipeline_secrets.rs src/lib.rs
git -C /home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types commit -m "feat: add pipeline_secrets module (elasticdotventures/_b00t_#1251)"
```

---

### Task 2: Add `pipeline_types` and `pipeline_flowctl` modules

**Files:**
- Modify: `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/Cargo.toml`
- Create: `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/src/pipeline_types.rs`
- Create: `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/src/pipeline_flowctl.rs`
- Modify: `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/src/lib.rs`

**Interfaces:**
- Consumes: `crate::pipeline_secrets::SecretRef` (from Task 1 — must be complete and committed first), `crate::satisfies::{Satisfies, SatisfiesResult, Disposition}` (already in the crate, untouched).
- Produces: `StageSpec`, `CapsuleProfile`, `StagePort`, `PortDirection`, `PortMediaType`, `ResourceRequirements`, `HostResources`, `ResourceFit`, `PipelineError`, `ErrorRoute`, `StageEntry`, `NegotiationResult`, `PipelineEdge`, `PipelineDag`, `can_negotiate`, `auto_insert_conversions` (from `pipeline_types`); `FlowStrategy`, `FlowControl`, `FlowGate`, `StageFlowConfig`, `auto_strategy`, `is_gpu_profile`, `is_memory_intensive` (from `pipeline_flowctl`). These two modules reference each other directly (`pipeline_types::StageSpec.flow_control: Option<pipeline_flowctl::StageFlowConfig>`; `pipeline_flowctl::auto_strategy(&StageSpec)`), so both must exist before either compiles — this is why they are one task, not two.

- [ ] **Step 1: Add `toml` dev-dependency to `Cargo.toml`**

Read the current file first (it now reflects Task 1's edit). Apply this edit:

Old:
```toml
[dev-dependencies]
serde_json = "1.0"
tempfile = "3"
```

New:
```toml
[dev-dependencies]
serde_json = "1.0"
tempfile = "3"
toml = "0.8"
```

(`toml` is needed only by one test in `pipeline_types.rs` — `capsule_profile_serialize`, which calls `toml::to_string`.)

- [ ] **Step 2: Create `src/pipeline_flowctl.rs` (verbatim copy, zero edits)**

Read the file at `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-b00t-extraction/pipeline_flowctl.rs` in full (549 lines). Write its exact, unmodified content to `src/pipeline_flowctl.rs`. This file has no `b00t_c0re_lib` reference and no other external dependency beyond what the crate already has (`serde`, `std`) — confirmed by reading it: its only crate-internal reference is `use crate::pipeline_types::{CapsuleProfile, StageSpec};`, which resolves once Step 3 creates that module in the same crate. Do not alter anything in this file.

- [ ] **Step 3: Create `src/pipeline_types.rs` (copy with one ported impl block + 3 updated tests)**

Read the file at `/home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-b00t-extraction/pipeline_types.rs` in full (1826 lines). Write its content to `src/pipeline_types.rs`, applying exactly these two edits — everything else copies verbatim.

**Edit A — the `Satisfies` impl block.** Replace:

Old:
```rust
// ── GH #780: Satisfies<ResourceRequirements> bridge — one concrete audit case ──
// 🤓 Wraps the existing ResourceFit::fits_on check and lifts its bool result
//    into an EvidenceReport, so a scheduling decision can be appended to the
//    JSONL audit trail (`b00t-cli audit trail --path .b00t/audit.jsonl`).

impl b00t_c0re_lib::satisfies::Satisfies<ResourceRequirements> for HostResources {
    fn satisfies(
        &self,
        constraint: &ResourceRequirements,
    ) -> anyhow::Result<b00t_c0re_lib::satisfies::EvidenceReport> {
        let passed = constraint.fits_on(self);
        let detail = if passed {
            format!(
                "host(ram={}GB,vram={}GB,gpu={},cores={}) satisfies requirements(min_ram={}GB,min_vram={}GB,requires_gpu={},cores={:?})",
                self.ram_gb, self.vram_gb, self.gpu_count, self.cpu_cores,
                constraint.min_ram_gb, constraint.min_vram_gb, constraint.requires_gpu, constraint.cpu_cores
            )
        } else {
            format!(
                "host(ram={}GB,vram={}GB,gpu={},cores={}) does NOT satisfy requirements(min_ram={}GB,min_vram={}GB,requires_gpu={},cores={:?})",
                self.ram_gb, self.vram_gb, self.gpu_count, self.cpu_cores,
                constraint.min_ram_gb, constraint.min_vram_gb, constraint.requires_gpu, constraint.cpu_cores
            )
        };
        Ok(b00t_c0re_lib::satisfies::EvidenceReport::new(
            "ResourceRequirements",
            passed,
            detail,
        ))
    }
}
```

New:
```rust
// ── GH #780 (ported): Satisfies<ResourceRequirements> — one concrete audit case ──
// 🤓 Wraps the existing ResourceFit::fits_on check and lifts its bool result
//    into a SatisfiesResult, so a scheduling decision can be represented in
//    this crate's uniform constraint-satisfaction shape (crate::satisfies).
//    Ported from b00t-cli's original b00t_c0re_lib::satisfies::Satisfies
//    (Result<EvidenceReport>, always-present free-text `detail`) to this
//    crate's own crate::satisfies::Satisfies (infallible SatisfiesResult).
//    The satisfied case has no free-text `detail` field in this shape — only
//    Violated carries a `reason` string — so the passing-case diagnostic
//    text from the original impl is intentionally dropped here rather than
//    misused as a fabricated evidence-graph NodeId.

impl crate::satisfies::Satisfies<ResourceRequirements> for HostResources {
    fn satisfies(&self, constraint: &ResourceRequirements) -> crate::satisfies::SatisfiesResult {
        if constraint.fits_on(self) {
            crate::satisfies::SatisfiesResult::satisfied(1.0, Vec::new())
        } else {
            let detail = format!(
                "host(ram={}GB,vram={}GB,gpu={},cores={}) does NOT satisfy requirements(min_ram={}GB,min_vram={}GB,requires_gpu={},cores={:?})",
                self.ram_gb, self.vram_gb, self.gpu_count, self.cpu_cores,
                constraint.min_ram_gb, constraint.min_vram_gb, constraint.requires_gpu, constraint.cpu_cores
            );
            crate::satisfies::SatisfiesResult::violated(detail)
        }
    }
}
```

**Edit B — the three `Satisfies` tests.** Replace:

Old:
```rust
    #[test]
    fn satisfies_resource_requirements_produces_passing_evidence() {
        use b00t_c0re_lib::satisfies::Satisfies;

        let req = ResourceRequirements {
            min_ram_gb: 4.0,
            min_vram_gb: 0.0,
            requires_gpu: false,
            cpu_cores: None,
            scratch_disk_gb: None,
        };
        let host = HostResources {
            ram_gb: 8.0,
            vram_gb: 0.0,
            gpu_count: 0,
            cpu_cores: 4,
        };
        let report = host.satisfies(&req).unwrap();
        assert!(report.passed);
        assert_eq!(report.constraint_type, "ResourceRequirements");
        assert!(report.node_id.starts_with("sat:"));
        assert!(!report.is_violated());
    }

    #[test]
    fn satisfies_resource_requirements_produces_violated_evidence() {
        use b00t_c0re_lib::satisfies::Satisfies;

        let req = ResourceRequirements {
            min_ram_gb: 64.0,
            min_vram_gb: 0.0,
            requires_gpu: false,
            cpu_cores: None,
            scratch_disk_gb: None,
        };
        let host = HostResources {
            ram_gb: 8.0,
            vram_gb: 0.0,
            gpu_count: 0,
            cpu_cores: 4,
        };
        let report = host.satisfies(&req).unwrap();
        assert!(!report.passed);
        assert!(report.is_violated());
        assert!(report.detail.contains("does NOT satisfy"));
    }

    #[test]
    fn satisfies_resource_requirements_agrees_with_fits_on() {
        use b00t_c0re_lib::satisfies::Satisfies;

        let req = ResourceRequirements {
            min_ram_gb: 1.0,
            min_vram_gb: 8.0,
            requires_gpu: true,
            cpu_cores: None,
            scratch_disk_gb: None,
        };
        let host = HostResources {
            ram_gb: 16.0,
            vram_gb: 16.0,
            gpu_count: 1,
            cpu_cores: 8,
        };
        assert_eq!(req.fits_on(&host), host.satisfies(&req).unwrap().passed);
    }
```

New:
```rust
    #[test]
    fn satisfies_resource_requirements_produces_passing_evidence() {
        use crate::satisfies::Satisfies;

        let req = ResourceRequirements {
            min_ram_gb: 4.0,
            min_vram_gb: 0.0,
            requires_gpu: false,
            cpu_cores: None,
            scratch_disk_gb: None,
        };
        let host = HostResources {
            ram_gb: 8.0,
            vram_gb: 0.0,
            gpu_count: 0,
            cpu_cores: 4,
        };
        let report = host.satisfies(&req);
        assert!(report.is_satisfied());
        assert!(!report.is_violated());
    }

    #[test]
    fn satisfies_resource_requirements_produces_violated_evidence() {
        use crate::satisfies::{Disposition, Satisfies};

        let req = ResourceRequirements {
            min_ram_gb: 64.0,
            min_vram_gb: 0.0,
            requires_gpu: false,
            cpu_cores: None,
            scratch_disk_gb: None,
        };
        let host = HostResources {
            ram_gb: 8.0,
            vram_gb: 0.0,
            gpu_count: 0,
            cpu_cores: 4,
        };
        let report = host.satisfies(&req);
        assert!(!report.is_satisfied());
        assert!(report.is_violated());
        match &report.disposition {
            Disposition::Violated { reason } => {
                assert!(reason.contains("does NOT satisfy"));
            }
            other => panic!("expected Violated, got {other:?}"),
        }
    }

    #[test]
    fn satisfies_resource_requirements_agrees_with_fits_on() {
        use crate::satisfies::Satisfies;

        let req = ResourceRequirements {
            min_ram_gb: 1.0,
            min_vram_gb: 8.0,
            requires_gpu: true,
            cpu_cores: None,
            scratch_disk_gb: None,
        };
        let host = HostResources {
            ram_gb: 16.0,
            vram_gb: 16.0,
            gpu_count: 1,
            cpu_cores: 8,
        };
        assert_eq!(req.fits_on(&host), host.satisfies(&req).is_satisfied());
    }
```

Nothing else in the file changes — every other struct, enum, impl, and test (all `#719`/`#720`/`#721`/`#722`/`#723`/`#724` sections and their tests) copies verbatim.

- [ ] **Step 4: Wire both modules into `src/lib.rs`**

Read the current file first (it now reflects Task 1's edits). Apply this edit to the `pub mod` block:

Old:
```rust
pub mod model_capability;
pub mod multi_model;
pub mod pipeline_secrets;
#[cfg(feature = "python")]
mod python;
```

New:
```rust
pub mod model_capability;
pub mod multi_model;
pub mod pipeline_flowctl;
pub mod pipeline_secrets;
pub mod pipeline_types;
#[cfg(feature = "python")]
mod python;
```

Then apply this edit to the re-export block:

Old:
```rust
pub use model_capability::ModelCapability;
pub use multi_model::{MockModelClient, ModelClient, MultiModelConfig, MultiModelVerifier};
pub use pipeline_secrets::{
    SecretRef, SecretSource, SecretStore, SecureStageEnv, list_azure_secret_names, load_secret,
};
pub use satisfies::{
```

New:
```rust
pub use model_capability::ModelCapability;
pub use multi_model::{MockModelClient, ModelClient, MultiModelConfig, MultiModelVerifier};
pub use pipeline_flowctl::{
    FlowControl, FlowGate, FlowStrategy, StageFlowConfig, auto_strategy, is_gpu_profile,
    is_memory_intensive,
};
pub use pipeline_secrets::{
    SecretRef, SecretSource, SecretStore, SecureStageEnv, list_azure_secret_names, load_secret,
};
pub use pipeline_types::{
    CapsuleProfile, ErrorRoute, HostResources, NegotiationResult, PipelineDag, PipelineEdge,
    PipelineError, PortDirection, PortMediaType, ResourceFit, ResourceRequirements, StageEntry,
    StagePort, StageSpec, auto_insert_conversions, can_negotiate,
};
pub use satisfies::{
```

- [ ] **Step 5: Add a module-doc bullet documenting the new modules**

Read the current file first. In the crate-level doc comment near the top, apply this edit — insert a new bullet between the "ISO standard wrappers" bullet and the "Any b00t-ecosystem project..." paragraph:

Old:
```rust
//! - **ISO standard wrappers** (`iso`): `Lei` (ISO 17442), `Isin` (ISO
//!   6166), `Currency` (ISO 4217 + common crypto tickers), `BankAccount`
//!   (IBAN/BIC/LEI bundle), `FinancialInstrument` (IFRS 9). These ARE
//!   domain-specific — they encode financial/legal-entity accounting
//!   standards and are only meaningful to consumers working in that space
//!   (e.g. Tax-Lawyer, `ledgrrr`). Not intended as a generic building block
//!   for unrelated domains.
//!
//! Any b00t-ecosystem project needing UFO-grounded domain types and the
```

New:
```rust
//! - **ISO standard wrappers** (`iso`): `Lei` (ISO 17442), `Isin` (ISO
//!   6166), `Currency` (ISO 4217 + common crypto tickers), `BankAccount`
//!   (IBAN/BIC/LEI bundle), `FinancialInstrument` (IFRS 9). These ARE
//!   domain-specific — they encode financial/legal-entity accounting
//!   standards and are only meaningful to consumers working in that space
//!   (e.g. Tax-Lawyer, `ledgrrr`). Not intended as a generic building block
//!   for unrelated domains.
//! - **Pipeline orchestration types** (`pipeline_types`, `pipeline_flowctl`,
//!   `pipeline_secrets`): `StageSpec`/`CapsuleProfile`/`StagePort` (a
//!   pipeline stage's shape, resources, and ports), `PipelineDag` (stage
//!   wiring, topological ordering, cycle detection), `FlowStrategy`/
//!   `FlowControl`/`FlowGate` (back-pressure between stages), and
//!   `SecretRef`/`SecretStore` (secret injection into stage environments).
//!   Extracted from `b00t-cli`'s `pipeline_*.rs` modules
//!   (`elasticdotventures/_b00t_#1251`) so a lightweight pipeline-engine
//!   consumer doesn't need the full 34-submodule `_b00t_` monorepo as a
//!   dependency. `HostResources`'s `Satisfies<ResourceRequirements>` impl
//!   uses this crate's own `satisfies` module directly (ported from
//!   `b00t-cli`'s separate `b00t_c0re_lib::satisfies` shape). Domain-generic.
//!
//! Any b00t-ecosystem project needing UFO-grounded domain types and the
```

- [ ] **Step 6: Run the full test suite**

Run: `cargo test --manifest-path /home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/Cargo.toml --all-features`

Expected: every existing test in the crate still passes, plus all `pipeline_types::tests::*` (55 tests — `port_media_type_mime`, `stage_port_compatible_direct`, `stage_port_incompatible_same_direction`, `stage_port_bytes_compatible_with_any`, `stage_port_incompatible_type_mismatch`, `port_serialize_round_trip`, `satisfies_resource_requirements_produces_passing_evidence`, `satisfies_resource_requirements_produces_violated_evidence`, `satisfies_resource_requirements_agrees_with_fits_on`, `resource_fits_basic`, `resource_fails_ram`, `resource_fails_gpu`, `resource_fits_gpu`, `resource_fails_cpu_cores`, `capsule_profile_serialize`, `input_validation_holds_string`, `resource_exhausted_holds_needed_and_available`, `from_anyhow_error`, `from_string`, `variant_name_all`, `glob_exact`, `glob_prefix`, `glob_catch_all`, `glob_empty`, `route_exact_match`, `route_glob_match`, `route_catch_all`, `retry_within_limit`, `retry_exhausted`, `retry_zero_max`, `serialize_round_trip`, `serialize_skips_retry_count`, `deserialize_defaults_retry_count_to_zero`, `stage_port_enum`, `stage_spec_from_name`, `stage_entry_name_resolves`, `stage_entry_spec_resolves`, `stage_spec_serialize_round_trip`, `port_media_type_all`, `negotiate_direct_match_same_type`, `negotiate_direct_match_bytes_wildcard`, `negotiate_convertible_lossy_false`, `negotiate_convertible_lossy_true`, `negotiate_incompatible_undefined_conversion`, `negotiate_incompatible_same_direction`, `negotiate_result_serialize_round_trip`, `auto_insert_creates_conversion_stage`, `dag_linear_pipeline`, `dag_fan_out`, `dag_fan_in`, `dag_cycle_detected`, `dag_empty_pipeline`, `dag_duplicate_names`, `dag_disconnected_stage`, `auto_insert_direct_match_no_change`) and all `pipeline_flowctl::tests::*` (14 tests — `unbounded_always_allows_emit_and_accept`, `buffered_blocks_when_full`, `buffered_can_accept_false_when_empty`, `throttled_rate_limits`, `throttled_zero_max_blocks_always`, `windowed_tracks_in_flight`, `gpu_stage_gets_buffered_strategy`, `cpu_stage_gets_unbounded_strategy`, `high_memory_stage_gets_buffered_strategy`, `flow_gate_shared_state`, `wait_backpressure_returns_duration`, `vram_stage_gets_buffered_strategy`, `stage_flow_config_creation`, `flow_control_initial_state`) pass.

- [ ] **Step 7: Format, lint, package check, commit**

Run:
```
cargo fmt --manifest-path /home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/Cargo.toml --all
cargo clippy --manifest-path /home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/Cargo.toml --all-targets --all-features -- -D warnings
cargo package --manifest-path /home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types/Cargo.toml --allow-dirty
```
(`cargo package` matches the repo's own CI "Verify distributable package" step — `--allow-dirty` is needed only because this task's own uncommitted changes are still in the working tree at this point; the actual commit happens next.) Fix any warnings before committing.

Then commit, using the literal path (never `cd`):
```
git -C /home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types add Cargo.toml src/pipeline_types.rs src/pipeline_flowctl.rs src/lib.rs
git -C /home/brianh/promptexecution/app4dog/common-core/.claude/worktrees/mesh-verification-gate-completion/scratch-ufo-types/ufo-types commit -m "feat: add pipeline_types and pipeline_flowctl modules (elasticdotventures/_b00t_#1251)

Ports HostResources's Satisfies<ResourceRequirements> impl from
b00t_c0re_lib::satisfies to this crate's own crate::satisfies shape."
```

---

## Follow-up (explicitly out of scope for this plan)

- **b00t-cli integration**: `b00t-cli` itself still has its own local `pipeline_types.rs`/`pipeline_flowctl.rs`/`pipeline_secrets.rs`, and ~15 other `pipeline_*.rs` modules (`pipeline_executor`, `pipeline_nats`, `pipeline_checkpoint`, etc.) that import from them via `crate::pipeline_types::...`. Updating b00t-cli to depend on `ufo-types` for these types instead, and deleting its local duplicates, is a separate, larger effort across the 34-submodule `_b00t_` monorepo — not attempted here per explicit user scoping decision ("New crate only (Recommended)").
- **`canonical-types` rename**: the user noted this crate will "ultimately be renamed to canonical-types" — not actioned by this plan; it's a future rename, not a blocker for this extraction.
