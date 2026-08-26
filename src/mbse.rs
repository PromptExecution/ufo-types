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
            out.push_str(&render_field(&k, &v));
        }
    }
    out.push_str("}\n");
    out
}

/// Renders one field as SysML v2 lines inside a `part` block, each already
/// indented one level (4 spaces) and newline-terminated, ready to append
/// directly into the parent block's body:
///
/// - a plain scalar -> one `attribute` line (multiplicity `[1..1]` is
///   SysML v2's implicit default, so it's left unwritten)
/// - `Option::None` (JSON `null`) -> an unset `attribute` declared
///   `[0..1]`, not a bogus empty-string value. Note `Option::Some(x)`
///   serializes identically to a plain `x` -- JSON alone can't distinguish
///   "optional but present" from "required", so that case renders as a
///   plain scalar like any required field.
/// - a homogeneous array of scalars (what any `Vec<T: Serialize>` for a
///   scalar `T` produces) -> one `attribute` line typed `[0..*]` with a
///   parenthesized sequence-literal value
/// - a homogeneous array of objects (`Vec<Struct>`) -> one nested `part`
///   per element, numbered `{name}_0`, `{name}_1`, ... -- SysML v2 has no
///   "sequence of parts" attribute form, so a nested part per element is
///   the idiomatic containment shape
/// - a JSON object (a nested struct field) -> a real nested `part` block,
///   recursively dumped, not a placeholder
/// - anything else (mixed-type or nested arrays -- only possible from an
///   enum whose variants serialize to different JSON shapes, since a
///   Rust `Vec<T>` is otherwise always element-uniform) -> an honest
///   placeholder; genuinely out of scope for a generic field reflector.
fn render_field(name: &str, v: &Value) -> String {
    let id = sanitize_ident(name);
    match v {
        Value::Null => format!("    attribute {id} : ScalarValues::String[0..1];\n"),
        Value::Object(map) => {
            let mut inner = format!("part {id} {{\n");
            for (k, v) in map {
                inner.push_str(&render_field(k, v));
            }
            inner.push_str("}\n");
            indent_block(&inner)
        }
        Value::Array(items) if items.is_empty() => {
            format!("    attribute {id} : ScalarValues::String[0..*] = ();\n")
        }
        Value::Array(items) if items.iter().all(|i| matches!(i, Value::Object(_))) => items
            .iter()
            .enumerate()
            .map(|(i, item)| render_field(&format!("{name}_{i}"), item))
            .collect(),
        Value::Array(items) if items.iter().all(|i| matches!(i, Value::String(_))) => {
            let joined = items
                .iter()
                .filter_map(|i| i.as_str())
                .map(|s| format!("{s:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("    attribute {id} : ScalarValues::String[0..*] = ({joined});\n")
        }
        Value::Array(items) if items.iter().all(|i| matches!(i, Value::Bool(_))) => {
            let joined = items
                .iter()
                .filter_map(|i| i.as_bool())
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!("    attribute {id} : ScalarValues::Boolean[0..*] = ({joined});\n")
        }
        Value::Array(items) if items.iter().all(|i| i.is_i64() || i.is_u64()) => {
            let joined = items.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", ");
            format!("    attribute {id} : ScalarValues::Integer[0..*] = ({joined});\n")
        }
        Value::Array(items) if items.iter().all(|i| i.is_number()) => {
            let joined = items.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", ");
            format!("    attribute {id} : ScalarValues::Real[0..*] = ({joined});\n")
        }
        Value::Array(_) => {
            format!("    attribute {id} : ScalarValues::String[0..*] = \"<nested>\";\n")
        }
        scalar => format!("    attribute {id} : {};\n", sysml_scalar_literal(scalar)),
    }
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

/// Renders a plain JSON scalar (bool/number/string) as a SysML v2 attribute
/// value expression. `render_field` handles `Null`/`Array`/`Object` itself
/// before ever reaching here, so the fallback below is defensive only.
fn sysml_scalar_literal(v: &Value) -> String {
    match v {
        Value::String(s) => format!("ScalarValues::String = {s:?}"),
        Value::Bool(b) => format!("ScalarValues::Boolean = {b}"),
        Value::Number(n) if n.is_i64() || n.is_u64() => format!("ScalarValues::Integer = {n}"),
        Value::Number(n) => format!("ScalarValues::Real = {n}"),
        Value::Null | Value::Array(_) | Value::Object(_) => {
            "ScalarValues::String = \"<nested>\"".to_string()
        }
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

    // ── Vec/Option multiplicity + nested-struct field dumps ──

    #[derive(Serialize)]
    struct Address {
        city: String,
        zip: String,
    }

    #[derive(Serialize)]
    struct Person {
        name: String,
        tags: Vec<String>,
        scores: Vec<i64>,
        nickname: Option<String>,
        title: Option<String>,
        home: Address,
        offices: Vec<Address>,
    }

    fn sample_person() -> Person {
        Person {
            name: "Ada".into(),
            tags: vec!["engineer".into(), "founder".into()],
            scores: vec![9, 10],
            nickname: Some("Countess".into()),
            title: None,
            home: Address { city: "London".into(), zip: "SW1".into() },
            offices: vec![
                Address { city: "Paris".into(), zip: "75001".into() },
                Address { city: "Berlin".into(), zip: "10115".into() },
            ],
        }
    }

    #[test]
    fn vec_of_strings_gets_star_multiplicity_and_a_sequence_literal() {
        let text = mbse_field_dump(&UfoStereotype::Kind("Person".into()), "Person", &sample_person());
        assert!(
            text.contains(r#"attribute tags : ScalarValues::String[0..*] = ("engineer", "founder");"#),
            "{text}"
        );
    }

    #[test]
    fn vec_of_integers_gets_star_multiplicity_and_a_sequence_literal() {
        let text = mbse_field_dump(&UfoStereotype::Kind("Person".into()), "Person", &sample_person());
        assert!(
            text.contains("attribute scores : ScalarValues::Integer[0..*] = (9, 10);"),
            "{text}"
        );
    }

    #[test]
    fn option_none_is_an_unset_attribute_not_a_bogus_empty_string() {
        let text = mbse_field_dump(&UfoStereotype::Kind("Person".into()), "Person", &sample_person());
        assert!(
            text.contains("attribute title : ScalarValues::String[0..1];"),
            "{text}"
        );
        assert!(!text.contains(r#"title : ScalarValues::String = """#), "{text}");
    }

    #[test]
    fn option_some_renders_as_a_plain_scalar() {
        let text = mbse_field_dump(&UfoStereotype::Kind("Person".into()), "Person", &sample_person());
        assert!(
            text.contains(r#"attribute nickname : ScalarValues::String = "Countess";"#),
            "{text}"
        );
    }

    #[test]
    fn nested_struct_becomes_a_real_nested_part_not_a_placeholder() {
        let text = mbse_field_dump(&UfoStereotype::Kind("Person".into()), "Person", &sample_person());
        assert!(text.contains("    part home {\n"), "{text}");
        assert!(text.contains(r#"        attribute city : ScalarValues::String = "London";"#), "{text}");
        assert!(!text.contains("<nested>"), "{text}");
    }

    #[test]
    fn vec_of_structs_becomes_one_numbered_nested_part_per_element() {
        let text = mbse_field_dump(&UfoStereotype::Kind("Person".into()), "Person", &sample_person());
        assert!(text.contains("    part offices_0 {\n"), "{text}");
        assert!(text.contains(r#"        attribute city : ScalarValues::String = "Paris";"#), "{text}");
        assert!(text.contains("    part offices_1 {\n"), "{text}");
        assert!(text.contains(r#"        attribute city : ScalarValues::String = "Berlin";"#), "{text}");
    }

    #[cfg(feature = "sysml")]
    #[test]
    fn vec_option_and_nested_struct_dump_is_syntactically_valid_sysml_v2() {
        use crate::sysml::validate_sysml_v2;

        let text = mbse_field_dump(&UfoStereotype::Kind("Person".into()), "Person", &sample_person());
        let wrapped = format!("package MbseExport {{\n{}}}\n", indent_block(&text));
        let result = validate_sysml_v2(&wrapped);
        assert!(
            result.disposition.is_satisfied(),
            "generated SysML v2 failed to parse: {:?}\n\n{wrapped}",
            result.disposition
        );
    }
}
