// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use std::fmt::{Display, Formatter};
use num::Zero;
use std::ops::{Add, Sub, SubAssign, AddAssign};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use std::str::FromStr;
use std::cmp::Ordering;

/// A signed version of some type of number. Used when a number is almost always positive, but in some rare situations may be negative, like votes lost to rounding if rounding isn't always down.
#[derive(Clone,Eq, PartialEq,Debug)]
pub struct SignedVersion<Tally> {
    pub negative : bool, // should always be false for zero.
    pub value : Tally
}

impl <Tally:Clone+PartialEq> SignedVersion<Tally> {
    /// extract the value as a positive value; panic if negative.
    pub fn assume_positive(&self) -> Tally {
        if self.negative { panic!("The value was assumed positive, but wasn't")}
        self.value.clone()
    }
    /// convert into a f64 given a function to f64 for Tally.
    pub fn convert_f64<F:Fn(Tally)->f64>(&self,f:F) -> f64 {
        let res = f(self.value.clone());
        if self.negative { -res} else { res }
    }
}

impl <Tally:ToString> Display for SignedVersion<Tally> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.negative { write!(f,"-")? }
        write!(f,"{}",self.value.to_string())
    }
}

impl <Tally:Clone+Ord+Zero+Add<Output=Tally>+Sub<Output=Tally>> Zero for SignedVersion<Tally> {
    fn zero() -> Self {
        SignedVersion{ negative: false, value: Tally::zero() }
    }

    fn is_zero(&self) -> bool { self.value.is_zero() }
}
impl <Tally:Clone+PartialEq+Default> Default for SignedVersion<Tally> {
    fn default() -> Self {
        SignedVersion{ negative: false, value: Tally::default() }
    }
}

impl <Tally:Clone+Ord+Zero+Add<Output=Tally>+Sub<Output=Tally>> Add for SignedVersion<Tally> {
    type Output = SignedVersion<Tally>;
    fn add(self, rhs: Self) -> Self::Output {
        if self.negative==rhs.negative { SignedVersion{ negative: self.negative, value: self.value+rhs.value } }
        else {
            if self.value==rhs.value { Self::zero() }
            else if self.value>rhs.value { SignedVersion{ negative: self.negative, value: self.value-rhs.value } }
            else { SignedVersion{ negative: rhs.negative, value: rhs.value-self.value } }
        }
    }
}

impl <Tally:Clone+Ord+Zero+Add<Output=Tally>+Sub<Output=Tally>> Sub for SignedVersion<Tally> {
    type Output = SignedVersion<Tally>;
    fn sub(self, rhs: Self) -> Self::Output {
        if self.negative!=rhs.negative { SignedVersion{ negative: self.negative, value: self.value+rhs.value } }
        else {
            if self.value==rhs.value { Self::zero() }
            else if self.value>rhs.value { SignedVersion{ negative: self.negative, value: self.value-rhs.value } }
            else { SignedVersion{ negative: !rhs.negative, value: rhs.value-self.value } }
        }
    }
}
/*
impl <Tally:Clone+Ord+Zero+Add<Output=Tally>+Sub<Output=Tally>> From<Tally> for SignedVersion<Tally> {
    fn from(value: Tally) -> Self {
        SignedVersion{negative:false,value}
    }
}*/
impl <Tally:Clone+AddAssign+SubAssign+Ord+Zero+Add<Output=Tally>+Sub<Output=Tally>> AddAssign<Tally> for SignedVersion<Tally> {
    fn add_assign(&mut self, rhs: Tally) {
        if self.negative {
            if self.value==rhs { self.negative=false; self.value=Tally::zero(); }
            else if self.value>rhs { self.value-=rhs; }
            else { self.negative=false; self.value=rhs-self.value.clone();}
        } else { self.value+=rhs; }
    }
}


impl <Tally:Clone+AddAssign+SubAssign+Ord+Zero+Add<Output=Tally>+Sub<Output=Tally>> SubAssign<Tally> for SignedVersion<Tally> {
    fn sub_assign(&mut self, rhs: Tally) {
        if !self.negative {
            if self.value==rhs { self.negative=false; self.value=Tally::zero(); }
            else if self.value>rhs { self.value-=rhs; }
            else { self.negative=true; self.value=rhs-self.value.clone();}
        } else { self.value+=rhs; }
    }
}

impl From<SignedVersion<f64>> for f64 {
    fn from(v: SignedVersion<f64>) -> Self {
        if v.negative { -v.value } else { v.value }
    }
}

impl <X:Clone> From<X> for SignedVersion<X> {
    fn from(value: X) -> Self { SignedVersion{ negative: false, value } }
}

impl <X:Ord> Ord for SignedVersion<X> {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.negative!=other.negative {
            if self.negative { Ordering::Less } else { Ordering::Greater }
        } else {
            let ignoring_sign = self.value.cmp(&other.value);
            if self.negative { ignoring_sign.reverse() } else { ignoring_sign }
        }
    }
}

impl <X:Ord> PartialOrd for SignedVersion<X> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl <X:ToString> Serialize for SignedVersion<X> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}


/*
impl <'de,X:FromStr> Deserialize<'de> for SignedVersion<X> where <X as FromStr>::Err: std::fmt::Display  {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let buf = String::deserialize(deserializer)?;
        let negative = buf.starts_with("-");
        let trimmed = if negative { &buf[1..] } else { &buf };
        let value = buf.parse().map_err(serde::de::Error::custom)?;
        Ok(SignedVersion{ negative, value })
    }
}
*/
impl <'de,X:FromStr> Deserialize<'de> for SignedVersion<X>  {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let buf = String::deserialize(deserializer)?;
        let negative = buf.starts_with("-");
        let trimmed = if negative { &buf[1..] } else { &buf };
        let value = trimmed.parse().map_err(|_| serde::de::Error::custom("fromstr"))?;
        Ok(SignedVersion{ negative, value })
    }
}

/*

impl <const DIGITS:usize> Serialize for SignedVersion<FixedPrecisionDecimal<DIGITS>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

impl <'de,const DIGITS:usize> Deserialize<'de> for SignedVersion<FixedPrecisionDecimal<DIGITS>> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let buf = String::deserialize(deserializer)?;
        let negative = buf.starts_with("-");
        let trimmed = if negative { &buf[1..] } else { &buf };
        let value = buf.parse().map_err(serde::de::Error::custom)?;
        Ok(SignedVersion{ negative, value })
    }
}

impl Serialize for SignedVersion<usize> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        if self.negative { serializer.serialize_i64(- (self.value as i64)) }
        else { serializer.serialize_u64(self.value as u64) }
    }
}

impl <'de> Deserialize<'de> for SignedVersion<usize> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let buf = i64::deserialize(deserializer)?;
        if buf>=0 { Ok(SignedVersion{ negative: false, value: buf as usize })}
        else { Ok(SignedVersion{ negative: true, value: (-buf) as usize })}
    }
}

impl Serialize for SignedVersion<BallotPaperCount> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        if self.negative { serializer.serialize_i64(- (self.value.0 as i64)) }
        else { serializer.serialize_u64(self.value.0 as u64) }
    }
}

impl <'de> Deserialize<'de> for SignedVersion<BallotPaperCount> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let buf = i64::deserialize(deserializer)?;
        if buf>=0 { Ok(SignedVersion{ negative: false, value: BallotPaperCount(buf as usize) })}
        else { Ok(SignedVersion{ negative: true, value: BallotPaperCount((-buf) as usize) })}
    }
}


*/

