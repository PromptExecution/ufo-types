//! PyO3 binding — P3 milestone of the b00t SysML v2 spine consolidation
//! epic (`elasticdotventures/_b00t_#1177`): cross-runtime validation. This
//! is the epic's answer to "bidirectionality" — the same Rust-owned
//! `sysml::validate_sysml_v2` is exposed to Python so both runtimes can be
//! shown to agree on the identical fixture set, without SysML text ever
//! generating Rust structs (that direction stays rejected; SysML remains
//! validated output only, per the epic's own non-goals).
//!
//! Follows `common-core/critter-keeper`'s established pyo3+maturin pattern
//! (`src/python.rs`, `#[pymodule] fn _core`) rather than inventing a new
//! binding approach.
//!
//! Only `validate_sysml_v2` is bound here, not `MbseExport::to_sysml_v2` —
//! `to_sysml_v2` is a per-type trait method with no concrete implementor in
//! this crate to bind generically (`ufo-types` only defines the trait;
//! downstream crates like `b00t-cli`'s `dispatch_sysml` provide concrete
//! exports). Binding it would mean picking one arbitrary downstream type,
//! which doesn't belong in this crate. `validate_sysml_v2` is the
//! standalone, load-bearing half of P3 — the free function every SysML v2
//! text producer already validates against.

use crate::sysml::validate_sysml_v2 as rust_validate_sysml_v2;
use pyo3::prelude::*;

/// Validate `text` as SysML v2 syntax. Returns `(is_valid, reason)`: `reason`
/// is `None` when valid, and the joined parser diagnostic message(s) when
/// not — mirroring `Disposition::Satisfied`/`Violated` without requiring a
/// `SatisfiesResult` Python binding for this first prototype.
#[pyfunction]
fn validate_sysml_v2(text: &str) -> (bool, Option<String>) {
    let result = rust_validate_sysml_v2(text);
    match result.disposition {
        crate::satisfies::Disposition::Satisfied => (true, None),
        crate::satisfies::Disposition::Violated { reason } => (false, Some(reason)),
        crate::satisfies::Disposition::Unknown => {
            (false, Some("unknown: insufficient evidence".to_string()))
        }
    }
}

#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn _core(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate_sysml_v2, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
