//! Range verdicts — what a constraint-propagation solver (SysMD's
//! `cspsolver`, `tukcps/SysMD`) actually returns, as data, wired into this
//! crate's `Satisfies<C>` idiom.
//!
//! A quantity's [`Interval`](crate::quantity::Interval) is the set of values it
//! *might* take. Checking it against a bound has three outcomes, not two:
//!
//! | declared range vs. bound | [`RangeVerdict`] | [`Disposition`](crate::satisfies::Disposition) |
//! |---|---|---|
//! | entirely inside | `Entailed` — every possible value satisfies | `Satisfied` |
//! | partly inside | `Narrowed` — satisfiable, here is the over-approximation | `Unknown` |
//! | disjoint | `Inconsistent` — empty set, nothing satisfies | `Violated` |
//!
//! `Narrowed` maps to `Unknown`, never `Satisfied`: an over-approximation says
//! a satisfying value *may* exist, not that the model's value does.
//! `Disposition` stays closed and three-valued; the narrowed range travels in
//! [`RangeVerdict`] itself.

use crate::quantity::{Quantity, QuantityError};
use crate::satisfies::{Constraint, Satisfies, SatisfiesResult};

/// Outcome of checking a quantity's possible values against a bound.
#[derive(Debug, Clone, PartialEq)]
pub enum RangeVerdict {
    /// The whole declared range satisfies the bound.
    Entailed,
    /// Some values satisfy; `narrowed` is the declared range cut down to them,
    /// in the checked quantity's unit.
    Narrowed { narrowed: Quantity },
    /// No value satisfies (the intersection is empty).
    Inconsistent { reason: String },
}

impl From<RangeVerdict> for SatisfiesResult {
    fn from(v: RangeVerdict) -> SatisfiesResult {
        match v {
            RangeVerdict::Entailed => SatisfiesResult::satisfied(1.0, Vec::new()),
            RangeVerdict::Narrowed { .. } => SatisfiesResult::unknown(),
            RangeVerdict::Inconsistent { reason } => SatisfiesResult::violated(reason),
        }
    }
}

/// Constraint: the value must lie within `bound`'s range.
///
/// The bound may be in any unit of the same dimension (and currency); it is
/// converted into the checked quantity's unit before comparing.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantityBound {
    pub bound: Quantity,
}

impl Constraint for QuantityBound {}

/// Check `value`'s possible range against `bound`.
pub fn check_bound(value: &Quantity, bound: &QuantityBound) -> Result<RangeVerdict, QuantityError> {
    let bound = bound.bound.convert_to(&value.unit)?;
    if bound.range.contains_interval(&value.range) {
        return Ok(RangeVerdict::Entailed);
    }
    match value.range.intersect(&bound.range) {
        Some(range) => Ok(RangeVerdict::Narrowed {
            narrowed: Quantity::new(range, value.unit.clone()),
        }),
        None => Ok(RangeVerdict::Inconsistent {
            reason: format!(
                "value range [{}, {}] {} is disjoint from bound [{}, {}]",
                value.range.lo(),
                value.range.hi(),
                value.unit.symbol,
                bound.range.lo(),
                bound.range.hi()
            ),
        }),
    }
}

impl Satisfies<QuantityBound> for Quantity {
    fn satisfies(&self, constraint: &QuantityBound) -> SatisfiesResult {
        match check_bound(self, constraint) {
            Ok(verdict) => verdict.into(),
            // A unit/dimension/currency mismatch is a malformed check, not an unknown.
            Err(e) => SatisfiesResult::violated(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantity::{Interval, Unit};
    use crate::satisfies::Disposition;

    fn qty(lo: f64, hi: f64, unit: &str) -> Quantity {
        Quantity::new(Interval::new(lo, hi).unwrap(), Unit::lookup(unit).unwrap())
    }

    fn bound(lo: f64, hi: f64, unit: &str) -> QuantityBound {
        QuantityBound {
            bound: qty(lo, hi, unit),
        }
    }

    #[test]
    fn entailed_when_range_is_inside_bound() {
        let r = qty(2.0, 3.0, "kg").satisfies(&bound(0.0, 5.0, "kg"));
        assert_eq!(r.disposition, Disposition::Satisfied);
        assert_eq!(r.confidence, 1.0);
    }

    #[test]
    fn narrowed_is_unknown_and_carries_the_range() {
        let v = check_bound(&qty(2.0, 8.0, "kg"), &bound(0.0, 5.0, "kg")).unwrap();
        assert_eq!(
            v,
            RangeVerdict::Narrowed {
                narrowed: qty(2.0, 5.0, "kg")
            }
        );
        let r = qty(2.0, 8.0, "kg").satisfies(&bound(0.0, 5.0, "kg"));
        assert_eq!(r.disposition, Disposition::Unknown);
    }

    #[test]
    fn disjoint_is_violated() {
        let r = qty(6.0, 8.0, "kg").satisfies(&bound(0.0, 5.0, "kg"));
        assert!(r.disposition.is_violated());
    }

    #[test]
    fn bound_is_converted_into_the_values_unit() {
        // 1500 g .. 2500 g inside [0, 3] kg
        let r = qty(1500.0, 2500.0, "g").satisfies(&bound(0.0, 3.0, "kg"));
        assert_eq!(r.disposition, Disposition::Satisfied);
        let v = check_bound(&qty(1500.0, 4000.0, "g"), &bound(0.0, 3.0, "kg")).unwrap();
        match v {
            RangeVerdict::Narrowed { narrowed } => {
                assert_eq!(narrowed.unit.symbol, "g");
                assert!((narrowed.range.hi() - 3000.0).abs() < 1e-9);
            }
            other => panic!("expected Narrowed, got {other:?}"),
        }
    }

    #[test]
    fn mismatched_dimension_or_currency_is_a_violation_not_unknown() {
        let r = qty(1.0, 2.0, "kg").satisfies(&bound(0.0, 5.0, "m"));
        assert!(r.disposition.is_violated());
        let r = qty(100.0, 200.0, "USD").satisfies(&bound(0.0, 500.0, "EUR"));
        assert!(r.disposition.is_violated());
    }
}
