#![cfg(feature = "revision")]

use serde::Deserialize;
use ufo_types::revision::{ChangeSet, ConflictKind, MergeOutcome, PortableModel, merge_models};

#[derive(Deserialize)]
struct Edit {
    pointer: String,
    value: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    ours: Vec<Edit>,
    theirs: Vec<Edit>,
    conflicts: Vec<ConflictKind>,
    merged: Option<Vec<Edit>>,
}

fn base() -> PortableModel {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/portable_revision.json")).unwrap();
    serde_json::from_value(fixture["model"].clone()).unwrap()
}

fn edited(base: &PortableModel, edits: &[Edit]) -> PortableModel {
    let mut value = serde_json::to_value(base).unwrap();
    for edit in edits {
        let (parent, key) = edit.pointer.rsplit_once('/').unwrap();
        let key = key.replace("~1", "/").replace("~0", "~");
        let object = value.pointer_mut(parent).unwrap().as_object_mut().unwrap();
        match &edit.value {
            Some(replacement) => {
                object.insert(key, replacement.clone());
            }
            None => {
                object.remove(&key);
            }
        }
    }
    serde_json::from_value(value).unwrap()
}

#[test]
fn concurrent_proposals_merge_or_return_typed_conflicts() {
    let base = base();
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("fixtures/revision_merge_cases.json")).unwrap();
    for case in cases {
        let ours = edited(&base, &case.ours);
        let theirs = edited(&base, &case.theirs);
        let result = merge_models(&base, &ours, &theirs).unwrap();
        let reverse = merge_models(&base, &theirs, &ours).unwrap();
        match result {
            MergeOutcome::Merged { model, changes } => {
                assert!(case.conflicts.is_empty(), "{}", case.name);
                let expected = edited(&base, case.merged.as_ref().unwrap());
                assert_eq!(model, expected, "{}", case.name);
                assert_eq!(changes.apply(&theirs).unwrap(), expected, "{}", case.name);
                let MergeOutcome::Merged {
                    model: reversed, ..
                } = reverse
                else {
                    panic!("asymmetric merge: {}", case.name);
                };
                assert_eq!(model, reversed, "{}", case.name);
                let proposal = ChangeSet::between(&base, &ours).unwrap();
                assert_eq!(proposal.apply(&base).unwrap(), ours, "{}", case.name);
            }
            MergeOutcome::Conflicted { conflicts } => {
                assert_eq!(
                    conflicts.iter().map(|c| c.kind).collect::<Vec<_>>(),
                    case.conflicts,
                    "{}",
                    case.name
                );
                let MergeOutcome::Conflicted {
                    conflicts: reversed,
                } = reverse
                else {
                    panic!("asymmetric conflict: {}", case.name);
                };
                assert_eq!(
                    conflicts
                        .iter()
                        .map(|c| (&c.path, c.kind))
                        .collect::<Vec<_>>(),
                    reversed
                        .iter()
                        .map(|c| (&c.path, c.kind))
                        .collect::<Vec<_>>(),
                    "{}",
                    case.name
                );
            }
        }
    }
}

#[test]
fn changes_are_preconditioned_and_detect_tampering() {
    let base = base();
    let mut proposed = base.clone();
    proposed.elements.get_mut("controller").unwrap().name = "renamed".into();
    let changes = ChangeSet::between(&base, &proposed).unwrap();
    assert_eq!(
        ChangeSet::from_bytes(&changes.to_bytes().unwrap()).unwrap(),
        changes
    );
    assert!(changes.apply(&proposed).is_err());
    let mut tampered = changes.clone();
    tampered.elements.clear();
    assert!(tampered.apply(&base).is_err());
    let mut tampered = changes.clone();
    tampered.elements.get_mut("controller").unwrap().before = None;
    assert!(tampered.apply(&base).is_err());
    assert_eq!(
        ChangeSet::between(&base, &base)
            .unwrap()
            .apply(&base)
            .unwrap(),
        base
    );
}

#[test]
fn changes_reject_duplicate_keys_and_nested_data_loss() {
    let base = base();
    let mut proposed = base.clone();
    proposed.relations.get_mut("enter").unwrap().anchors =
        base.elements["controller"].anchors.clone();
    let changes = ChangeSet::between(&base, &proposed).unwrap();
    let mut input = serde_json::to_value(&changes).unwrap();
    input["relations"]["enter"]["after"]["relation"]["succession"]["future_guard"] =
        serde_json::Value::Bool(true);
    assert!(ChangeSet::from_bytes(&serde_json::to_vec(&input).unwrap()).is_err());
    let encoded = String::from_utf8(changes.to_bytes().unwrap()).unwrap();
    let duplicate = format!("{{\"elements\":{{}},{}", &encoded[1..]);
    assert!(ChangeSet::from_bytes(duplicate.as_bytes()).is_err());
}
