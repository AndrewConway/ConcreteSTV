// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use serde::Serialize;
use serde::Deserialize;
use num::{One, BigRational, BigInt, ToPrimitive, Zero};
use crate::ballot_pile::{BallotPaperCount, DistributedVotes};
use std::fmt::{Debug, Display, Formatter};
use std::convert::TryFrom;
use std::hash::Hash;
use std::str::FromStr;
use num::rational::{ParseRatioError, Ratio};
use crate::ballot_metadata::CandidateIndex;
use crate::distribution_of_preferences_transcript::{DecisionMadeByEC, Transcript};
use crate::tie_resolution::{MethodOfTieResolution, TieResolutionGranularityNeeded};

#[derive(Clone,Debug,Serialize,Deserialize,Ord, PartialOrd, Eq, PartialEq,Hash)]
#[serde(into = "String")]
#[serde(try_from = "String")]
pub struct TransferValue(pub(crate) BigRational);

impl TransferValue {
    pub fn one() -> Self { TransferValue(BigRational::one())}
    pub fn new(numerator:BigInt,denominator:BigInt) -> Self {
        TransferValue(BigRational::new(numerator,denominator))
    }
    pub fn from_surplus(surplus:usize,denominator:BallotPaperCount) -> Self {
        TransferValue::new(BigInt::from(surplus),BigInt::from(denominator.0))
    }

    pub fn mul(&self,papers:BallotPaperCount) -> BigRational {
        BigRational::new(self.0.numer().clone()*BigInt::from(papers.0),self.0.denom().clone())
    }

    pub fn mul_rounding_down(&self,papers:BallotPaperCount) -> usize {
        let exact = self.mul(papers);
        round_rational_down_to_usize(exact)
    }
    pub fn mul_rounding_down_and_remainder(&self,papers:BallotPaperCount) -> (usize,BigRational) {
        let exact = self.mul(papers);
        let rounded_down_to_integer = round_rational_down_to_usize(exact.clone());
        let remainder = exact-BigRational::new(BigInt::from(rounded_down_to_integer), BigInt::one());
        (rounded_down_to_integer,remainder)
    }
    /// like mul_rounding_down, but round up if the fraction is >0.5
    pub fn mul_rounding_nearest(&self,papers:BallotPaperCount) -> usize {
        let exact = self.mul(papers);
        let rounded_down = exact.numer().clone()/exact.denom().clone();
        let rounded_down = rounded_down.to_usize().unwrap();
        let remainder = exact.numer().clone()%exact.denom().clone();
        if &(remainder*2) > exact.denom() { rounded_down+1 } else {rounded_down}
    }

    pub fn num_ballot_papers_to_get_this_tv(&self,tally:BigRational) -> BallotPaperCount {
        if tally.is_zero() { BallotPaperCount(0) } else {
            let needed = tally/self.0.clone();
            // want to round needed up to nearest integer.
            let rounded_up_to_int = (needed.numer().clone() + needed.denom().clone() - BigInt::one()) / needed.denom().clone();
            BallotPaperCount(rounded_up_to_int.to_usize().unwrap())
        }
    }

    pub fn is_one(&self) -> bool { &Self::one()==self}
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

/// Utility for NSW random style selection
struct SelectVotesToSetAsideByTV {
    candidate : CandidateIndex,
    distributed : BallotPaperCount,
    integer_portion : BallotPaperCount,
    fractional_portion : BigRational,
}



impl TransferValue {
    /// Implement the NSW Legislative Council and old LGE method of working out how many
    /// votes for each candidate need to be set aside so that the correct number of ballot papers
    /// are transferred.
    ///
    /// Returns an array of candidates
    pub fn calculate_number_of_ballot_papers_to_be_set_aside<Tally:Clone+Hash+Ord+Display+FromStr+Debug>(&self,surplus:BallotPaperCount,num_candidates:usize,transcript:&Transcript<Tally>,distributed:&DistributedVotes<'_>) -> (Vec<BallotPaperCount>,Option<DecisionMadeByEC>)  {
        let mut ec_decision : Option<DecisionMadeByEC> = None;
        let set_aside_by_candidate = if self.is_one() { // work out how to distribute.
            vec![BallotPaperCount::zero();num_candidates]
        } else {
            let mut compute_transferred = vec![];
            let mut extra_to_distribute : usize = surplus.0;
            for candidate in 0..num_candidates {
                let n_distributed = distributed.by_candidate[candidate].num_ballots;
                let (integer_portion,fractional_portion) = self.mul_rounding_down_and_remainder(n_distributed);
                compute_transferred.push(SelectVotesToSetAsideByTV{
                    candidate: CandidateIndex(candidate),
                    distributed: n_distributed,
                    integer_portion : BallotPaperCount(integer_portion),
                    fractional_portion,
                });
                extra_to_distribute-=integer_portion;
            }
            if extra_to_distribute>0 {
                compute_transferred.sort_unstable_by(|a,b|{
                    let c1 = b.fractional_portion.cmp(&a.fractional_portion);
                    if c1.is_eq() {
                        b.integer_portion.cmp(&a.integer_portion)
                    } else {c1}
                });
                if compute_transferred[extra_to_distribute-1].distributed==compute_transferred[extra_to_distribute].distributed { // need to split ties somehow.
                    let mut start_tied_index = extra_to_distribute-1;
                    while start_tied_index>0 && compute_transferred[extra_to_distribute].distributed==compute_transferred[start_tied_index-1].distributed { start_tied_index-=1; }
                    let mut end_tied_index_exclusive = extra_to_distribute+1;
                    while end_tied_index_exclusive<compute_transferred.len() && compute_transferred[extra_to_distribute].distributed==compute_transferred[end_tied_index_exclusive].distributed { end_tied_index_exclusive+=1; }
                    let mut tied_candidates : Vec<CandidateIndex> = compute_transferred[start_tied_index..end_tied_index_exclusive].iter().map(|v|v.candidate).collect();
                    let num_missing_out_on_rounding_up = end_tied_index_exclusive-extra_to_distribute;
                    ec_decision = MethodOfTieResolution::AnyDifferenceIsADiscriminator.resolve(&mut tied_candidates,transcript,TieResolutionGranularityNeeded::LowestSeparated(num_missing_out_on_rounding_up));
                    for i in 0..tied_candidates.len() {
                        compute_transferred[i+start_tied_index].candidate=tied_candidates[tied_candidates.len()-1-i]; // tied_candidates is sorted low to high.
                    }
                }
                for i in 0..extra_to_distribute {
                    compute_transferred[i].integer_portion+=BallotPaperCount(1);
                }
                compute_transferred.sort_unstable_by_key(|v|v.candidate.0)
            }
            compute_transferred.iter().map(|v|v.distributed-v.integer_portion).collect()
        };
        (set_aside_by_candidate,ec_decision)
    }
}

