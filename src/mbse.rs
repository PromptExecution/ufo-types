//! MBSE (Model-Based Systems Engineering) export — turn a `Stereotyped`
//! domain value into a SysML v2 `part` usage, so DARE decisions, risks, and
//! proposals double as systems-engineering model artifacts (self-documenting
//! requirements/regulatory evidence), not just internal Rust values.
//!
//! Output is validated as SysML v2 syntax by the `sysml` module's test
//! suite whenever the `sysml` feature is enabled — the same
//! `sysml-v2-parser` grammar that validates `holon-viz`'s emitter output.
//! This module has no `sysml-v2-parser` dependency itself (it only builds
//! `String`s), so it compiles unconditionally.
//!
//! # For agents extending this crate
//!
//! Implementing `MbseExport` is one line, placed directly beside the
//! type's `impl Stereotyped` block:
//!
//! ```ignore
//! impl Stereotyped for MyType {
//!     fn ufo_stereotype(&self) -> UfoStereotype { UfoStereotype::Kind("MyType".into()) }
//! }
//!
//! impl MbseExport for MyType {
//!     fn to_sysml_v2(&self) -> String {
//!         mbse_field_dump(&self.ufo_stereotype(), "MyType", self)
//!     }
//! }
//! ```
//!
//! `mbse_field_dump` reflects the type's own `Serialize` impl — field
//! renames or additions in the struct show up in the export automatically,
//! with no second place to update.
//!
//! Only reach past that one-liner when the type *composes* other
//! `Stereotyped` values (a proposal containing decisions, risks,
//! alternatives) and should emit real SysML v2 part-containment instead of
//! a flat field dump — see `DaredProposal::to_sysml_v2` in `dare.rs` for
//! that pattern: it nests each child's own `to_sysml_v2()` output rather
//! than re-deriving it.

use serde::Serialize;
use serde_json::Value;

use crate::stereotype::{Stereotyped, UfoStereotype};

/// Export as a SysML v2 `part` usage. Required for any `Stereotyped` type
/// that should also stand as systems-engineering model evidence.
pub trait MbseExport: Stereotyped {
    fn to_sysml_v2(&self) -> String;
}

/// Generic single-level field dump: a stereotype comment, then one
/// `attribute` line per top-level field, reflected from `value`'s own
/// `Serialize` impl. `part_name` becomes the SysML `part` identifier — pass
/// something human-recognizable (the type name for a singleton field, or an
/// instance-specific name like `self.name` when many instances of the same
/// type can appear as siblings) and it will be sanitized into a legal
/// identifier for you.
pub fn mbse_field_dump<T: Serialize>(
    stereotype: &UfoStereotype,
    part_name: &str,
    value: &T,
) -> String {
    let json = serde_json::to_value(value).unwrap_or(Value::Null);
    let mut out = format!("// {stereotype}\npart {} {{\n", sanitize_ident(part_name));
    if let Value::Object(map) = json {
        for (k, v) in map {
            out.push_str(&format!("    attribute {k} : {};\n", sysml_scalar_literal(&v)));
        }
    }
    out.push_str("}\n");
    out
}

/// Indent every line of an already-rendered `to_sysml_v2()` block by one
/// level (4 spaces), for embedding as a nested `part` inside a composite
/// type's own block. Trailing newline in, trailing newline out.
pub fn indent_block(text: &str) -> String {
    text.lines().map(|l| format!("    {l}\n")).collect()
}

/// Sanitize an arbitrary string into a legal bare SysML v2 identifier
/// (`[A-Za-z_][A-Za-z0-9_]*`). Used for part names derived from
/// user-authored content (proposal IDs, decision/risk names) that may
/// contain spaces, hyphens, or other characters a bare identifier can't.
pub fn sanitize_ident(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if out.is_empty() || out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn sysml_scalar_literal(v: &Value) -> String {
    match v {
        Value::String(s) => format!("ScalarValues::String = {s:?}"),
        Value::Bool(b) => format!("ScalarValues::Boolean = {b}"),
        Value::Number(n) if n.is_i64() || n.is_u64() => format!("ScalarValues::Integer = {n}"),
        Value::Number(n) => format!("ScalarValues::Real = {n}"),
        Value::Null => "ScalarValues::String = \"\"".to_string(),
        Value::Array(items) if items.iter().all(|i| i.is_string()) => {
            let joined = items
                .iter()
                .filter_map(|i| i.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            format!("ScalarValues::String = {joined:?}")
        }
        Value::Array(_) | Value::Object(_) => "ScalarValues::String = \"<nested>\"".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_ident_replaces_illegal_characters() {
        assert_eq!(sanitize_ident("DARED-001"), "DARED_001");
        assert_eq!(sanitize_ident("has spaces"), "has_spaces");
    }

    #[test]
    fn sanitize_ident_prefixes_leading_digit() {
        assert_eq!(sanitize_ident("001"), "_001");
    }

    #[test]
    fn sanitize_ident_never_empty() {
        assert_eq!(sanitize_ident(""), "_");
    }
}
