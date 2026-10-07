#![cfg(feature = "revision")]

use serde::Deserialize;
use serde_json::Value;
use ufo_types::revision::*;

#[derive(Deserialize)]
struct Mutation {
    base: usize,
    pointer: String,
    value: Value,
    reason: String,
}

#[derive(Deserialize)]
struct Pair {
    request: usize,
    response: usize,
    valid: bool,
}

#[derive(Deserialize)]
struct Inconsistent {
    response: usize,
    pointer: String,
    value: Value,
}

#[derive(Deserialize)]
struct Fixture {
    requests: Vec<Value>,
    responses: Vec<Value>,
    request_response_cases: Vec<Pair>,
    invalid_requests: Vec<Mutation>,
    invalid_responses: Vec<Mutation>,
    inconsistent_pairs: Vec<Inconsistent>,
    duplicate_json: Vec<String>,
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("fixtures/revision_queries.json")).unwrap()
}

fn replace(value: &mut Value, pointer: &str, replacement: Value) {
    if let Some(target) = value.pointer_mut(pointer) {
        *target = replacement;
    } else {
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        value
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(key.into(), replacement);
    }
}

#[test]
fn wire_roundtrip_preserves_terms_unbound_values_order_and_explicit_states() {
    let f = fixture();
    for value in &f.requests {
        let request: RevisionQueryRequest = serde_json::from_value(value.clone()).unwrap();
        request.validate().unwrap();
        assert_eq!(&serde_json::to_value(&request).unwrap(), value);
    }
    for value in &f.responses {
        let response: RevisionQueryResponse = serde_json::from_value(value.clone()).unwrap();
        response.validate().unwrap();
        assert_eq!(&serde_json::to_value(&response).unwrap(), value);
    }
    for case in &f.request_response_cases {
        let request = serde_json::from_value(f.requests[case.request].clone()).unwrap();
        let response: RevisionQueryResponse =
            serde_json::from_value(f.responses[case.response].clone()).unwrap();
        assert_eq!(response.validate_for(&request).is_ok(), case.valid);
    }
}

#[test]
fn boundary_rejects_inconsistent_identities_freshness_terms_and_spoofed_fields() {
    let f = fixture();
    for case in &f.invalid_requests {
        let mut value = f.requests[case.base].clone();
        replace(&mut value, &case.pointer, case.value.clone());
        let error = serde_json::from_value::<RevisionQueryRequest>(value).unwrap_err();
        assert!(
            error.to_string().contains(&case.reason),
            "{error}: {}",
            case.pointer
        );
    }
    for case in &f.invalid_responses {
        let mut value = f.responses[case.base].clone();
        replace(&mut value, &case.pointer, case.value.clone());
        let error = serde_json::from_value::<RevisionQueryResponse>(value).unwrap_err();
        assert!(
            error.to_string().contains(&case.reason),
            "{error}: {}",
            case.pointer
        );
    }
    let request = serde_json::from_value(f.requests[0].clone()).unwrap();
    for case in &f.inconsistent_pairs {
        let mut value = f.responses[case.response].clone();
        replace(&mut value, &case.pointer, case.value.clone());
        // A cross-project completed descriptor is rejected at decode already.
        if let Ok(response) = serde_json::from_value::<RevisionQueryResponse>(value) {
            assert!(response.validate_for(&request).is_err());
        }
    }
    assert!(
        serde_json::from_str::<RevisionQueryRequest>(&f.duplicate_json[0])
            .unwrap_err()
            .to_string()
            .contains("duplicate JSON key")
    );
    assert!(
        serde_json::from_str::<RevisionQueryResponse>(&f.duplicate_json[1])
            .unwrap_err()
            .to_string()
            .contains("duplicate JSON key")
    );
}

#[test]
fn wire_bounds_fail_before_publication_or_successful_query_response() {
    let f = fixture();
    let mut request: RevisionQueryRequest = serde_json::from_value(f.requests[0].clone()).unwrap();
    request.query = request
        .query
        .repeat(MAX_REVISION_QUERY_BYTES / request.query.len() + 1);
    assert!(request.validate().is_err());
    let response: RevisionQueryResponse = serde_json::from_value(f.responses[0].clone()).unwrap();
    let RevisionQueryOutcome::Completed { results, .. } = response.outcome else {
        panic!()
    };
    let RevisionQueryResults::Select { variables, rows } = results else {
        panic!()
    };
    let oversized = RevisionQueryResults::Select {
        variables: variables.clone(),
        rows: vec![rows[0].clone(); MAX_REVISION_QUERY_ROWS + 1],
    };
    assert!(oversized.validate().is_err());
    let mut oversized = rows[0].clone();
    if let Some(RdfTerm::LanguageLiteral { value, .. }) = oversized.get_mut(&variables[1]) {
        *value = value.repeat(MAX_REVISION_QUERY_RESULT_BYTES / value.len() + 1);
    }
    assert!(
        RevisionQueryResults::Select {
            variables,
            rows: vec![oversized]
        }
        .validate()
        .is_err()
    );
}
