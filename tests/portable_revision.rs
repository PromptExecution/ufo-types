#![cfg(feature = "revision")]

use std::collections::BTreeMap;

use serde::Deserialize;
use ufo_types::revision::*;

#[derive(Deserialize)]
struct Fixture {
    context: RevisionContext,
    model: PortableModel,
    artifacts: BTreeMap<ArtifactPath, Vec<u8>>,
    expected_semantic_digest: ArtifactDigest,
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("fixtures/portable_revision.json")).unwrap()
}

fn bundle() -> PortableBundle {
    let f = fixture();
    PortableBundle::dehydrate(f.model, f.context, f.artifacts).unwrap()
}

#[test]
fn fixture_roundtrip_preserves_semantics_and_every_original_byte() {
    let f = fixture();
    let original = f.model.clone();
    let bundle = PortableBundle::dehydrate(f.model, f.context, f.artifacts.clone()).unwrap();
    assert_eq!(bundle.manifest.semantic_digest, f.expected_semantic_digest);
    let bytes = bundle.to_bytes().unwrap();
    let hydrated = PortableBundle::from_bytes(&bytes).unwrap();
    assert_eq!(hydrated.hydrate().unwrap(), original);
    assert_eq!(hydrated.to_bytes().unwrap(), bytes);
    for (path, expected) in &f.artifacts {
        assert_eq!(
            &hydrated.blobs[&hydrated.manifest.artifacts[path]],
            expected
        );
    }
    // The fixture includes Unicode source IDs, BOM/CRLF bytes, binary attachments,
    // timestamps, nested collections, unassigned attributes and cyclic dependencies.
    assert!(hydrated.model.elements.contains_key("REQ-日本語"));
}

#[test]
fn a_fresh_project_keeps_the_same_model_digest() {
    let original = bundle();
    let mut context = original.manifest.context.clone();
    context.project = ProjectId::new("fresh-project").unwrap();
    let rebuilt =
        PortableBundle::dehydrate(original.hydrate().unwrap(), context, fixture().artifacts)
            .unwrap();
    assert_eq!(
        rebuilt.manifest.semantic_digest,
        original.manifest.semantic_digest
    );
    assert_ne!(
        rebuilt.manifest.context.project,
        original.manifest.context.project
    );
}

#[derive(Deserialize)]
struct InvalidCase {
    pointer: String,
    value: serde_json::Value,
    reason: String,
}

#[test]
fn structural_and_typed_invariants_reject_invalid_fixture_variants() {
    let cases: Vec<InvalidCase> =
        serde_json::from_str(include_str!("fixtures/portable_invalid_cases.json")).unwrap();
    for case in cases {
        let mut data: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/portable_revision.json")).unwrap();
        *data.pointer_mut(&case.pointer).unwrap() = case.value;
        let f: Fixture = serde_json::from_value(data).unwrap();
        let error = PortableBundle::dehydrate(f.model, f.context, f.artifacts).unwrap_err();
        assert!(
            error.to_string().contains(&case.reason),
            "{}: {error}",
            case.pointer
        );
    }
}

#[test]
fn missing_corrupt_and_unreferenced_blobs_are_rejected() {
    let original = bundle();
    let digest = original.blobs.keys().next().unwrap().clone();
    let mut missing = original.clone();
    missing.blobs.remove(&digest);
    assert!(matches!(
        missing.hydrate(),
        Err(RevisionError::MissingArtifact(_))
    ));
    let mut corrupt = original.clone();
    corrupt.blobs.get_mut(&digest).unwrap().push(0);
    assert!(matches!(
        corrupt.hydrate(),
        Err(RevisionError::DigestMismatch(_))
    ));
    let mut extra = original;
    extra.blobs.insert(
        ArtifactDigest::of(b"unreferenced"),
        b"unreferenced".to_vec(),
    );
    assert!(matches!(
        extra.hydrate(),
        Err(RevisionError::InvalidModel { .. })
    ));
}

#[test]
fn source_evidence_requires_its_preserved_artifact() {
    let mut bundle = bundle();
    let path = ArtifactPath::new("src/controller.rs").unwrap();
    let digest = bundle.manifest.artifacts.remove(&path).unwrap();
    bundle.blobs.remove(&digest);
    assert!(matches!(
        bundle.hydrate(),
        Err(RevisionError::MissingArtifact(_))
    ));
}

#[test]
fn model_tampering_and_future_dialects_are_rejected() {
    let mut changed = bundle();
    changed.model.elements.get_mut("controller").unwrap().name = "changed".into();
    assert!(matches!(
        changed.hydrate(),
        Err(RevisionError::DigestMismatch(_))
    ));
    let mut future = bundle();
    future.manifest.dialect = "urn:b00t:dialect:ufo-types:revision:2.0.0".into();
    assert!(matches!(
        future.hydrate(),
        Err(RevisionError::UnsupportedDialect(_))
    ));
}

#[test]
fn data_loss_is_a_structured_error_not_successful_hydration() {
    let mut bundle = bundle();
    bundle.manifest.fidelity.issues.push(FidelityIssue {
        path: "relations/verify-1".into(),
        kind: FidelityKind::Dropped,
        reason: "server dropped the verification trace".into(),
    });
    assert!(matches!(
        bundle.hydrate(),
        Err(RevisionError::FidelityLoss(_))
    ));
    assert_eq!(
        serde_json::to_value(bundle.manifest.fidelity).unwrap()["issues"][0]["kind"],
        "dropped"
    );
}

#[test]
fn duplicate_wire_identities_and_unknown_nested_fields_are_rejected() {
    let bytes = bundle().to_bytes().unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let duplicate = text.replacen(
        "\"context\":{",
        "\"context\":{\"project\":\"duplicate\",",
        1,
    );
    assert!(
        PortableBundle::from_bytes(duplicate.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("duplicate JSON key")
    );
    let mut unknown: serde_json::Value = serde_json::from_str(&text).unwrap();
    unknown["model"]["relations"]["verify-1"]["relation"]["verify"]["surprise"] = true.into();
    assert!(
        PortableBundle::from_bytes(&serde_json::to_vec(&unknown).unwrap())
            .unwrap_err()
            .to_string()
            .contains("unknown field would be lost")
    );
}

#[test]
fn artifact_paths_are_portable_and_cannot_escape_a_restore_root() {
    let paths: Vec<String> =
        serde_json::from_str(include_str!("fixtures/invalid_artifact_paths.json")).unwrap();
    for path in paths {
        assert!(ArtifactPath::new(path).is_err());
    }
    assert_eq!(
        ArtifactPath::new("model/日本語.sysml").unwrap().as_str(),
        "model/日本語.sysml"
    );
}

#[test]
fn wire_identities_are_strings_and_digests_are_strict() {
    assert!(serde_json::from_str::<ProjectId>("17").is_err());
    assert!(ProjectId::new(" ").is_err());
    assert!(ArtifactDigest::try_from("sha256:abc".to_string()).is_err());
    assert_eq!(
        ArtifactDigest::of(b"abc").as_str(),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn canonical_objects_and_public_schema_are_deterministic() {
    let b = bundle();
    let mut value = serde_json::to_value(&b).unwrap();
    // Reinsert nested extension data in reverse key order; semantic bytes stay fixed.
    let ext = value["model"]["elements"]["REQ-日本語"]["extensions"]
        .as_object_mut()
        .unwrap();
    let entries: Vec<_> = ext.clone().into_iter().collect();
    ext.clear();
    for (key, value) in entries.into_iter().rev() {
        ext.insert(key, value);
    }
    assert_eq!(canonical_bytes(&value).unwrap(), b.to_bytes().unwrap());
    let schema = schemars::schema_for!(PortableBundle);
    assert!(schema.definitions.contains_key("ModelRelation"));
    assert!(schema.definitions.contains_key("SourceEvidence"));
    assert_eq!(
        canonical_bytes(&schema).unwrap(),
        canonical_bytes(&schemars::schema_for!(PortableBundle)).unwrap()
    );
}

#[test]
fn strict_server_capabilities_report_exact_metadata_loss() {
    let mut capabilities: AdapterCapabilities =
        serde_json::from_str(include_str!("fixtures/portable_adapter.json")).unwrap();
    let bundle = bundle();
    assert!(
        bundle
            .adapter_fidelity(&capabilities)
            .require_lossless()
            .is_ok()
    );
    capabilities.preserves_extensions = false;
    capabilities.preserves_source_evidence = false;
    let report = bundle.adapter_fidelity(&capabilities);
    assert_eq!(report.issues.len(), 2);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.path == "elements/REQ-日本語/extensions")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.path == "elements/controller/anchors")
    );
    assert!(report.require_lossless().is_err());
}

#[test]
fn properties_and_fact_authority_require_verified_capabilities() {
    let mut capabilities: AdapterCapabilities =
        serde_json::from_str(include_str!("fixtures/portable_adapter.json")).unwrap();
    capabilities.preserves_properties = false;
    capabilities.preserves_fact_authority = false;
    let bundle = bundle();
    let report = bundle.adapter_fidelity(&capabilities);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.path == "elements/controller/properties")
    );
    for id in bundle.model.relations.keys() {
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.path == format!("relations/{id}/authority"))
        );
    }
    assert!(report.require_lossless().is_err());
    let mut old = serde_json::to_value(&capabilities).unwrap();
    old.as_object_mut().unwrap().remove("preserves_properties");
    old.as_object_mut()
        .unwrap()
        .remove("preserves_fact_authority");
    let unprobed: AdapterCapabilities = serde_json::from_value(old).unwrap();
    assert!(!unprobed.preserves_properties && !unprobed.preserves_fact_authority);
}

#[test]
fn bundle_digest_binds_source_bytes_and_context_independently_of_model() {
    let original = bundle();
    let mut fixture = fixture();
    let path = ArtifactPath::new("src/controller.rs").unwrap();
    fixture.artifacts.get_mut(&path).unwrap().push(b' ');
    let modified =
        PortableBundle::dehydrate(fixture.model, fixture.context, fixture.artifacts).unwrap();
    assert_eq!(
        original.manifest.semantic_digest,
        modified.manifest.semantic_digest
    );
    assert_ne!(
        original.bundle_digest().unwrap(),
        modified.bundle_digest().unwrap()
    );
    let mut other_project = original.clone();
    other_project.manifest.context.project = ProjectId::new("project-2").unwrap();
    assert_ne!(
        original.bundle_digest().unwrap(),
        other_project.bundle_digest().unwrap()
    );
    assert_eq!(
        original.bundle_digest().unwrap(),
        PortableBundle::from_bytes(&original.to_bytes().unwrap())
            .unwrap()
            .bundle_digest()
            .unwrap()
    );
}

#[test]
fn exact_real_lexemes_round_trip_beyond_machine_float_range() {
    #[derive(Deserialize)]
    struct Case {
        lexeme: String,
        valid: bool,
    }
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("fixtures/portable_real_cases.json")).unwrap();
    for case in cases {
        let mut fixture = fixture();
        let mut property = fixture.model.elements["controller"].properties["label"].clone();
        property.type_ref = TypeRef::Real;
        property.values = Some(vec![TypedValue::Real(case.lexeme.clone())]);
        fixture
            .model
            .elements
            .get_mut("controller")
            .unwrap()
            .properties
            .insert("real".into(), property);
        let result = PortableBundle::dehydrate(fixture.model, fixture.context, fixture.artifacts);
        if case.valid {
            let bundle = result.unwrap();
            let hydrated = PortableBundle::from_bytes(&bundle.to_bytes().unwrap())
                .unwrap()
                .hydrate()
                .unwrap();
            assert_eq!(
                hydrated.elements["controller"].properties["real"].values,
                Some(vec![TypedValue::Real(case.lexeme)])
            );
        } else {
            assert!(result.is_err(), "{}", case.lexeme);
        }
    }
}

#[test]
fn external_types_require_a_pinned_library_and_parent_cannot_be_self() {
    let mut bundle = bundle();
    bundle.manifest.context.parent = Some(bundle.manifest.context.revision.clone());
    assert!(
        bundle
            .hydrate()
            .unwrap_err()
            .to_string()
            .contains("own parent")
    );
    bundle.manifest.context.parent = None;
    let property = bundle
        .model
        .elements
        .get_mut("controller")
        .unwrap()
        .properties
        .get_mut("label")
        .unwrap();
    property.type_ref = TypeRef::External {
        library: "domain-library".into(),
        qualified_name: "Domain::Label".into(),
    };
    bundle.manifest.semantic_digest = bundle.model.semantic_digest().unwrap();
    assert!(
        bundle
            .hydrate()
            .unwrap_err()
            .to_string()
            .contains("no pinned revision")
    );
    bundle.manifest.context.library_revisions.insert(
        "domain-library".into(),
        RevisionId::new("git:domain-1").unwrap(),
    );
    assert!(bundle.hydrate().is_ok());
}
