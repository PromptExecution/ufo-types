//! Qualified-name-addressed incremental model edits — the `hasA` / `isA`
//! triples of the SysMD notebook dialect (`tukcps/SysMD`,
//! `doc/SysMDLanguageExtensions.md`), as data.
//!
//! ```text
//! Subject  hasA  <element list> .     add owned element(s) to a namespace
//! Subject  isA   <Type> .             (re)type an existing classifier
//! ```
//!
//! `Subject` is an existing element's qualified name (`Pkg::Sub::Item`). The
//! point of the form is that an edit is *addressed* and *small*: a generator
//! (or an LLM) states "add this to that" instead of re-emitting a whole
//! package, and a reviewer can read the diff. This type parses and prints the
//! triple; it does not apply it and does not parse the element text, which
//! stays opaque SysML for a real parser (`sysml-v2-parser`) to check.
//!
//! This is the SysMD *dialect*, not standard SysML v2 text. Keep it at the
//! edge: a [`ModelEdit`] is an instruction, never a stored model element.

use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A malformed edit.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModelEditError {
    #[error("an edit must end with `.`")]
    MissingTerminator,
    #[error("expected `<QualifiedName> hasA|isA <rest> .`")]
    Shape,
    #[error("`{0}` is not a plain qualified name (identifiers joined by `::`)")]
    QualifiedName(String),
    #[error("`hasA` needs at least one element after it")]
    EmptyElements,
}

/// One incremental edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "edit")]
pub enum ModelEdit {
    /// `owner hasA <elements> .` — add owned element(s) (opaque SysML text).
    HasA { owner: String, elements: String },
    /// `subject isA <ty> .` — type or specialize an existing element.
    IsA { subject: String, ty: String },
}

/// Plain qualified names only: `Ident(::Ident)*`, `Ident = [A-Za-z_][A-Za-z0-9_]*`.
/// Quoted (`'my part'`) unrestricted names are not accepted.
pub fn is_qualified_name(s: &str) -> bool {
    !s.is_empty()
        && s.split("::").all(|seg| {
            let mut chars = seg.chars();
            matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

impl FromStr for ModelEdit {
    type Err = ModelEditError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let body = s
            .trim()
            .strip_suffix('.')
            .ok_or(ModelEditError::MissingTerminator)?;
        let body = body.trim_end();
        let (subject, rest) = body
            .split_once(char::is_whitespace)
            .ok_or(ModelEditError::Shape)?;
        if !is_qualified_name(subject) {
            return Err(ModelEditError::QualifiedName(subject.to_string()));
        }
        let rest = rest.trim_start();
        let (keyword, tail) = match rest.split_once(char::is_whitespace) {
            Some((k, t)) => (k, t.trim()),
            None => (rest, ""),
        };
        match keyword {
            "hasA" => {
                if tail.is_empty() {
                    return Err(ModelEditError::EmptyElements);
                }
                Ok(ModelEdit::HasA {
                    owner: subject.to_string(),
                    elements: tail.to_string(),
                })
            }
            "isA" => {
                if !is_qualified_name(tail) {
                    return Err(ModelEditError::QualifiedName(tail.to_string()));
                }
                Ok(ModelEdit::IsA {
                    subject: subject.to_string(),
                    ty: tail.to_string(),
                })
            }
            _ => Err(ModelEditError::Shape),
        }
    }
}

impl std::fmt::Display for ModelEdit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelEdit::HasA { owner, elements } => write!(f, "{owner} hasA {elements}."),
            ModelEdit::IsA { subject, ty } => write!(f, "{subject} isA {ty}."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_sysmd_examples() {
        assert_eq!(
            "Global hasA package Vehicles."
                .parse::<ModelEdit>()
                .unwrap(),
            ModelEdit::HasA {
                owner: "Global".into(),
                elements: "package Vehicles".into()
            }
        );
        assert_eq!(
            "Vehicles::Car isA Vehicle.".parse::<ModelEdit>().unwrap(),
            ModelEdit::IsA {
                subject: "Vehicles::Car".into(),
                ty: "Vehicle".into()
            }
        );
    }

    #[test]
    fn element_lists_may_contain_dots_semicolons_and_trailing_space_dot() {
        let e: ModelEdit =
            "p::Car hasA\n    import ScalarValues::Real;\n    attribute speed: Real = 10.0 ."
                .parse()
                .unwrap();
        match e {
            ModelEdit::HasA { owner, elements } => {
                assert_eq!(owner, "p::Car");
                assert!(elements.starts_with("import ScalarValues::Real;"));
                assert!(elements.ends_with("= 10.0"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn display_round_trips() {
        for src in [
            "Global hasA package Vehicles.",
            "Vehicles::Car isA Vehicle.",
            "A hasA attribute x: Real = 10..", // element text that itself ends in a dot
        ] {
            let e: ModelEdit = src.parse().unwrap();
            assert_eq!(e.to_string(), src);
            assert_eq!(e.to_string().parse::<ModelEdit>().unwrap(), e);
        }
    }

    #[test]
    fn rejects_malformed_edits() {
        assert_eq!(
            "A hasA part x".parse::<ModelEdit>(),
            Err(ModelEditError::MissingTerminator)
        );
        assert_eq!(
            "A hasA .".parse::<ModelEdit>(),
            Err(ModelEditError::EmptyElements)
        );
        assert_eq!(
            "A hasA".parse::<ModelEdit>(),
            Err(ModelEditError::MissingTerminator)
        );
        assert!(matches!(
            "A::  isA B.".parse::<ModelEdit>(),
            Err(ModelEditError::QualifiedName(_))
        ));
        assert!(matches!(
            "A isA 'My Type'.".parse::<ModelEdit>(),
            Err(ModelEditError::QualifiedName(_))
        ));
        assert!(matches!(
            "1bad hasA part x.".parse::<ModelEdit>(),
            Err(ModelEditError::QualifiedName(_))
        ));
        assert_eq!(
            "A contains part x.".parse::<ModelEdit>(),
            Err(ModelEditError::Shape)
        );
        assert_eq!(
            "".parse::<ModelEdit>(),
            Err(ModelEditError::MissingTerminator)
        );
    }

    #[test]
    fn qualified_names() {
        assert!(is_qualified_name("A"));
        assert!(is_qualified_name("Pkg::Sub::_item2"));
        assert!(!is_qualified_name(""));
        assert!(!is_qualified_name("A::"));
        assert!(!is_qualified_name("::A"));
        assert!(!is_qualified_name("A:B"));
        assert!(!is_qualified_name("A B"));
    }
}
