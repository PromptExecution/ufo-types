//! Generic multi-model proposer/reviewer orchestration — generalized from
//! `ledgrrr`'s `ledger-core::verify` module (rule-repair-specific), per this
//! crate's established consolidation precedent: independently-duplicated
//! logic gets reconciled onto one canonical shape here, the same way
//! `Satisfies`/`Stereotyped` were (see this crate's top-level doc comment).
//!
//! This module deliberately does NOT build prompts — it only orchestrates
//! "call a model, extract structured JSON, check a confidence threshold."
//! Prompt construction and domain-specific payload types (what "propose" and
//! "review" mean for a given problem) are the caller's responsibility.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Trait for model invocation. Implementors substitute different LLM
/// providers, or a mock for testing.
pub trait ModelClient: Send + Sync {
    /// Generate a raw text completion from the model.
    fn complete(&self, prompt: &str, max_tokens: usize) -> anyhow::Result<String>;

    /// Extract structured output (JSON) from the model's response to `prompt`.
    fn extract<T: DeserializeOwned>(&self, prompt: &str) -> anyhow::Result<T>;
}

/// Mock model client for testing — always returns a fixed response.
pub struct MockModelClient {
    response: String,
}

impl MockModelClient {
    pub fn with_response(mut self, response: impl Into<String>) -> Self {
        self.response = response.into();
        self
    }
}

impl Default for MockModelClient {
    fn default() -> Self {
        Self {
            response: "mock response".to_string(),
        }
    }
}

impl ModelClient for MockModelClient {
    fn complete(&self, _prompt: &str, _max_tokens: usize) -> anyhow::Result<String> {
        Ok(self.response.clone())
    }

    fn extract<T: DeserializeOwned>(&self, _prompt: &str) -> anyhow::Result<T> {
        serde_json::from_str(&self.response).map_err(|e| anyhow::anyhow!(e))
    }
}

/// Configuration for a proposer/reviewer pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiModelConfig {
    pub proposer_model: String,
    pub reviewer_model: String,
    pub min_reviewer_confidence: f32,
}

impl Default for MultiModelConfig {
    fn default() -> Self {
        Self {
            proposer_model: "claude-sonnet-4-5".to_string(),
            reviewer_model: "claude-haiku-4-5".to_string(),
            min_reviewer_confidence: 0.80,
        }
    }
}

impl MultiModelConfig {
    pub fn new(proposer: impl Into<String>, reviewer: impl Into<String>) -> Self {
        Self {
            proposer_model: proposer.into(),
            reviewer_model: reviewer.into(),
            min_reviewer_confidence: 0.80,
        }
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.min_reviewer_confidence = threshold;
        self
    }
}

/// Orchestrates a proposer model and a reviewer model against a shared
/// confidence threshold. Does not know or care what "proposal" and "review"
/// mean domain-wise — the caller builds prompts and supplies payload types.
pub struct MultiModelVerifier<C: ModelClient> {
    proposer: C,
    reviewer: C,
    config: MultiModelConfig,
}

impl<C: ModelClient> MultiModelVerifier<C> {
    pub fn new(proposer: C, reviewer: C, config: MultiModelConfig) -> Self {
        Self {
            proposer,
            reviewer,
            config,
        }
    }

    pub fn config(&self) -> &MultiModelConfig {
        &self.config
    }

    /// Ask the proposer model to extract a structured proposal from `prompt`.
    pub fn propose<P: DeserializeOwned>(&self, prompt: &str) -> anyhow::Result<P> {
        self.proposer.extract::<P>(prompt)
    }

    /// Ask the reviewer model to extract a structured review from `prompt`,
    /// with no threshold logic applied — the caller decides what a "review"
    /// type's confidence field means and how to react to it, using
    /// `meets_confidence_threshold` below.
    pub fn review_raw<R: DeserializeOwned>(&self, prompt: &str) -> anyhow::Result<R> {
        self.reviewer.extract::<R>(prompt)
    }

    /// Whether a reported confidence value clears this verifier's configured
    /// threshold.
    pub fn meets_confidence_threshold(&self, confidence: f32) -> bool {
        confidence >= self.config.min_reviewer_confidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestProposal {
        value: String,
        confidence: f32,
    }

    #[test]
    fn mock_client_extracts_arbitrary_struct_type() {
        let json = r#"{"value":"hello","confidence":0.75}"#;
        let mock = MockModelClient::default().with_response(json);
        let result: TestProposal = mock.extract("any prompt").unwrap();
        assert_eq!(
            result,
            TestProposal {
                value: "hello".to_string(),
                confidence: 0.75
            }
        );
    }

    #[test]
    fn config_defaults_match_expected_values() {
        let config = MultiModelConfig::default();
        assert_eq!(config.proposer_model, "claude-sonnet-4-5");
        assert_eq!(config.reviewer_model, "claude-haiku-4-5");
        assert_eq!(config.min_reviewer_confidence, 0.80);
    }

    #[test]
    fn meets_confidence_threshold_checks_correctly() {
        let config = MultiModelConfig::default().with_threshold(0.80);
        let verifier = MultiModelVerifier::new(
            MockModelClient::default(),
            MockModelClient::default(),
            config,
        );
        assert!(verifier.meets_confidence_threshold(0.80));
        assert!(verifier.meets_confidence_threshold(0.95));
        assert!(!verifier.meets_confidence_threshold(0.79));
    }

    #[test]
    fn propose_and_review_raw_use_the_correct_client() {
        let proposer_json = r#"{"value":"proposed","confidence":0.9}"#;
        let reviewer_json = r#"{"value":"reviewed","confidence":0.6}"#;
        let verifier = MultiModelVerifier::new(
            MockModelClient::default().with_response(proposer_json),
            MockModelClient::default().with_response(reviewer_json),
            MultiModelConfig::default(),
        );

        let proposal: TestProposal = verifier.propose("propose prompt").unwrap();
        assert_eq!(proposal.value, "proposed");

        let review: TestProposal = verifier.review_raw("review prompt").unwrap();
        assert_eq!(review.value, "reviewed");
    }
}
