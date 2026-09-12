//! Fixed-point ratios (A5). `Fixed` is an `i64` scaled by 1/1000, so `1.5` is `1500`.
//!
//! Arithmetic is total and deterministic: operators and `mul` saturate at the representable
//! range instead of wrapping or panicking, and `div` refuses a zero divisor. Every rounding
//! step truncates toward zero, like Rust integer division, unless the method says otherwise.

use core::fmt;
use core::ops::{Add, AddAssign, Neg, Sub, SubAssign};
use serde::{Deserialize, Serialize};

/// A ratio with three decimal places, stored as thousandths.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Fixed(i64);

/// Thousandths per unit.
const SCALE: i64 = 1000;
const SCALE_WIDE: i128 = SCALE as i128;

impl Fixed {
    /// Zero.
    pub const ZERO: Fixed = Fixed(0);
    /// One.
    pub const ONE: Fixed = Fixed(SCALE);
    /// One half.
    pub const HALF: Fixed = Fixed(SCALE / 2);
    /// The largest value.
    pub const MAX: Fixed = Fixed(i64::MAX);
    /// The smallest value.
    pub const MIN: Fixed = Fixed(i64::MIN);

    /// From a whole number, saturating.
    #[must_use]
    pub const fn from_int(n: i64) -> Fixed {
        Fixed(n.saturating_mul(SCALE))
    }

    /// From a count of thousandths.
    #[must_use]
    pub const fn from_millis(millis: i64) -> Fixed {
        Fixed(millis)
    }

    /// `num / den`, truncated toward zero. `None` when `den` is zero.
    #[must_use]
    pub const fn from_ratio(num: i64, den: i64) -> Option<Fixed> {
        if den == 0 {
            return None;
        }
        let wide = (num as i128) * SCALE_WIDE / (den as i128);
        Some(Fixed(clamp(wide)))
    }

    /// The thousandths.
    #[must_use]
    pub const fn millis(self) -> i64 {
        self.0
    }

    /// The whole part, truncated toward zero.
    #[must_use]
    pub const fn to_int(self) -> i64 {
        self.0 / SCALE
    }

    /// The largest whole number not above this value.
    #[must_use]
    pub const fn floor(self) -> i64 {
        self.0.div_euclid(SCALE)
    }

    /// The smallest whole number not below this value.
    #[must_use]
    pub const fn ceil(self) -> i64 {
        let wide = -((-(self.0 as i128)).div_euclid(SCALE_WIDE));
        clamp(wide)
    }

    /// The nearest whole number, halves rounding away from zero.
    #[must_use]
    pub const fn round(self) -> i64 {
        let half = if self.0 < 0 { -(SCALE / 2) } else { SCALE / 2 };
        clamp((self.0 as i128 + half as i128) / SCALE_WIDE)
    }

    /// Product, saturating.
    #[must_use]
    pub const fn mul(self, rhs: Fixed) -> Fixed {
        Fixed(clamp((self.0 as i128) * (rhs.0 as i128) / SCALE_WIDE))
    }

    /// Quotient, truncated toward zero and saturating. `None` when `rhs` is zero.
    #[must_use]
    pub const fn div(self, rhs: Fixed) -> Option<Fixed> {
        if rhs.0 == 0 {
            return None;
        }
        Some(Fixed(clamp(
            (self.0 as i128) * SCALE_WIDE / (rhs.0 as i128),
        )))
    }

    /// Product with a whole number, saturating.
    #[must_use]
    pub const fn mul_int(self, n: i64) -> Fixed {
        Fixed(self.0.saturating_mul(n))
    }

    /// This value scaled by `percent / 100`, truncated toward zero.
    #[must_use]
    pub const fn percent(self, percent: i64) -> Fixed {
        Fixed(clamp((self.0 as i128) * (percent as i128) / 100))
    }

    /// Sum, or `None` on overflow.
    #[must_use]
    pub const fn checked_add(self, rhs: Fixed) -> Option<Fixed> {
        match self.0.checked_add(rhs.0) {
            Some(v) => Some(Fixed(v)),
            None => None,
        }
    }

    /// Product, or `None` on overflow.
    #[must_use]
    pub const fn checked_mul(self, rhs: Fixed) -> Option<Fixed> {
        let wide = (self.0 as i128) * (rhs.0 as i128) / SCALE_WIDE;
        if wide > i64::MAX as i128 || wide < i64::MIN as i128 {
            None
        } else {
            Some(Fixed(wide as i64))
        }
    }

    /// The absolute value, saturating at `MAX` for `MIN`.
    #[must_use]
    pub const fn abs(self) -> Fixed {
        Fixed(self.0.saturating_abs())
    }

    /// The smaller of two values.
    #[must_use]
    pub const fn min(self, other: Fixed) -> Fixed {
        if self.0 <= other.0 { self } else { other }
    }

    /// The larger of two values.
    #[must_use]
    pub const fn max(self, other: Fixed) -> Fixed {
        if self.0 >= other.0 { self } else { other }
    }

    /// This value limited to `lo..=hi`.
    #[must_use]
    pub const fn clamp(self, lo: Fixed, hi: Fixed) -> Fixed {
        self.max(lo).min(hi)
    }
}

const fn clamp(wide: i128) -> i64 {
    if wide > i64::MAX as i128 {
        i64::MAX
    } else if wide < i64::MIN as i128 {
        i64::MIN
    } else {
        wide as i64
    }
}

impl Add for Fixed {
    type Output = Fixed;
    fn add(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Fixed {
    type Output = Fixed;
    fn sub(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.saturating_sub(rhs.0))
    }
}

impl Neg for Fixed {
    type Output = Fixed;
    fn neg(self) -> Fixed {
        Fixed(self.0.saturating_neg())
    }
}

impl AddAssign for Fixed {
    fn add_assign(&mut self, rhs: Fixed) {
        *self = *self + rhs;
    }
}

impl SubAssign for Fixed {
    fn sub_assign(&mut self, rhs: Fixed) {
        *self = *self - rhs;
    }
}

impl From<i64> for Fixed {
    fn from(n: i64) -> Fixed {
        Fixed::from_int(n)
    }
}

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let magnitude = (self.0 as i128).abs();
        write!(
            f,
            "{sign}{}.{:03}",
            magnitude / SCALE_WIDE,
            magnitude % SCALE_WIDE
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn ratio_truncates_toward_zero() {
        assert_eq!(Fixed::from_ratio(1, 3), Some(Fixed(333)));
        assert_eq!(Fixed::from_ratio(-1, 3), Some(Fixed(-333)));
        assert_eq!(Fixed::from_ratio(2, 3), Some(Fixed(666)));
        assert_eq!(Fixed::from_ratio(1, 0), None);
    }

    #[test]
    fn products_and_quotients() {
        assert_eq!(Fixed::HALF.mul(Fixed::HALF), Fixed(250));
        assert_eq!(Fixed::from_int(-3).mul(Fixed::HALF), Fixed(-1500));
        assert_eq!(Fixed::ONE.div(Fixed::from_int(3)), Some(Fixed(333)));
        assert_eq!(Fixed::ONE.div(Fixed::ZERO), None);
        assert_eq!(Fixed::from_int(7).percent(15), Fixed(1050));
        assert_eq!(Fixed::from_int(-7).percent(15), Fixed(-1050));
    }

    #[test]
    fn rounding_modes() {
        let cases: [(i64, i64, i64, i64, i64); 6] = [
            // millis, to_int, floor, ceil, round
            (1500, 1, 1, 2, 2),
            (-1500, -1, -2, -1, -2),
            (1499, 1, 1, 2, 1),
            (-1499, -1, -2, -1, -1),
            (2000, 2, 2, 2, 2),
            (0, 0, 0, 0, 0),
        ];
        for (m, t, fl, ce, ro) in cases {
            let v = Fixed::from_millis(m);
            assert_eq!(
                (v.to_int(), v.floor(), v.ceil(), v.round()),
                (t, fl, ce, ro),
                "{m}"
            );
        }
    }

    #[test]
    fn saturates_instead_of_wrapping() {
        assert_eq!(Fixed::MAX + Fixed::ONE, Fixed::MAX);
        assert_eq!(Fixed::MIN - Fixed::ONE, Fixed::MIN);
        assert_eq!(-Fixed::MIN, Fixed::MAX);
        assert_eq!(Fixed::MAX.mul(Fixed::from_int(2)), Fixed::MAX);
        assert_eq!(Fixed::MIN.mul(Fixed::from_int(2)), Fixed::MIN);
        assert_eq!(Fixed::MAX.checked_mul(Fixed::from_int(2)), None);
        assert_eq!(Fixed::MAX.checked_add(Fixed::ONE), None);
        assert_eq!(Fixed::MIN.abs(), Fixed::MAX);
        assert_eq!(Fixed::MIN.ceil(), i64::MIN / SCALE);
        assert_eq!(Fixed::MAX.round(), i64::MAX / SCALE + 1, "MAX ends in .807");
        assert_eq!(Fixed::from_int(i64::MAX), Fixed::MAX);
        assert_eq!(Fixed::from_ratio(i64::MAX, 1), Some(Fixed::MAX));
    }

    #[test]
    fn display_keeps_sign_and_padding() {
        assert_eq!(format!("{}", Fixed(1500)), "1.500");
        assert_eq!(format!("{}", Fixed(-500)), "-0.500");
        assert_eq!(format!("{}", Fixed(5)), "0.005");
        assert_eq!(format!("{}", Fixed::MIN), "-9223372036854775.808");
    }
}
