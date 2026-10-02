//! Dimensioned quantities over closed intervals — the concept behind SysMD's
//! constraint-propagation values (`tukcps/SysMD`, Apache-2.0), re-expressed as
//! plain data. No solver lives here; this is the value algebra a solver (or a
//! `Satisfies<C>` check, see [`crate::verdict`]) needs.
//!
//! # Model
//!
//! - [`Dimension`] — exponents over the seven ISQ base dimensions plus
//!   `information` (bit) and `money`.
//! - [`Unit`] — a dimension, a positive linear `scale` to the base unit, and
//!   an `offset` (only the temperature scales use one).
//! - [`Interval`] — a **finite**, closed `[lo, hi]`. A quantity whose value is
//!   not yet known is a wide interval; a known one has `lo == hi`.
//! - [`Quantity`] — an interval in a unit.
//!
//! # Money
//!
//! `money` is a dimension, and a currency is part of a unit's *identity*, not
//! a scale. Two currencies have no fixed ratio, so `EUR` and `USD` are the same
//! dimension but never implicitly interchangeable: [`Quantity::add`] and
//! [`Quantity::convert_to`] reject them with
//! [`QuantityError::CurrencyMismatch`]. The only way across is an explicit
//! [`ExchangeRate`], which carries its own caller-supplied `as_of` and
//! `source` (this crate never reads a clock). This is deliberately the
//! opposite of SysMD 4.3.0, where every currency canonicalizes to Euro and
//! `100 USD + 100 EUR` evaluates to `200 EUR`.
//!
//! A money amount is a *value*; who bears it (a cost-center / attribution
//! code and share) is a separate, upstream concern and is not modeled here.
//!
//! # Offset units
//!
//! `°C` and `°F` convert (via [`Quantity::convert_to`]) but refuse arithmetic:
//! `20 °C + 10 °C` is not `30 °C`. Do temperature differences in kelvin.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::iso::Currency;

/// Why a quantity operation was rejected.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum QuantityError {
    /// An interval bound was NaN or infinite.
    #[error("interval bounds must be finite, got [{lo}, {hi}]")]
    NotFinite { lo: f64, hi: f64 },
    /// `lo > hi`.
    #[error("interval lower bound {lo} exceeds upper bound {hi}")]
    Inverted { lo: f64, hi: f64 },
    /// An operation produced a non-finite bound.
    #[error("interval arithmetic overflowed to a non-finite bound")]
    Overflow,
    /// Division by an interval that contains zero.
    #[error("division by an interval containing zero: [{lo}, {hi}]")]
    DivisionByZeroInterval { lo: f64, hi: f64 },
    /// Operands (or a conversion target) have different dimensions.
    #[error("dimension mismatch: `{left}` vs `{right}`")]
    DimensionMismatch { left: String, right: String },
    /// Both sides are money, but in different currencies. Supply an [`ExchangeRate`].
    #[error("currency mismatch: {left} vs {right} (an explicit ExchangeRate is required)")]
    CurrencyMismatch { left: Currency, right: Currency },
    /// A unit with an offset (°C, °F) was used in arithmetic.
    #[error("unit `{0}` has an offset; only conversion is defined, use kelvin for arithmetic")]
    OffsetUnitArithmetic(String),
    /// `scale` must be finite and strictly positive.
    #[error("unit `{symbol}` has invalid scale {scale}")]
    InvalidScale { symbol: String, scale: f64 },
    /// A unit's `currency` must be set exactly when its `money` exponent is non-zero.
    #[error("unit `{0}`: currency must be set exactly when the money exponent is non-zero")]
    CurrencyIdentity(String),
    /// A dimension exponent left the `i8` range.
    #[error("dimension exponent overflow")]
    DimensionOverflow,
    /// An [`ExchangeRate`] was malformed or does not apply to the quantity.
    #[error("exchange rate: {0}")]
    ExchangeRate(String),
}

// ── Dimension ────────────────────────────────────────────────────────────────

/// Integer exponents over the base dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
pub struct Dimension {
    pub length: i8,
    pub mass: i8,
    pub time: i8,
    pub current: i8,
    pub temperature: i8,
    pub amount: i8,
    pub luminous: i8,
    /// Information (bit).
    pub information: i8,
    /// Money. Which currency is carried by [`Unit::currency`].
    pub money: i8,
}

impl Dimension {
    pub const DIMENSIONLESS: Dimension = Dimension::zero();
    pub const LENGTH: Dimension = Dimension {
        length: 1,
        ..Dimension::zero()
    };
    pub const MASS: Dimension = Dimension {
        mass: 1,
        ..Dimension::zero()
    };
    pub const TIME: Dimension = Dimension {
        time: 1,
        ..Dimension::zero()
    };
    pub const CURRENT: Dimension = Dimension {
        current: 1,
        ..Dimension::zero()
    };
    pub const TEMPERATURE: Dimension = Dimension {
        temperature: 1,
        ..Dimension::zero()
    };
    pub const AMOUNT: Dimension = Dimension {
        amount: 1,
        ..Dimension::zero()
    };
    pub const LUMINOUS: Dimension = Dimension {
        luminous: 1,
        ..Dimension::zero()
    };
    pub const INFORMATION: Dimension = Dimension {
        information: 1,
        ..Dimension::zero()
    };
    pub const MONEY: Dimension = Dimension {
        money: 1,
        ..Dimension::zero()
    };

    const fn zero() -> Dimension {
        Dimension {
            length: 0,
            mass: 0,
            time: 0,
            current: 0,
            temperature: 0,
            amount: 0,
            luminous: 0,
            information: 0,
            money: 0,
        }
    }

    fn exps(&self) -> [i8; 9] {
        [
            self.length,
            self.mass,
            self.time,
            self.current,
            self.temperature,
            self.amount,
            self.luminous,
            self.information,
            self.money,
        ]
    }

    fn from_exps(e: [i8; 9]) -> Dimension {
        Dimension {
            length: e[0],
            mass: e[1],
            time: e[2],
            current: e[3],
            temperature: e[4],
            amount: e[5],
            luminous: e[6],
            information: e[7],
            money: e[8],
        }
    }

    fn combine(&self, other: &Dimension, sign: i8) -> Result<Dimension, QuantityError> {
        let (a, b) = (self.exps(), other.exps());
        let mut out = [0i8; 9];
        for i in 0..9 {
            let delta = b[i]
                .checked_mul(sign)
                .ok_or(QuantityError::DimensionOverflow)?;
            out[i] = a[i]
                .checked_add(delta)
                .ok_or(QuantityError::DimensionOverflow)?;
        }
        Ok(Dimension::from_exps(out))
    }

    /// Product of dimensions (exponents add).
    pub fn mul(&self, other: &Dimension) -> Result<Dimension, QuantityError> {
        self.combine(other, 1)
    }

    /// Quotient of dimensions (exponents subtract).
    pub fn div(&self, other: &Dimension) -> Result<Dimension, QuantityError> {
        self.combine(other, -1)
    }

    /// `true` when every exponent is zero.
    pub fn is_dimensionless(&self) -> bool {
        *self == Dimension::DIMENSIONLESS
    }
}

impl std::fmt::Display for Dimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const NAMES: [&str; 9] = ["L", "M", "T", "I", "Θ", "N", "J", "bit", "$"];
        let mut parts = Vec::new();
        for (name, exp) in NAMES.iter().zip(self.exps()) {
            match exp {
                0 => {}
                1 => parts.push((*name).to_string()),
                e => parts.push(format!("{name}^{e}")),
            }
        }
        if parts.is_empty() {
            write!(f, "1")
        } else {
            write!(f, "{}", parts.join("·"))
        }
    }
}

// ── Interval ─────────────────────────────────────────────────────────────────

/// A finite, closed interval `[lo, hi]`.
///
/// Unbounded intervals are not representable on purpose: `inf - inf` is NaN and
/// a NaN bound silently poisons every later comparison.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Interval {
    lo: f64,
    hi: f64,
}

impl Interval {
    /// A validated interval.
    pub fn new(lo: f64, hi: f64) -> Result<Interval, QuantityError> {
        if !lo.is_finite() || !hi.is_finite() {
            return Err(QuantityError::NotFinite { lo, hi });
        }
        if lo > hi {
            return Err(QuantityError::Inverted { lo, hi });
        }
        Ok(Interval { lo, hi })
    }

    /// The degenerate interval `[v, v]`.
    pub fn exact(v: f64) -> Result<Interval, QuantityError> {
        Interval::new(v, v)
    }

    pub fn lo(&self) -> f64 {
        self.lo
    }

    pub fn hi(&self) -> f64 {
        self.hi
    }

    pub fn width(&self) -> f64 {
        self.hi - self.lo
    }

    /// `true` when `other` lies entirely inside `self`.
    pub fn contains_interval(&self, other: &Interval) -> bool {
        self.lo <= other.lo && other.hi <= self.hi
    }

    /// The overlap, or `None` when the intervals are disjoint.
    pub fn intersect(&self, other: &Interval) -> Option<Interval> {
        let lo = self.lo.max(other.lo);
        let hi = self.hi.min(other.hi);
        (lo <= hi).then_some(Interval { lo, hi })
    }

    pub fn add(&self, o: &Interval) -> Result<Interval, QuantityError> {
        Interval::checked(self.lo + o.lo, self.hi + o.hi)
    }

    pub fn sub(&self, o: &Interval) -> Result<Interval, QuantityError> {
        Interval::checked(self.lo - o.hi, self.hi - o.lo)
    }

    pub fn mul(&self, o: &Interval) -> Result<Interval, QuantityError> {
        let p = [
            self.lo * o.lo,
            self.lo * o.hi,
            self.hi * o.lo,
            self.hi * o.hi,
        ];
        let lo = p.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = p.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        Interval::checked(lo, hi)
    }

    pub fn div(&self, o: &Interval) -> Result<Interval, QuantityError> {
        if o.lo <= 0.0 && 0.0 <= o.hi {
            return Err(QuantityError::DivisionByZeroInterval { lo: o.lo, hi: o.hi });
        }
        let inv = Interval::checked(1.0 / o.hi, 1.0 / o.lo)?;
        self.mul(&inv)
    }

    /// Multiply by a strictly positive factor.
    fn scaled(&self, factor: f64) -> Result<Interval, QuantityError> {
        Interval::checked(self.lo * factor, self.hi * factor)
    }

    fn shifted(&self, delta: f64) -> Result<Interval, QuantityError> {
        Interval::checked(self.lo + delta, self.hi + delta)
    }

    fn checked(lo: f64, hi: f64) -> Result<Interval, QuantityError> {
        if !lo.is_finite() || !hi.is_finite() {
            return Err(QuantityError::Overflow);
        }
        Ok(Interval { lo, hi })
    }
}

// ── Unit ─────────────────────────────────────────────────────────────────────

/// A unit of measure.
///
/// `base = value * scale + offset`, where *base* is the SI base (or the
/// currency itself, or the bit).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Unit {
    pub symbol: String,
    pub dimension: Dimension,
    pub scale: f64,
    pub offset: f64,
    /// Set exactly when `dimension.money != 0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
}

impl Unit {
    /// A validated unit.
    pub fn new(
        symbol: impl Into<String>,
        dimension: Dimension,
        scale: f64,
        offset: f64,
        currency: Option<Currency>,
    ) -> Result<Unit, QuantityError> {
        let symbol = symbol.into();
        if !scale.is_finite() || scale <= 0.0 || !offset.is_finite() {
            return Err(QuantityError::InvalidScale { symbol, scale });
        }
        if currency.is_some() != (dimension.money != 0) {
            return Err(QuantityError::CurrencyIdentity(symbol));
        }
        Ok(Unit {
            symbol,
            dimension,
            scale,
            offset,
            currency,
        })
    }

    /// The dimensionless unit `1`.
    pub fn one() -> Unit {
        Unit {
            symbol: "1".into(),
            dimension: Dimension::DIMENSIONLESS,
            scale: 1.0,
            offset: 0.0,
            currency: None,
        }
    }

    /// The unit of one whole `currency` (e.g. `EUR`).
    pub fn of_currency(currency: Currency) -> Unit {
        Unit {
            symbol: currency.to_string(),
            dimension: Dimension::MONEY,
            scale: 1.0,
            offset: 0.0,
            currency: Some(currency),
        }
    }

    /// Look up a unit by symbol: a currency code (`EUR`), a table unit (`kg`,
    /// `mm`, `MHz`, `h`, `°C`) with an optional SI prefix. `None` if unknown.
    pub fn lookup(symbol: &str) -> Option<Unit> {
        if symbol == "1" {
            return Some(Unit::one());
        }
        if let Some(c) = ALL_CURRENCIES.iter().find(|c| c.to_string() == symbol) {
            return Some(Unit::of_currency(*c));
        }
        if let Some(e) = TABLE.iter().find(|e| e.symbol == symbol) {
            return Some(e.unit(symbol, 1.0));
        }
        for (prefix, factor) in PREFIXES {
            if let Some(rest) = symbol.strip_prefix(prefix) {
                if let Some(e) = TABLE.iter().find(|e| e.symbol == rest && e.prefixable) {
                    return Some(e.unit(symbol, *factor));
                }
            }
        }
        None
    }

    /// `true` for units that carry an offset (°C, °F).
    pub fn has_offset(&self) -> bool {
        self.offset != 0.0
    }

    fn combine(&self, other: &Unit, divide: bool) -> Result<Unit, QuantityError> {
        if self.has_offset() {
            return Err(QuantityError::OffsetUnitArithmetic(self.symbol.clone()));
        }
        if other.has_offset() {
            return Err(QuantityError::OffsetUnitArithmetic(other.symbol.clone()));
        }
        let dimension = if divide {
            self.dimension.div(&other.dimension)?
        } else {
            self.dimension.mul(&other.dimension)?
        };
        let currency = match (self.currency, other.currency) {
            (Some(a), Some(b)) if a != b => {
                return Err(QuantityError::CurrencyMismatch { left: a, right: b });
            }
            (Some(a), _) | (None, Some(a)) => Some(a),
            (None, None) => None,
        };
        // EUR / EUR has no money left, so it carries no currency either.
        let currency = if dimension.money == 0 { None } else { currency };
        let (symbol, scale) = if divide {
            (
                format!("{}/{}", self.symbol, other.symbol),
                self.scale / other.scale,
            )
        } else {
            (
                format!("{}·{}", self.symbol, other.symbol),
                self.scale * other.scale,
            )
        };
        Unit::new(symbol, dimension, scale, 0.0, currency)
    }

    fn same_kind(&self, other: &Unit) -> Result<(), QuantityError> {
        if self.dimension != other.dimension {
            return Err(QuantityError::DimensionMismatch {
                left: self.dimension.to_string(),
                right: other.dimension.to_string(),
            });
        }
        if let (Some(a), Some(b)) = (self.currency, other.currency) {
            if a != b {
                return Err(QuantityError::CurrencyMismatch { left: a, right: b });
            }
        }
        Ok(())
    }
}

const ALL_CURRENCIES: [Currency; 16] = [
    Currency::Aud,
    Currency::Usd,
    Currency::Eur,
    Currency::Gbp,
    Currency::Jpy,
    Currency::Cad,
    Currency::Chf,
    Currency::Nzd,
    Currency::Sgd,
    Currency::Hkd,
    Currency::Btc,
    Currency::Eth,
    Currency::Usdt,
    Currency::Usdc,
    Currency::Sol,
    Currency::Xrp,
];

const PREFIXES: &[(&str, f64)] = &[
    ("Y", 1e24),
    ("Z", 1e21),
    ("E", 1e18),
    ("P", 1e15),
    ("T", 1e12),
    ("G", 1e9),
    ("M", 1e6),
    ("k", 1e3),
    ("m", 1e-3),
    ("µ", 1e-6),
    ("u", 1e-6),
    ("n", 1e-9),
    ("p", 1e-12),
    ("f", 1e-15),
];

struct Entry {
    symbol: &'static str,
    dimension: Dimension,
    scale: f64,
    offset: f64,
    prefixable: bool,
}

impl Entry {
    fn unit(&self, symbol: &str, prefix: f64) -> Unit {
        Unit {
            symbol: symbol.to_string(),
            dimension: self.dimension,
            scale: self.scale * prefix,
            offset: self.offset,
            currency: None,
        }
    }
}

const fn dim(
    length: i8,
    mass: i8,
    time: i8,
    current: i8,
    temperature: i8,
    information: i8,
) -> Dimension {
    Dimension {
        length,
        mass,
        time,
        current,
        temperature,
        amount: 0,
        luminous: 0,
        information,
        money: 0,
    }
}

const fn entry(symbol: &'static str, dimension: Dimension, scale: f64, prefixable: bool) -> Entry {
    Entry {
        symbol,
        dimension,
        scale,
        offset: 0.0,
        prefixable,
    }
}

/// The unit table. Scales are to the SI base (`kg` is the mass base, so `g` is `1e-3`).
const TABLE: &[Entry] = &[
    entry("m", Dimension::LENGTH, 1.0, true),
    entry("g", Dimension::MASS, 1e-3, true),
    entry("s", Dimension::TIME, 1.0, true),
    entry("A", Dimension::CURRENT, 1.0, true),
    entry("K", Dimension::TEMPERATURE, 1.0, true),
    entry("mol", Dimension::AMOUNT, 1.0, true),
    entry("cd", Dimension::LUMINOUS, 1.0, true),
    entry("bit", Dimension::INFORMATION, 1.0, true),
    entry("B", Dimension::INFORMATION, 8.0, true),
    entry("Hz", dim(0, 0, -1, 0, 0, 0), 1.0, true),
    entry("N", dim(1, 1, -2, 0, 0, 0), 1.0, true),
    entry("Pa", dim(-1, 1, -2, 0, 0, 0), 1.0, true),
    entry("J", dim(2, 1, -2, 0, 0, 0), 1.0, true),
    entry("W", dim(2, 1, -3, 0, 0, 0), 1.0, true),
    entry("V", dim(2, 1, -3, -1, 0, 0), 1.0, true),
    entry("ohm", dim(2, 1, -3, -2, 0, 0), 1.0, true),
    entry("L", dim(3, 0, 0, 0, 0, 0), 1e-3, true),
    entry("min", Dimension::TIME, 60.0, false),
    entry("h", Dimension::TIME, 3600.0, false),
    entry("d", Dimension::TIME, 86_400.0, false),
    Entry {
        symbol: "°C",
        dimension: Dimension::TEMPERATURE,
        scale: 1.0,
        offset: 273.15,
        prefixable: false,
    },
    Entry {
        symbol: "°F",
        dimension: Dimension::TEMPERATURE,
        scale: 5.0 / 9.0,
        offset: 255.372_222_222_222_2,
        prefixable: false,
    },
];

// ── ExchangeRate ─────────────────────────────────────────────────────────────

/// An explicit, attributable exchange rate: `1 from = rate to`.
///
/// `as_of` and `source` are caller-supplied provenance (an ISO-8601 instant, a
/// ledger entry id, a rate-table reference …); this crate never reads a clock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExchangeRate {
    pub from: Currency,
    pub to: Currency,
    pub rate: f64,
    pub as_of: String,
    pub source: String,
}

impl ExchangeRate {
    fn validate(&self) -> Result<(), QuantityError> {
        if !self.rate.is_finite() || self.rate <= 0.0 {
            return Err(QuantityError::ExchangeRate(format!(
                "rate must be finite and positive, got {}",
                self.rate
            )));
        }
        if self.as_of.trim().is_empty() || self.source.trim().is_empty() {
            return Err(QuantityError::ExchangeRate(
                "`as_of` and `source` are required: a rate without provenance is a guess".into(),
            ));
        }
        Ok(())
    }
}

// ── Quantity ─────────────────────────────────────────────────────────────────

/// An interval of values in a [`Unit`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Quantity {
    pub range: Interval,
    pub unit: Unit,
}

impl Quantity {
    pub fn new(range: Interval, unit: Unit) -> Quantity {
        Quantity { range, unit }
    }

    /// A single known value.
    pub fn exact(value: f64, unit: Unit) -> Result<Quantity, QuantityError> {
        Ok(Quantity {
            range: Interval::exact(value)?,
            unit,
        })
    }

    /// Convert into `target`. Currencies never convert here (see [`ExchangeRate`]).
    pub fn convert_to(&self, target: &Unit) -> Result<Quantity, QuantityError> {
        self.unit.same_kind(target)?;
        let base = self
            .range
            .scaled(self.unit.scale)?
            .shifted(self.unit.offset)?;
        let range = base.shifted(-target.offset)?.scaled(1.0 / target.scale)?;
        Ok(Quantity {
            range,
            unit: target.clone(),
        })
    }

    /// Sum, expressed in `self`'s unit. Requires the same dimension and currency.
    pub fn add(&self, other: &Quantity) -> Result<Quantity, QuantityError> {
        self.additive(other, false)
    }

    /// Difference, expressed in `self`'s unit.
    pub fn sub(&self, other: &Quantity) -> Result<Quantity, QuantityError> {
        self.additive(other, true)
    }

    fn additive(&self, other: &Quantity, subtract: bool) -> Result<Quantity, QuantityError> {
        for u in [&self.unit, &other.unit] {
            if u.has_offset() {
                return Err(QuantityError::OffsetUnitArithmetic(u.symbol.clone()));
            }
        }
        self.unit.same_kind(&other.unit)?;
        let rhs = other.range.scaled(other.unit.scale / self.unit.scale)?;
        let range = if subtract {
            self.range.sub(&rhs)?
        } else {
            self.range.add(&rhs)?
        };
        Ok(Quantity {
            range,
            unit: self.unit.clone(),
        })
    }

    pub fn mul(&self, other: &Quantity) -> Result<Quantity, QuantityError> {
        Ok(Quantity {
            range: self.range.mul(&other.range)?,
            unit: self.unit.combine(&other.unit, false)?,
        })
    }

    pub fn div(&self, other: &Quantity) -> Result<Quantity, QuantityError> {
        Ok(Quantity {
            range: self.range.div(&other.range)?,
            unit: self.unit.combine(&other.unit, true)?,
        })
    }

    /// Convert a plain currency amount with an explicit rate. Compound units
    /// (`EUR/kg`) are not converted: a safe symbol rewrite needs unit structure
    /// this model deliberately does not keep.
    pub fn convert_currency(&self, rate: &ExchangeRate) -> Result<Quantity, QuantityError> {
        rate.validate()?;
        if self.unit != Unit::of_currency(rate.from) {
            return Err(QuantityError::ExchangeRate(format!(
                "rate {}→{} applies only to a plain {} amount, got unit `{}`",
                rate.from, rate.to, rate.from, self.unit.symbol
            )));
        }
        Ok(Quantity {
            range: self.range.scaled(rate.rate)?,
            unit: Unit::of_currency(rate.to),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(sym: &str) -> Unit {
        Unit::lookup(sym).unwrap_or_else(|| panic!("unknown unit {sym}"))
    }

    fn q(v: f64, sym: &str) -> Quantity {
        Quantity::exact(v, u(sym)).unwrap()
    }

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    #[test]
    fn interval_rejects_nan_inf_and_inverted() {
        assert!(matches!(
            Interval::new(f64::NAN, 1.0),
            Err(QuantityError::NotFinite { .. })
        ));
        assert!(matches!(
            Interval::new(0.0, f64::INFINITY),
            Err(QuantityError::NotFinite { .. })
        ));
        assert!(matches!(
            Interval::new(2.0, 1.0),
            Err(QuantityError::Inverted { .. })
        ));
    }

    #[test]
    fn interval_arithmetic_is_sound() {
        let a = Interval::new(-2.0, 3.0).unwrap();
        let b = Interval::new(4.0, 5.0).unwrap();
        assert_eq!(a.add(&b).unwrap(), Interval::new(2.0, 8.0).unwrap());
        assert_eq!(a.sub(&b).unwrap(), Interval::new(-7.0, -1.0).unwrap());
        assert_eq!(a.mul(&b).unwrap(), Interval::new(-10.0, 15.0).unwrap());
        assert_eq!(a.div(&b).unwrap(), Interval::new(-0.5, 0.75).unwrap());
        assert!(matches!(
            b.div(&a),
            Err(QuantityError::DivisionByZeroInterval { .. })
        ));
    }

    #[test]
    fn intersect_and_contain() {
        let a = Interval::new(0.0, 10.0).unwrap();
        let b = Interval::new(5.0, 20.0).unwrap();
        assert_eq!(a.intersect(&b), Some(Interval::new(5.0, 10.0).unwrap()));
        assert_eq!(a.intersect(&Interval::new(11.0, 12.0).unwrap()), None);
        assert!(a.contains_interval(&Interval::new(1.0, 2.0).unwrap()));
        assert!(!a.contains_interval(&b));
    }

    #[test]
    fn lookup_handles_prefixes_and_ambiguities() {
        approx(u("kg").scale, 1.0);
        approx(u("g").scale, 1e-3);
        approx(u("mm").scale, 1e-3);
        approx(u("MHz").scale, 1e6);
        approx(u("µs").scale, 1e-6);
        approx(u("min").scale, 60.0); // exact entry wins over milli-inch-ish splits
        approx(u("kB").scale, 8000.0);
        assert_eq!(
            u("N").dimension,
            Dimension {
                length: 1,
                mass: 1,
                time: -2,
                ..Dimension::default()
            }
        );
        assert!(Unit::lookup("furlong").is_none());
        assert!(Unit::lookup("kmin").is_none()); // min is not prefixable
    }

    #[test]
    fn same_dimension_converts() {
        let r = q(1.5, "km").convert_to(&u("m")).unwrap();
        approx(r.range.lo(), 1500.0);
        let r = q(2.0, "h").convert_to(&u("min")).unwrap();
        approx(r.range.hi(), 120.0);
    }

    #[test]
    fn dimension_mismatch_is_rejected() {
        assert!(matches!(
            q(1.0, "kg").convert_to(&u("m")),
            Err(QuantityError::DimensionMismatch { .. })
        ));
        assert!(matches!(
            q(1.0, "kg").add(&q(1.0, "m")),
            Err(QuantityError::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn addition_converts_rhs_into_lhs_unit() {
        let r = q(1.0, "km").add(&q(500.0, "m")).unwrap();
        assert_eq!(r.unit.symbol, "km");
        approx(r.range.lo(), 1.5);
    }

    #[test]
    fn mul_div_build_derived_units() {
        let f = q(2.0, "kg").mul(&q(3.0, "m")).unwrap();
        assert_eq!(
            f.unit.dimension,
            Dimension {
                length: 1,
                mass: 1,
                ..Dimension::default()
            }
        );
        let speed = q(100.0, "m").div(&q(10.0, "s")).unwrap();
        approx(speed.range.lo(), 10.0);
        assert_eq!(
            speed.unit.dimension,
            Dimension {
                length: 1,
                time: -1,
                ..Dimension::default()
            }
        );
        let ratio = q(5.0, "m").div(&q(5.0, "m")).unwrap();
        assert!(ratio.unit.dimension.is_dimensionless());
    }

    #[test]
    fn temperature_converts_but_refuses_arithmetic() {
        let k = q(20.0, "°C").convert_to(&u("K")).unwrap();
        approx(k.range.lo(), 293.15);
        let c = q(293.15, "K").convert_to(&u("°C")).unwrap();
        approx(c.range.lo(), 20.0);
        let f = q(212.0, "°F").convert_to(&u("°C")).unwrap();
        approx(f.range.lo(), 100.0);
        assert!(matches!(
            q(20.0, "°C").add(&q(10.0, "°C")),
            Err(QuantityError::OffsetUnitArithmetic(_))
        ));
        assert!(matches!(
            q(20.0, "°C").mul(&q(2.0, "1")),
            Err(QuantityError::OffsetUnitArithmetic(_))
        ));
    }

    // ── money: the SysMD 4.3.0 hazard, pinned ───────────────────────────────

    #[test]
    fn same_currency_adds() {
        let r = q(100.0, "USD").add(&q(50.0, "USD")).unwrap();
        approx(r.range.lo(), 150.0);
        assert_eq!(r.unit.symbol, "USD");
    }

    #[test]
    fn different_currencies_never_add_or_convert_implicitly() {
        assert!(matches!(
            q(100.0, "USD").add(&q(100.0, "EUR")),
            Err(QuantityError::CurrencyMismatch {
                left: Currency::Usd,
                right: Currency::Eur
            })
        ));
        assert!(matches!(
            q(1.0, "GBP").convert_to(&u("USD")),
            Err(QuantityError::CurrencyMismatch { .. })
        ));
    }

    #[test]
    fn currency_cannot_be_multiplied_across_currencies() {
        assert!(matches!(
            q(2.0, "USD").mul(&q(3.0, "EUR")),
            Err(QuantityError::CurrencyMismatch { .. })
        ));
    }

    #[test]
    fn cost_per_mass_keeps_its_currency_and_cancels_when_divided_out() {
        let per_kg = q(12.0, "EUR").div(&q(4.0, "kg")).unwrap();
        assert_eq!(per_kg.unit.currency, Some(Currency::Eur));
        approx(per_kg.range.lo(), 3.0);
        let total = per_kg.mul(&q(10.0, "kg")).unwrap();
        assert_eq!(total.unit.currency, Some(Currency::Eur));
        approx(total.range.lo(), 30.0);
        let ratio = q(10.0, "EUR").div(&q(5.0, "EUR")).unwrap();
        assert_eq!(ratio.unit.currency, None);
        assert!(ratio.unit.dimension.is_dimensionless());
    }

    #[test]
    fn explicit_exchange_rate_is_the_only_way_across() {
        let rate = ExchangeRate {
            from: Currency::Usd,
            to: Currency::Eur,
            rate: 0.9,
            as_of: "2026-10-02T00:00:00Z".into(),
            source: "ecb:ref-rate".into(),
        };
        let eur = q(100.0, "USD").convert_currency(&rate).unwrap();
        assert_eq!(eur.unit.symbol, "EUR");
        approx(eur.range.lo(), 90.0);

        let no_provenance = ExchangeRate {
            source: " ".into(),
            ..rate.clone()
        };
        assert!(matches!(
            q(100.0, "USD").convert_currency(&no_provenance),
            Err(QuantityError::ExchangeRate(_))
        ));
        assert!(matches!(
            q(100.0, "EUR").convert_currency(&rate),
            Err(QuantityError::ExchangeRate(_))
        ));
        let per_kg = q(12.0, "USD").div(&q(1.0, "kg")).unwrap();
        assert!(matches!(
            per_kg.convert_currency(&rate),
            Err(QuantityError::ExchangeRate(_))
        ));
    }

    #[test]
    fn unit_new_enforces_currency_identity() {
        assert!(matches!(
            Unit::new("x", Dimension::MONEY, 1.0, 0.0, None),
            Err(QuantityError::CurrencyIdentity(_))
        ));
        assert!(matches!(
            Unit::new("x", Dimension::LENGTH, 1.0, 0.0, Some(Currency::Eur)),
            Err(QuantityError::CurrencyIdentity(_))
        ));
        assert!(matches!(
            Unit::new("x", Dimension::LENGTH, 0.0, 0.0, None),
            Err(QuantityError::InvalidScale { .. })
        ));
    }

    #[test]
    fn serde_round_trip() {
        let original = q(12.0, "EUR").div(&q(4.0, "kg")).unwrap();
        let json = serde_json::to_string(&original).unwrap();
        let back: Quantity = serde_json::from_str(&json).unwrap();
        assert_eq!(back, original);
    }
}
