// Copyright 2021-2024 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Store the history of the distribution of preferences


use crate::ballot_pile::BallotPaperCount;
use crate::ballot_metadata::{CandidateIndex, ElectionMetadata, NumberOfCandidates};
use crate::transfer_value::{TransferValue, StringSerializedRational};
use serde::{Serialize,Deserialize};
use std::fmt::{Debug, Display, Formatter};
use crate::preference_distribution::TransferValueMethod;
use crate::signed_version::SignedVersion;
use std::str::FromStr;
use crate::official_dop_transcript::CanConvertToF64PossiblyLossily;
use crate::simple_list_of_votes::ListOfVotes;
use crate::tie_resolution::TieResolutionExplicitDecision;


/// The index of a count. 0 means the first. This is different from the human readable
/// count, which may be more complex and have sub-counts as well.
#[derive(Copy,Clone,Debug,Ord, PartialOrd, Eq, PartialEq,Hash,Serialize,Deserialize)]
pub struct CountIndex(pub usize);

impl Display for CountIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.0)
    }
}


/// A value that is primarily per candidate, but may also go to some other source.
/// Generally, this is used for preserved properties such that the sum over all candidates and other destinations is always the same.
/// For instance, ballots, which start out all assigned to candidates, are shifted around between people, but some will get exhausted.
/// Alternatively, votes, which start out all assigned to candidates, but may get lost due to rounding or weird rules or exhaustion.
#[derive(Clone,Serialize,Deserialize, PartialEq,Debug)]
pub struct PerCandidate<X:PartialEq+Clone+Display+FromStr> {
    /// the value for a given candidate.
    pub candidate : Vec<X>,
    /// something is exhausted if it can't go to a specific candidate as there are not enough preferences on a particular ballot.
    pub exhausted : X,
    /// something goes to rounding if it can't go to a specific candidate as fractions are not allowed.
    pub rounding : SignedVersion<X>,
    /// something gets set aside if some feature of the rules means it doesn't go to a particular candidate. None if not applicable.
    pub set_aside : Option<X>,
}

impl <X:Default+PartialEq+Clone+Display+FromStr> Default for PerCandidate<X> {
    fn default() -> Self {
        PerCandidate{
            candidate: vec![],
            exhausted: X::default(),
            rounding: Default::default(),
            set_aside: None
        }
    }
}

impl <X:Default+PartialEq+Clone+Display+FromStr> PerCandidate<X> {
    pub fn from_num_candidates(len:usize,unknown_value:X) -> Self {
        PerCandidate{
            candidate: vec![unknown_value;len],
            exhausted: X::default(),
            rounding: Default::default(),
            set_aside: None
        }
    }
}


impl <Tally:PartialEq+Clone+Display+FromStr+CanConvertToF64PossiblyLossily> PerCandidate<Tally> {
    /// Like equals, but for potentially different tally types
    pub fn same<Tally2:PartialEq+Clone+Display+FromStr+CanConvertToF64PossiblyLossily>(&self,other:&PerCandidate<Tally2>) -> bool {
        if self.candidate.len()!=other.candidate.len() { return false; }
        for i in 0..self.candidate.len() {
            if self.candidate[i].convert_to_f64()!=other.candidate[i].convert_to_f64() { return false; }
        }
        if self.exhausted.convert_to_f64()!=other.exhausted.convert_to_f64() { return false; }
        if self.rounding.convert_f64(|t|t.convert_to_f64())!=other.rounding.convert_f64(|t|t.convert_to_f64()) { return false; }
        match (self.set_aside.as_ref(),other.set_aside.as_ref()) {
            (None, None) => true,
            (Some(_), None) => false,
            (None,Some(_)) => false,
            (Some(a),Some(b)) => a.convert_to_f64()==b.convert_to_f64(),
        }
    }
}
#[derive(thiserror::Error, Debug)]
#[error("Not an integer")]
pub struct NotInteger {}

#[derive(thiserror::Error, Debug)]
#[error("Not a non-negative integer")]
pub struct NotNonnegativeInteger {}

impl TryFrom<PerCandidate<f64>> for PerCandidate<isize> {
    type Error = NotInteger;

    fn try_from(value: PerCandidate<f64>) -> Result<Self, Self::Error> {
        let as_int = |f:f64| -> Result<isize,NotInteger> {
            if f.is_nan() { Ok(isize::MAX) }
            else if f == (f as isize) as f64 { Ok(f as isize) }
            else { Err(NotInteger{}) }
        };
        Ok(PerCandidate::<isize>{
            candidate: value.candidate.into_iter().map(as_int).collect::<Result<Vec<isize>,NotInteger>>()?,
            exhausted: as_int(value.exhausted)?,
            rounding: SignedVersion{ negative: value.rounding.negative, value: as_int(value.rounding.value)? },
            set_aside:  value.set_aside.map(as_int).transpose()?,
        })
    }
}

impl TryFrom<PerCandidate<f64>> for PerCandidate<usize> {
    type Error = NotNonnegativeInteger;

    fn try_from(value: PerCandidate<f64>) -> Result<Self, Self::Error> {
        let as_int = |f:f64| -> Result<usize,NotNonnegativeInteger> {
            if f.is_nan() { Ok(usize::MAX) }
            else if f == (f as usize) as f64 { Ok(f as usize) }
            else { Err(NotNonnegativeInteger{}) }
        };
        Ok(PerCandidate::<usize>{
            candidate: value.candidate.into_iter().map(as_int).collect::<Result<Vec<usize>,NotNonnegativeInteger>>()?,
            exhausted: as_int(value.exhausted)?,
            rounding: SignedVersion{ negative: value.rounding.negative, value: as_int(value.rounding.value)? },
            set_aside:  value.set_aside.map(as_int).transpose()?,
        })
    }
}

impl PerCandidate<usize> {
    /// See if every field is either 0 or usize::MAX (often meaning unknown).
    pub fn is_empty(&self) -> bool {
        fn zero(i:usize) -> bool { i==0 || i==usize::MAX}
        zero(self.exhausted) && self.candidate.iter().all(|v|zero(*v)) && self.set_aside.iter().all(|v|zero(*v)) && zero(self.rounding.value)
    }

    pub fn sum(&self) -> usize {
        let mut res = self.exhausted;
        for s in &self.candidate { res+=*s; }
        if let Some(s) = self.set_aside { res+=s; }
        if self.rounding.negative { res-=self.rounding.value } else { res+=self.rounding.value }
        res
    }
}

impl From<PerCandidate<isize>> for PerCandidate<f64> {
    fn from(value: PerCandidate<isize>) -> Self {
        let from_int = |f:isize| -> f64 {
            if f==isize::MAX { f64::NAN }
            else { f as f64 }
        };
        PerCandidate::<f64>{
            candidate: value.candidate.into_iter().map(from_int).collect(),
            exhausted: from_int(value.exhausted),
            rounding: SignedVersion{ negative: value.rounding.negative, value: from_int(value.rounding.value) },
            set_aside:  value.set_aside.map(from_int),
        }
    }
}
impl From<PerCandidate<usize>> for PerCandidate<f64> {
    fn from(value: PerCandidate<usize>) -> Self {
        let from_int = |f:usize| -> f64 {
            if f==usize::MAX { f64::NAN }
            else { f as f64 }
        };
        PerCandidate::<f64>{
            candidate: value.candidate.into_iter().map(from_int).collect(),
            exhausted: from_int(value.exhausted),
            rounding: SignedVersion{ negative: value.rounding.negative, value: from_int(value.rounding.value) },
            set_aside:  value.set_aside.map(from_int),
        }
    }
}
/// Record the status of the count at the end of the count.
#[derive(Clone,Serialize,Deserialize,PartialEq,Debug)]
pub struct EndCountStatus<Tally:PartialEq+Clone+Display+FromStr> {
    /// tallies for each candidate
    pub tallies : PerCandidate<Tally>,
    /// the number of pieces of paper for each candidate
    pub papers : PerCandidate<BallotPaperCount>,
    /// the number of above pieces of paper that are ATL.
    pub atl_papers : Option<PerCandidate<BallotPaperCount>>,
    /// usually not present list of all votes' positions.
    #[serde(default,skip_serializing_if = "Option::is_none")]
    pub list_of_votes: Option<PerCandidate<ListOfVotes>>,
}

impl <Tally:PartialEq+Clone+Display+FromStr+CanConvertToF64PossiblyLossily> EndCountStatus<Tally> {
    /// Like equals, but for potentially different tally types
    pub fn same<Tally2:PartialEq+Clone+Display+FromStr+CanConvertToF64PossiblyLossily>(&self,other:&EndCountStatus<Tally2>) -> bool {
        self.papers==other.papers && self.atl_papers==other.atl_papers && self.tallies.same(&other.tallies)
    }
}

#[derive(Clone,Serialize,Deserialize,Debug)]
pub enum ReasonForCount {
    FirstPreferenceCount,
    ExcessDistribution(CandidateIndex),
    Elimination(Vec<CandidateIndex>),  // usually just one candidate, but federal rules allow multiple elimination
}

impl ReasonForCount {
    pub fn is_elimination(&self) -> bool {
        match self {
            ReasonForCount::Elimination(_) => true,
            _ => false,
        }
    }
    pub fn is_surplus(&self) -> bool {
        match self {
            ReasonForCount::ExcessDistribution(_) => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone,Serialize,Deserialize,Eq, PartialEq,Debug)]
pub enum ElectionReason {
    ReachedQuota,
    HighestOfLastTwoStanding,
    AllRemainingMustBeElected,
    OverwhelmingTally,
}

#[derive(Copy, Clone,Serialize,Deserialize,Eq, PartialEq,Debug)]
pub struct CandidateElected {
    pub who : CandidateIndex,
    pub why : ElectionReason,
}

#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct PortionOfReasonBeingDoneThisCount {
    pub transfer_value : Option<TransferValue>,
    pub when_tv_created: Option<CountIndex>,
    pub papers_came_from_counts : Vec<CountIndex>,
}


#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct TransferValueCreation<Tally> {
    pub surplus : Tally,
    pub votes : Tally,
    #[serde(default)] // added post first release, so old files may not have it.
    pub excluded_exhausted_tally : Option<StringSerializedRational>,
    pub original_transfer_value : Option<TransferValue>,
    #[serde(default)] // added post first release, so old files may not have it.
    pub multiplied_transfer_value : Option<TransferValue>,
    /// The number of ballots considered for redistribution. This may be all or a last parcel.
    pub ballots_considered : BallotPaperCount,
    /// The number of the considered ballots that are continuing
    pub continuing_ballots : BallotPaperCount,
    pub transfer_value : TransferValue,
    pub source : TransferValueMethod,
}

/// Sometimes the Electoral Commission needs to make a decision, such as tie resolution.
/// Sometimes legislation mandates this be random, sometimes the returning officer.
/// Regardless, this records that the decision needs to be made.
/// TODO remove.
#[derive(Clone,Serialize,Deserialize)]
pub struct DecisionMadeByEC {
    pub affected : Vec<CandidateIndex>
}

impl Display for DecisionMadeByEC {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Decision({})",self.affected.iter().map(|c|c.to_string()).collect::<Vec<_>>().join(","))
    }
}





#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct SingleCount<Tally:PartialEq+Clone+Display+FromStr> {
    /// The action that is being done in said count
    pub reason : ReasonForCount,
    /// If only a sub portion of that reason is done in that count, why will be in here. Other info could also be in here (like which counts papers came from) even if it doesn't restrict things for this set of STV rules.
    pub portion : PortionOfReasonBeingDoneThisCount,
    /// true if the action in reason is finished in this count.
    pub reason_completed : bool,
    /// Who, if anyone, was elected in this count.
    pub elected : Vec<CandidateElected>,
    /// Who stopped being a continuing candidate for the first time at the start of this count. Candidates who are excluded from the contest are labeled here for the first count. Candidates elected in this count will be included here in the next count.
    pub not_continuing : Vec<CandidateIndex>,
    /// If a transfer value was created, how
    pub created_transfer_value : Option<TransferValueCreation<Tally>>,
    /// the decisions made by the EC (possibly randomly)
    pub decisions : Vec<TieResolutionExplicitDecision>,
    /// if there are any set aside for quota votes on this distribution (at time of writing only used for old NSW)
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub set_aside_for_quota: Option<PerCandidate<BallotPaperCount>>,
    /// status at end of count.
    pub status : EndCountStatus<Tally>,
    /// A special name for the count, if not 1,2,3,... Mainly used so that each exclusion or surplus distribution is a single "major" count with possibly minor counts included.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub count_name : Option<String>,
}

#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct QuotaInfo<Tally:Debug> {
    pub papers : BallotPaperCount,
    pub vacancies : NumberOfCandidates,
    pub quota : Tally,
}

impl <Tally:Display+Debug> Display for QuotaInfo<Tally> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"({} Papers)/({} vacancies+1) -> quota {}",self.papers,self.vacancies,self.quota)
    }
}

#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct Transcript<Tally:PartialEq+Clone+Display+FromStr+Debug> {
    /// The rules that were used to compute this transcript.
    pub rules : String,
    #[serde(skip_serializing_if = "Option::is_none",default="produce_none")] // can't just have default as there is no default on Tally, which is needed for some reason.
    pub quota : Option<QuotaInfo<Tally>>,
    pub counts : Vec<SingleCount<Tally>>,
    pub elected : Vec<CandidateIndex>,
}

fn produce_none<T>() -> Option<T> { None }

impl <Tally:PartialEq+Clone+Display+FromStr+Debug> Transcript<Tally> {
    pub fn count(&self,index:CountIndex) -> &SingleCount<Tally> {
        &self.counts[index.0]
    }
}

#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct TranscriptWithMetadata<Tally:PartialEq+Clone+Display+FromStr+Debug> {
    pub metadata : ElectionMetadata,
    pub transcript : Transcript<Tally>,
}


