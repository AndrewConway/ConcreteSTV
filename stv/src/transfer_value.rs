// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use serde::Serialize;
use serde::Deserialize;
use num::{One, BigRational, BigInt, ToPrimitive};
use crate::ballot_pile::BallotPaperCount;
use std::fmt::{Display, Formatter};
use std::convert::TryFrom;
use std::str::FromStr;
use num::rational::{ParseRatioError, Ratio};

#[derive(Clone,Debug,Serialize,Deserialize,Ord, PartialOrd, Eq, PartialEq,Hash)]
#[serde(into = "String")]
#[serde(try_from = "String")]
pub struct TransferValue(pub(crate) num::rational::BigRational);

impl TransferValue {
    pub fn one() -> Self { TransferValue(num::rational::BigRational::one())}
    pub fn new(numerator:BigInt,denominator:BigInt) -> Self {
        TransferValue(BigRational::new(numerator,denominator))
    }
    pub fn from_surplus(surplus:usize,denominator:BallotPaperCount) -> Self {
        TransferValue::new(BigInt::from(surplus),BigInt::from(denominator.0))
    }

    pub fn mul(&self,papers:BallotPaperCount) -> num::rational::BigRational {
        BigRational::new(self.0.numer().clone()*BigInt::from(papers.0),self.0.denom().clone())
    }

    pub fn mul_rounding_down(&self,papers:BallotPaperCount) -> usize {
        let exact = self.mul(papers);
        round_rational_down_to_usize(exact)
    }
    /// like mul_rounding_down, but round up if the fraction is >0.5
    pub fn mul_rounding_nearest(&self,papers:BallotPaperCount) -> usize {
        let exact = self.mul(papers);
        let rounded_down = exact.numer().clone()/exact.denom().clone();
        let rounded_down = rounded_down.to_usize().unwrap();
        let remainder = exact.numer().clone()%exact.denom().clone();
        if &(remainder*2) > exact.denom() { rounded_down+1 } else {rounded_down}
    }
}

/// Round a rational number down to a usize.
pub fn round_rational_down_to_usize(rational:BigRational) -> usize {
    let rounded_down = rational.numer().clone()/rational.denom().clone();
    rounded_down.to_usize().unwrap()
}
pub fn convert_usize_to_rational(tally:usize) -> BigRational {
    BigRational::new(BigInt::from(tally),BigInt::one())
}

impl Display for TransferValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.0)
    }
}

impl From<TransferValue> for String {
    fn from(t: TransferValue) -> Self { t.0.to_string() }
}

impl FromStr for TransferValue {
    type Err = ParseRatioError;
    fn from_str(s: &str) -> Result<Self, Self::Err> { Ok(TransferValue(Ratio::from_str(s)?)) }
}

impl TryFrom<String> for TransferValue {
    type Error = ParseRatioError;
    fn try_from(s: String) -> Result<Self, Self::Error> { Ok(TransferValue(Ratio::from_str(&s)?)) }
}

#[derive(Clone,Debug,Serialize,Deserialize,Ord, PartialOrd, Eq, PartialEq,Hash)]
#[serde(into = "String")]
#[serde(try_from = "String")]
/// A rational number that should be serialized/deserialized as a string. Equivalent to TransferValue in most ways, except without the TransferValue specific methods and name.
pub struct StringSerializedRational(pub num::rational::BigRational);

impl Display for StringSerializedRational {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.0)
    }
}

impl From<StringSerializedRational> for String {
    fn from(t: StringSerializedRational) -> Self { t.0.to_string() }
}

impl FromStr for StringSerializedRational {
    type Err = ParseRatioError;
    fn from_str(s: &str) -> Result<Self, Self::Err> { Ok(StringSerializedRational(Ratio::from_str(s)?)) }
}

impl TryFrom<String> for StringSerializedRational {
    type Error = ParseRatioError;
    fn try_from(s: String) -> Result<Self, Self::Error> { Ok(StringSerializedRational(Ratio::from_str(&s)?)) }
}
