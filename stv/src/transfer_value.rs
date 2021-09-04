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
        let rounded_down = exact.numer().clone()/exact.denom().clone();
        rounded_down.to_usize().unwrap()
    }
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
