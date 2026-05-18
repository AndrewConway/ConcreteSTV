// Copyright 2021-2026 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.



//! A fixed precision decimal type for jurisdictions like ACT who count votes to a particular number of decimal places.

use std::ops::{AddAssign, SubAssign, Sub, Add, Div, Mul};
use num::{Zero, BigRational, BigInt, ToPrimitive};
use std::fmt::{Debug, Display, Formatter};
use std::iter::Sum;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use std::str::FromStr;
use crate::ballot_pile::BallotPaperCount;
use crate::official_dop_transcript::CanConvertToF64PossiblyLossily;
use crate::preference_distribution::{KeepValue, RoundUpToUsize};

/// Stores a fixed precision decimal number as an integer scaled by 10^DIGITS
#[derive(Copy, Clone,Eq, PartialEq,Ord, PartialOrd,Hash)]
pub struct FixedPrecisionDecimal<const DIGITS:usize> {
    scaled_value : u64
}

impl <const DIGITS:usize> FixedPrecisionDecimal<DIGITS> {
    /// The scale that scaled_value has been multiplied by.
    pub const SCALE : u64 = {
        let mut res : u64 = 1;
        let mut togo = DIGITS;
        while togo > 0 {
            res*=10;
            togo-=1;
        }
        res
    };
    pub const MAX : u64 = u64::MAX/Self::SCALE;

    /// return SCALE*the value this number represents.
    pub fn get_scaled_value(&self) -> u64 { self.scaled_value }
    /// return the value representing scaled_value/SCALE.
    pub fn from_scaled_value(scaled_value:u64) -> Self { FixedPrecisionDecimal{scaled_value}}

    pub fn round_down(&self) -> Self { FixedPrecisionDecimal{scaled_value:Self::SCALE * (self.scaled_value/Self::SCALE)} }

    pub fn to_rational(&self) -> BigRational { BigRational::new(BigInt::from(self.scaled_value),BigInt::from(Self::SCALE)) }
    pub fn from_rational_rounding_down(rational:BigRational) -> Self { FixedPrecisionDecimal{scaled_value: ((rational.numer().clone()*BigInt::from(Self::SCALE))/rational.denom()).to_u64().unwrap()} }

    /// multiply two fixed precision values, rounding up if not exact.
    pub fn multiply_rounding_up(self,other:Self) -> Self {
        let s1 = (self.scaled_value as u128)*(other.scaled_value as u128);
        let mut rounded_down = s1/(Self::SCALE as u128);
        if rounded_down * (Self::SCALE as u128) != s1 { rounded_down+=1; }
        Self{scaled_value:rounded_down as u64}
    }
    /// divide by a fixed precision value, rounding up if not exact.
    pub fn divide_rounding_up(self,other:Self) -> Self {
        let s1 = (self.scaled_value as u128)*(Self::SCALE as u128);
        let mut rounded_down = s1/(other.scaled_value as u128);
        if rounded_down * (other.scaled_value as u128) != s1 { rounded_down+=1; }
        Self{scaled_value:rounded_down as u64}
    }
}

impl <const DIGITS:usize> From<FixedPrecisionDecimal<DIGITS>> for f64 {
    fn from(v: FixedPrecisionDecimal<DIGITS>) -> Self {
        v.scaled_value as f64/(FixedPrecisionDecimal::<DIGITS>::SCALE as f64)
    }
}
impl <const DIGITS:usize> CanConvertToF64PossiblyLossily for FixedPrecisionDecimal<DIGITS> {
    fn convert_to_f64(&self) -> f64 {
        self.scaled_value as f64/(FixedPrecisionDecimal::<DIGITS>::SCALE as f64)
    }
}

impl <const DIGITS:usize> AddAssign for FixedPrecisionDecimal<DIGITS> {
    fn add_assign(&mut self, rhs: Self) {
        self.scaled_value+=rhs.scaled_value
    }
}
impl <const DIGITS:usize> SubAssign for FixedPrecisionDecimal<DIGITS> {
    fn sub_assign(&mut self, rhs: Self) {
        self.scaled_value-=rhs.scaled_value
    }
}

impl <const DIGITS:usize> From<BallotPaperCount> for FixedPrecisionDecimal<DIGITS> {
    fn from(v: BallotPaperCount) -> Self {
        if v.0 as u64>Self::MAX { panic!("Can only represent integers up to {}, and {} was too big.",Self::MAX,v)}
        FixedPrecisionDecimal{scaled_value:v.0 as u64*Self::SCALE}
    }
}

impl <const DIGITS:usize> Zero for FixedPrecisionDecimal<DIGITS> {
    fn zero() -> Self { FixedPrecisionDecimal{scaled_value:0} }
    fn is_zero(&self) -> bool { self.scaled_value==0 }
}

impl <const DIGITS:usize> Display for FixedPrecisionDecimal<DIGITS> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let int_portion = self.scaled_value/Self::SCALE;
        let frac_portion = self.scaled_value%Self::SCALE;
        if frac_portion==0 { write!(f,"{}",int_portion)}
        else {
            let decimal_digits : String = format!("{:01$}",frac_portion,DIGITS);
            write!(f,"{}.{}",int_portion,decimal_digits.trim_end_matches("0"))
        }
    }
}

impl <const DIGITS:usize> Debug for FixedPrecisionDecimal<DIGITS> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self)
    }
}

impl <const DIGITS:usize> Sub for FixedPrecisionDecimal<DIGITS> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output { FixedPrecisionDecimal{scaled_value:self.scaled_value-rhs.scaled_value} }
}

impl <const DIGITS:usize> Add for FixedPrecisionDecimal<DIGITS> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output { FixedPrecisionDecimal{scaled_value:self.scaled_value+rhs.scaled_value} }
}
impl <const DIGITS:usize> Sum for FixedPrecisionDecimal<DIGITS> {
    fn sum<I: Iterator<Item=Self>>(iter: I) -> Self {
        let mut res = Zero::zero();
        for v in iter {
            res+=v
        }
        res
    }
}

impl <'a,const DIGITS:usize> Sum<&'a Self> for FixedPrecisionDecimal<DIGITS> {
    fn sum<I: Iterator<Item=&'a Self>>(iter: I) -> Self {
        let mut res : Self = Zero::zero();
        for v in iter {
            res+=*v
        }
        res
    }
}

impl <const DIGITS:usize> FromStr for FixedPrecisionDecimal<DIGITS> {
    type Err = <u64 as FromStr>::Err;

    fn from_str(buf: &str) -> Result<Self, Self::Err> {
        if let Some((int_part,frac_part)) = buf.split_once('.') {
            let int_part : u64 = int_part.parse()?;
            let mut frac_part_u64 : u64 = frac_part.parse()?;
            for _ in frac_part.len()..DIGITS { frac_part_u64*=10; }
            Ok(FixedPrecisionDecimal{ scaled_value: frac_part_u64+Self::SCALE*int_part })
        } else {
            let int_part : u64 = buf.parse()?;
            Ok(FixedPrecisionDecimal{ scaled_value: Self::SCALE*int_part })
        }
    }
}

impl <const DIGITS:usize> Serialize for FixedPrecisionDecimal<DIGITS> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

impl <'de,const DIGITS:usize> Deserialize<'de> for FixedPrecisionDecimal<DIGITS> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let buf = String::deserialize(deserializer)?;
        buf.parse().map_err(serde::de::Error::custom)
    }
}

impl <const DIGITS:usize> RoundUpToUsize for FixedPrecisionDecimal<DIGITS> {
    fn ceil(&self) -> usize {
        ((self.scaled_value+FixedPrecisionDecimal::<DIGITS>::SCALE-1)/FixedPrecisionDecimal::<DIGITS>::SCALE) as usize
    }
}

impl <const DIGITS:usize> Div<usize> for FixedPrecisionDecimal<DIGITS> {
    type Output = Self;

    fn div(self, rhs: usize) -> Self::Output {
        FixedPrecisionDecimal{scaled_value: self.scaled_value/(rhs as u64)}
    }
}
impl <const DIGITS:usize> Mul<usize> for FixedPrecisionDecimal<DIGITS> {
    type Output = Self;

    fn mul(self, rhs: usize) -> Self::Output {
        FixedPrecisionDecimal{scaled_value: self.scaled_value*(rhs as u64)}
    }
}

/// Implement the New Zealand Keep Value rules (generalizing the number of digits)
impl <const DIGITS:usize> KeepValue<FixedPrecisionDecimal<DIGITS>> for FixedPrecisionDecimal<DIGITS> {
    /// Clause 17, New Zealand Local Electoral Regulation 2001, Schedule 1A,
    /// "New Zealand method of counting single transferable votes" as inserted 1 Jan 2004
    /// ```text
    /// Use the following formula instead of the formula in clause 7:
    ///   kc = q × kc ÷ vc
    /// where—
    ///   kc is the keep value for the successful candidate
    ///   q is the quota for this step
    ///   vc is the candidate’s votes
    /// and the product of q and kc is truncated to 9 digits after the point and rounded up if not exact before being divided by vc and kc is also truncated to 9 digits after the point and rounded up if not exact.
    /// ```
    fn compute_keep_value(quota: FixedPrecisionDecimal<DIGITS>, tally: FixedPrecisionDecimal<DIGITS>, existing_keep_value: Self) -> Self {
        quota.multiply_rounding_up(existing_keep_value).divide_rounding_up(tally)
    }

    fn zero() -> Self { Self{scaled_value:0} }
    fn one() -> Self { Self{scaled_value:Self::SCALE} }

    fn multiply_rounding_up(self, tally: FixedPrecisionDecimal<DIGITS>) -> FixedPrecisionDecimal<DIGITS> {
        FixedPrecisionDecimal::multiply_rounding_up(self,tally)
    }

    fn multiply_one_minus_self_rounding_up(self, tally: FixedPrecisionDecimal<DIGITS>) -> FixedPrecisionDecimal<DIGITS> {
        FixedPrecisionDecimal::multiply_rounding_up(Self::one()-self,tally)
    }

    fn multiply_papers(tally: FixedPrecisionDecimal<DIGITS>, count: BallotPaperCount) -> FixedPrecisionDecimal<DIGITS> { tally*count.0 }
    fn is_one(&self) -> bool { self.scaled_value == Self::SCALE }
}
#[cfg(test)]
mod tests {
    use crate::fixed_precision_decimal::FixedPrecisionDecimal;
    use num::Zero;
    use crate::ballot_pile::BallotPaperCount;

    #[test]
    fn test_six_digit_decimal() {
        type SixDigitDecimal = FixedPrecisionDecimal<6>;
        assert!(SixDigitDecimal::zero().is_zero());

        let mut d_42 : SixDigitDecimal = BallotPaperCount(42).into();
        assert_eq!("42",format!("{}",d_42));
        d_42+=SixDigitDecimal::zero();
        assert_eq!("42",format!("{}",d_42));
        let d_1 : SixDigitDecimal = BallotPaperCount(1).into();
        assert_eq!("43",format!("{}",d_42+d_1));
        assert_eq!("41",format!("{}",d_42-d_1));
        let sum : SixDigitDecimal = [d_42,d_1].iter().sum();
        assert_eq!("43",format!("{}",sum));
        d_42+=d_1;
        assert_eq!("43",format!("{}",d_42));
        d_42-=d_1;
        assert_eq!("42",format!("{}",d_42));
        assert_eq!("21",format!("{}",d_42/2));
        let parsed : SixDigitDecimal = "45.25".parse().unwrap();
        assert_eq!("45.25",format!("{}",parsed));

    }
}
