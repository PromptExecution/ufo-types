//! Versioned wire-contract identity and forward-only decode.
//!
//! Sits *beside* the canonical typed layer, never inside it: per
//! `kr0ki`'s `docs/DESIGN-NOTE-typed-model-layer.md` §2.8, a canonical type
//! (`sysgraph::SysGraph`, `mbse::requirements::RequirementGraph`) carries no
//! `{major, minor}` field of its own — "stability is enforced by golden
//! fixtures + sha256 content hashes + frozen wire constants... a version
//! field invites the drift the fixture discipline exists to prevent."
//! [`DialectUrn`] and [`Upgrade`] name and decode a *wire artifact* that
//! produces one of those types; they never become a field on the type
//! itself.
//!
//! Distinct from an *adapter* (e.g. `reqif`'s ReqIF-XML -> `RequirementGraph`
//! lowering): an adapter crosses from a foreign format this crate does not
//! own the evolution of into a canonical type, once, with no version
//! negotiation. [`Upgrade`] is for decoding the *same* dialect — a format
//! this crate owns both ends of — at a historical minor/patch version
//! forward to the current shape.

use std::fmt;

/// A stable identity for one versioned wire dialect of a canonical type.
///
/// Renders as `urn:b00t:dialect:<crate>:<path>:<semver>`, e.g.
/// `urn:b00t:dialect:ufo-types:mbse::requirements:0.15.0` — unifying the two
/// ad hoc conventions already in use in kr0ki-core before this module
/// existed: `b00t.type/reqif-import/v1` (envelope identity, major-only) and
/// `ufo-types/0.15.0:mbse::requirements` (crate + full semver + path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialectUrn {
    pub crate_name: &'static str,
    pub path: &'static str,
    pub version: semver::Version,
}

impl fmt::Display for DialectUrn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "urn:b00t:dialect:{}:{}:{}",
            self.crate_name, self.path, self.version
        )
    }
}

/// Why an [`Upgrade::upgrade`] call failed.
#[derive(Debug, thiserror::Error)]
pub enum DialectError {
    /// `from_version`'s major component does not match the target type's
    /// `Upgrade::DIALECT` major. A major bump is a breaking wire change —
    /// `upgrade()` never attempts it; the caller needs an explicit,
    /// separately-versioned migration instead.
    #[error(
        "dialect major-version mismatch: bytes declare {from_version} but {dialect} only upgrades within major {expected_major}"
    )]
    MajorMismatch {
        from_version: semver::Version,
        // Boxed: `DialectUrn` embeds a `semver::Version` too, and clippy's
        // `result_large_err` flags `Result<Self, DialectError>` (this is
        // `Upgrade::upgrade`'s return type) as too large to return by value
        // otherwise.
        dialect: Box<DialectUrn>,
        expected_major: u64,
    },
    /// The bytes did not decode as this dialect at the declared version.
    #[error("malformed {dialect} payload at declared version {from_version}: {reason}")]
    Malformed {
        dialect: Box<DialectUrn>,
        from_version: semver::Version,
        reason: String,
    },
}

/// Forward-only decode: turn versioned wire bytes into the current
/// in-process value.
///
/// There is deliberately no `downgrade()` on this trait. Concurrency is
/// structural rather than incidental: `upgrade()` is a plain function per
/// call with no shared mutable dispatch state, so many callers can decode
/// many sources at many declared versions at once with zero contention —
/// e.g. a `b00t://` loader iterating several concurrently-registered
/// sources, each at its own historical minor/patch version of the same
/// dialect.
pub trait Upgrade: Sized {
    /// This type's current dialect identity. Bump the version here as this
    /// type's wire contract evolves within the same major; existing
    /// `from_version` handling in `upgrade()` accrues, it never drops
    /// support for a version this type has shipped at the current major.
    const DIALECT: DialectUrn;

    /// Decode `bytes`, produced against `from_version` of this same
    /// dialect. Returns [`DialectError::MajorMismatch`] without attempting
    /// a decode when the majors differ. Every `from_version` this type has
    /// ever shipped at the current major MUST succeed — enforced by a
    /// golden-fixture round-trip test per historical version, the same bar
    /// `kr0ki-core`'s `normalized_graph_sha256` already sets for a single
    /// version, applied per version instead of once.
    fn upgrade(bytes: &[u8], from_version: &semver::Version) -> Result<Self, DialectError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialect_urn_renders_as_urn_b00t_dialect() {
        let urn = DialectUrn {
            crate_name: "ufo-types",
            path: "mbse::requirements",
            version: semver::Version::new(0, 15, 0),
        };
        assert_eq!(
            urn.to_string(),
            "urn:b00t:dialect:ufo-types:mbse::requirements:0.15.0"
        );
    }
}
