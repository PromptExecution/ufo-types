//! "Coherence" sanity checks: do multiple values (e.g. two different
//! extraction/inference passes' results) agree with each other closely
//! enough to be trusted. Follows this crate's established `Satisfies<C>`
//! idiom exactly (see `src/sysml.rs`/`src/data_format.rs`).

use crate::satisfies::{Constraint, Satisfies, SatisfiesResult};

/// Constraint: every value in the subject slice is within `tolerance` of
/// every other value (checked via max-minus-min, which is equivalent to
/// pairwise-within-tolerance for a single scalar tolerance).
pub struct NumericAgreement {
    pub tolerance: f64,
}

impl Constraint for NumericAgreement {}

impl Satisfies<NumericAgreement> for [f64] {
    fn satisfies(&self, constraint: &NumericAgreement) -> SatisfiesResult {
        validate_numeric_agreement(self, constraint.tolerance)
    }
}

impl Satisfies<NumericAgreement> for Vec<f64> {
    fn satisfies(&self, constraint: &NumericAgreement) -> SatisfiesResult {
        self.as_slice().satisfies(constraint)
    }
}

/// Checks whether `values` agree within `tolerance` (max - min <= tolerance).
/// Fewer than 2 values can't demonstrate agreement or disagreement between
/// independent sources, so this returns `Unknown` rather than a false
/// `Satisfied`.
pub fn validate_numeric_agreement(values: &[f64], tolerance: f64) -> SatisfiesResult {
    if values.len() < 2 {
        return SatisfiesResult::unknown();
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let spread = max - min;
    if spread <= tolerance {
        SatisfiesResult::satisfied(1.0, Vec::new())
    } else {
        SatisfiesResult::violated(format!(
            "values disagree by {spread} (min {min}, max {max}), exceeding tolerance {tolerance}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_within_tolerance_are_satisfied() {
        let values = vec![100.00, 100.02, 99.99];
        let result = validate_numeric_agreement(&values, 0.05);
        assert!(
            result.disposition.is_satisfied(),
            "{:?}",
            result.disposition
        );
    }

    #[test]
    fn values_outside_tolerance_are_violated() {
        let values = vec![100.00, 105.00];
        let result = validate_numeric_agreement(&values, 0.05);
        assert!(!result.disposition.is_satisfied());
        assert!(matches!(
            result.disposition,
            crate::satisfies::Disposition::Violated { .. }
        ));
    }

    #[test]
    fn single_value_is_unknown_not_satisfied() {
        let values = vec![100.00];
        let result = validate_numeric_agreement(&values, 0.05);
        assert!(matches!(
            result.disposition,
            crate::satisfies::Disposition::Unknown
        ));
    }

    #[test]
    fn empty_values_is_unknown() {
        let values: Vec<f64> = vec![];
        let result = validate_numeric_agreement(&values, 0.05);
        assert!(matches!(
            result.disposition,
            crate::satisfies::Disposition::Unknown
        ));
    }

    #[test]
    fn satisfies_trait_usable_on_slice_and_vec() {
        let values = vec![50.0, 50.01];
        let constraint = NumericAgreement { tolerance: 0.05 };
        assert!(values.satisfies(&constraint).disposition.is_satisfied());
        assert!(
            values
                .as_slice()
                .satisfies(&constraint)
                .disposition
                .is_satisfied()
        );
    }
}
