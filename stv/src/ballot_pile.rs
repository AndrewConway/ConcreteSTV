// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Things to do with piles of ballots.



use std::cmp::Ordering;
use crate::ballot_metadata::CandidateIndex;
use crate::ballot_paper::VoteSource;
use std::collections::{HashSet, HashMap};
use crate::transfer_value::TransferValue;
use num::{Zero};
use std::ops::{AddAssign, Sub, Add, SubAssign};
use serde::Deserialize;
use serde::Serialize;
use std::hash::Hash;
use crate::distribution_of_preferences_transcript::{PortionOfReasonBeingDoneThisCount, CountIndex, Transcript};
use crate::util::{DetectUnique, CollectAll};
use std::fmt;
use std::fmt::{Debug, Display};
use std::iter::Sum;
use std::str::FromStr;

/// A number representing a count of pieces of paper.
/// This is distinct from votes which may be fractional in the presence of weights.
#[derive(Copy,Clone,Eq, PartialEq,Serialize,Deserialize,Ord, PartialOrd)]
pub struct BallotPaperCount(pub usize);

impl AddAssign for BallotPaperCount {
    fn add_assign(&mut self, rhs: Self) { self.0+=rhs.0; }
}
impl SubAssign for BallotPaperCount {
    fn sub_assign(&mut self, rhs: Self) { self.0-=rhs.0; }
}

impl Sub for BallotPaperCount {
    type Output = BallotPaperCount;
    fn sub(self, rhs: Self) -> Self::Output { BallotPaperCount(self.0-rhs.0) }
}

impl Add for BallotPaperCount {
    type Output = BallotPaperCount;
    fn add(self, rhs: Self) -> Self::Output { BallotPaperCount(self.0+rhs.0) }
}
// type alias really, don't want long display
impl fmt::Display for BallotPaperCount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}
// type alias really, don't want long display
impl fmt::Debug for BallotPaperCount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}
impl Zero for BallotPaperCount {
    fn zero() -> Self { BallotPaperCount(0) }
    fn is_zero(&self) -> bool { self.0 == 0 }
}
impl FromStr for BallotPaperCount {
    type Err = <usize as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(BallotPaperCount(s.parse()?))
    }
}
impl Sum for BallotPaperCount {
    fn sum<I: Iterator<Item=Self>>(iter: I) -> Self {
        BallotPaperCount(usize::sum(iter.map(|b|b.0)))
    }
}

/// A vote, resolved into BTL, that is somewhere through being distributed.
/// Ignore preferences with index less than upto.
/// May consist of multiple independent identical votes.
#[derive(Copy, Clone,Debug)]
pub struct PartiallyDistributedVote<'a> {
    pub(crate) upto : usize,
    /// The number of voters
    pub n : BallotPaperCount,
    /// Preferred candidates, with index 0 being the most favoured candidate.
    pub(crate) prefs : &'a[CandidateIndex],
    pub(crate) source : VoteSource<'a>,
}

impl<'a>  PartiallyDistributedVote<'a> {
    pub fn new(n:usize,prefs : &'a[CandidateIndex],source : VoteSource<'a>) -> Self {
        PartiallyDistributedVote{
            upto: 0,
            n: BallotPaperCount(n),
            prefs,
            source
        }
    }
    pub fn exhausted(&self) -> bool { self.upto==self.prefs.len() }
    pub fn candidate(&self) -> CandidateIndex { self.prefs[self.upto] }
    pub fn next(&self,continuing:&HashSet<CandidateIndex>) -> Option<Self> {
        for i in self.upto .. self.prefs.len() {
            if continuing.contains(&self.prefs[i]) {
                return Some(PartiallyDistributedVote{upto:i,n:self.n,prefs:self.prefs,source:self.source})
            }
        }
        None
    }
    /// true iff it is an above the line vote
    pub fn is_atl(&self) -> bool {
        match self.source {
            VoteSource::Btl(_) => false,
            VoteSource::Atl(_) => true
        }
    }
}
#[derive(Clone)]
pub struct PileProvenance<Tally> {
    pub counts_comes_from : HashSet<CountIndex>,
    pub when_tv_created:Option<CountIndex>, // if there is a unique time the TV was created, hold it.
    /// The number of actual votes this translated to.
    pub tally : Tally,
}

impl <Tally:AddAssign> PileProvenance<Tally> {
    pub fn add(&mut self,count_index:CountIndex,when_tv_created:Option<CountIndex>,tally:Tally) {
        self.counts_comes_from.insert(count_index);
        if self.when_tv_created!=when_tv_created { self.when_tv_created=None} // conflicting -> None.
        self.tally+=tally
    }
}

/// A pile of votes with the same transfer value, and whatever provenence matters.
/// In a physical count, this would typically be a single pile. Except it might get too high. A metaphorical single pile.
#[derive(Clone)]
pub struct VotesWithSameTransferValue<'a> {
    pub votes : Vec<PartiallyDistributedVote<'a>>,
    pub num_ballots : BallotPaperCount,
    pub num_atl_ballots : BallotPaperCount,
}

impl <'a> Default for VotesWithSameTransferValue<'a> {
    fn default() -> Self {
        VotesWithSameTransferValue{
            votes: vec![],
            num_ballots: BallotPaperCount(0),
            num_atl_ballots: BallotPaperCount(0)
        }
    }
}

/// For jurisdictions that use a last parcel, sufficient information to revert to an earlier state. Used with struct [VotesWithSameTransferValue]
pub struct StateBeforeAddition {
    votes_len : usize, // length of the votes vector
}

impl <'a> VotesWithSameTransferValue<'a> {
    // number of below the line ballots in this pile
    pub fn num_btl_ballots(&self) -> BallotPaperCount {  BallotPaperCount(self.num_ballots.0-self.num_atl_ballots.0)  }

    pub fn add_vote(&mut self,vote : PartiallyDistributedVote<'a>) {
        self.num_ballots+=vote.n;
        if vote.is_atl() { self.num_atl_ballots+=vote.n; }
        self.votes.push(vote);
    }
    /// Add in some votes, and give a token that can be passed to [extract_last_parcel] to revert to the current state.
    pub fn add(&mut self,votes:&Vec<PartiallyDistributedVote<'a>>) -> StateBeforeAddition {
        let old_state = StateBeforeAddition{votes_len:self.votes.len()};
        for v in votes {
            self.add_vote(*v);
        }
        old_state
    }
    /// revert to the prior state with a token from [add], returning the votes removed. Used to get the last parcel.
    fn extract_last_parcel(&mut self,old_state:StateBeforeAddition) -> VotesWithSameTransferValue<'a> {
        let mut res = VotesWithSameTransferValue::default();
        for v in self.votes.drain(old_state.votes_len..) {
            res.add_vote(v);
        }
        self.num_atl_ballots-=res.num_atl_ballots;
        self.num_ballots-=res.num_ballots;
        res
    }
}

/// Votes distributed amongst continuing candidates.
pub struct DistributedVotes<'a> {
    pub(crate) by_candidate : Vec<VotesWithSameTransferValue<'a>>,
    pub(crate) exhausted : BallotPaperCount,
    pub(crate) exhausted_atl : BallotPaperCount,
}

impl <'a> DistributedVotes<'a> {
    pub fn distribute(votes:&Vec<PartiallyDistributedVote<'a>>,continuing_candidates:&HashSet<CandidateIndex>,num_candidates:usize) -> Self {
        let mut by_candidate = vec![VotesWithSameTransferValue::default();num_candidates];
        let mut exhausted = BallotPaperCount(0);
        let mut exhausted_atl = BallotPaperCount(0);
        for vote in votes {
            if let Some(next) = vote.next(continuing_candidates) {
                by_candidate[next.candidate().0].add_vote(next);
            } else { exhausted+=vote.n; if vote.is_atl() { exhausted_atl+=vote.n; } }
        }
        DistributedVotes{by_candidate,exhausted,exhausted_atl}
    }
}


/// Different jurisdictions split up parcels of shares by their provenence in different ways. This abstracts that.
pub trait HowSplitByCountNumber {
    type KeyToDivide : Eq+Hash+Clone+Ord+Debug;
    fn key(count_index:CountIndex,when_tv_created:Option<CountIndex>) -> Self::KeyToDivide;
}

/// Simple, don't care about the count number, just the value of the transfer value.
pub struct DoNotSplitByCountNumber {}
impl HowSplitByCountNumber for DoNotSplitByCountNumber {
    type KeyToDivide = ();
    fn key(_count_index: CountIndex, _when_tv_created: Option<CountIndex>) -> () {}
}
/// Treat each count number as a separate parcel.
pub struct FullySplitByCountNumber {}
impl HowSplitByCountNumber for FullySplitByCountNumber {
    type KeyToDivide = CountIndex;
    fn key(count_index: CountIndex, _when_tv_created: Option<CountIndex>) -> Self::KeyToDivide { count_index }
}
/// Treat the first count as a separate parcel from all other counts
pub struct SplitFirstCount {}
impl HowSplitByCountNumber for SplitFirstCount {
    type KeyToDivide = bool;
    fn key(count_index: CountIndex, _when_tv_created: Option<CountIndex>) -> Self::KeyToDivide { count_index.0 == 0 }
}
/// Split by when the transfer value was created
pub struct SplitByWhenTransferValueWasCreated {}
impl HowSplitByCountNumber for SplitByWhenTransferValueWasCreated {
    type KeyToDivide = CountIndex;
    fn key(_count_index: CountIndex, when_tv_created: Option<CountIndex>) -> Self::KeyToDivide { when_tv_created.unwrap() }
}

struct LastParcelInfo {
    prior_state : StateBeforeAddition,
    transfer_value : TransferValue,
    when_tv_created:Option<CountIndex>,
    count_index:CountIndex,
}
/// A set of votes potentially with multiple transfer values or sources.
/// These would typically be the votes given to a particular individual.
pub struct VotesWithMultipleTransferValues<'a,S:HowSplitByCountNumber,Tally> {

    last_parcel : Option<LastParcelInfo>,
    by_provenance : HashMap<(S::KeyToDivide,TransferValue),(PileProvenance<Tally>,VotesWithSameTransferValue<'a>)>

}

impl <'a,S:HowSplitByCountNumber,Tally> Default for VotesWithMultipleTransferValues<'a,S,Tally> {
    fn default() -> Self {
        VotesWithMultipleTransferValues{ last_parcel: None, by_provenance: HashMap::default() }
    }
}

impl <'a,S:HowSplitByCountNumber,Tally:AddAssign+Zero+Clone+Display+FromStr+PartialEq> VotesWithMultipleTransferValues<'a,S,Tally> {
    pub fn add(& mut self,votes:&'_ VotesWithSameTransferValue<'a>,transfer_value:TransferValue,count_index:CountIndex,when_tv_created:Option<CountIndex>,tally:Tally) {
        let key = (S::key(count_index,when_tv_created),transfer_value.clone());
        let entry = self.by_provenance.entry(key).or_insert_with(||
            (PileProvenance{ counts_comes_from: Default::default(),when_tv_created,tally:Tally::zero()}, VotesWithSameTransferValue::default()));
        entry.0.add(count_index,when_tv_created,tally);
        let prior_state = entry.1.add(&votes.votes);
        self.last_parcel = Some(LastParcelInfo{
            prior_state,
            transfer_value,
            when_tv_created,
            count_index,
        });
    }

    pub fn get_all_provenance_keys(&self) -> Vec<(S::KeyToDivide,TransferValue)> {
        let mut res: Vec<(S::KeyToDivide,TransferValue)> = vec![];
        for x in self.by_provenance.keys() {
            res.push((*x).clone());
        }
        res
    }

    pub fn num_ballots(&self) -> BallotPaperCount {
        let mut res = BallotPaperCount(0);
        for (_,votes) in self.by_provenance.values() {
            res+=votes.num_ballots;
        }
        res
    }
    pub fn num_atl_ballots(&self) -> BallotPaperCount {
        let mut res = BallotPaperCount(0);
        for (_,votes) in self.by_provenance.values() {
            res+=votes.num_atl_ballots;
        }
        res
    }

    /// Removes the last parcel from this object, returning the object created.
    pub fn extract_last_parcel(&'_ mut self) -> (VotesWithSameTransferValue<'a>,PortionOfReasonBeingDoneThisCount) {
        if let Some(last_parcel) = self.last_parcel.take() {
            let key = (S::key(last_parcel.count_index,last_parcel.when_tv_created),last_parcel.transfer_value.clone());
            let (_,votes) = self.by_provenance.get_mut(&key).expect("Last parcel has vanished!");
            let res = votes.extract_last_parcel(last_parcel.prior_state);
            let provenance = PortionOfReasonBeingDoneThisCount{
                transfer_value: Some(last_parcel.transfer_value),
                when_tv_created: last_parcel.when_tv_created,
                papers_came_from_counts: vec![last_parcel.count_index],
            };
            (res,provenance)
        } else {
            panic!("No last parcel");
        }
    }
    /// Extracts all the ballots, adding all with same transfer value together. Sort highest to lowest.
    /// Clears this object.
    pub fn extract_all_ballots_separated_by_transfer_value(&'_ mut self) -> Vec<(TransferValue,(Tally,VotesWithSameTransferValue<'a>,PortionOfReasonBeingDoneThisCount))> {
        let mut helpers : HashMap<TransferValue,MergeVotesHelper<Tally>> = HashMap::default();
        for ((_,tv),(prov,votes)) in self.by_provenance.drain() {
            let helper = helpers.entry(tv.clone()).or_insert_with(||MergeVotesHelper::default());
            helper.add(tv,prov,votes);
        }
        let mut res : Vec<(TransferValue,(Tally,VotesWithSameTransferValue<'a>,PortionOfReasonBeingDoneThisCount))> = helpers.into_iter().map(|(tv,helper)|(tv,helper.extract())).collect();
        res.sort_unstable_by(|(a,_), (b,_)| b.cmp(a)); // sort highest to lowest.
        res
    }
    /// Extracts all the ballots, without doing any merging.
    /// Clears this object.
    /// Sorting will be by  the standard Ord on the key, unless overridden by a custom function.
    pub fn extract_all_ballots_separated_by_key(&'_ mut self,custom_sort:Option<Box<dyn FnMut(&Transcript<Tally>,<S as HowSplitByCountNumber>::KeyToDivide,<S as HowSplitByCountNumber>::KeyToDivide)->Ordering>>,transcript:&Transcript<Tally>) -> Vec<(TransferValue,(Tally,VotesWithSameTransferValue<'a>,PortionOfReasonBeingDoneThisCount))> {
        let mut res = vec![];
        let mut sorted_by_key : Vec<_> = self.by_provenance.drain().collect(); // not sorted yet
        if let Some(mut f) = custom_sort {
            sorted_by_key.sort_by(|((key1,_),_),((key2,_),_)|f(transcript,key1.clone(),key2.clone()));
        } else {
            sorted_by_key.sort_by_key(|((key,_),_)|key.clone()); // now sorted.
        }
        for ((_key,tv),(prov,votes)) in sorted_by_key {
            let provenance = PortionOfReasonBeingDoneThisCount{
                transfer_value: Some(tv.clone()),
                when_tv_created: prov.when_tv_created,
                papers_came_from_counts: prov.counts_comes_from.iter().cloned().collect(),
            };
            res.push((tv.clone(),(prov.tally,votes,provenance)));
        }
        res
    }
    /// Extracts all the ballots, adding all together, ignoring everything but pieces of paper.
    /// Clears this object.
    pub fn extract_all_ballots_ignoring_transfer_value(&'_ mut self) -> (Tally,VotesWithSameTransferValue<'a>,PortionOfReasonBeingDoneThisCount) {
        let mut helper = MergeVotesHelper::default();
        for ((_,tv),(prov,votes)) in self.by_provenance.drain() {
            helper.add(tv,prov,votes);
        }
        helper.extract()
    }

    /// Extracts all the ballots with a given provenance from this key.
    pub fn extract_all_ballots_with_given_provenance(&'_ mut self, key:&'_ (S::KeyToDivide,TransferValue)) -> Option<(PileProvenance<Tally>, VotesWithSameTransferValue<'a>)> {
        self.by_provenance.remove(key)
    }
}

/// A helper for extract_all_ballots_ignoring_transfer_value and extract_all_ballots_separated_by_transfer_value
struct MergeVotesHelper<'a,Tally> {
    tally : Tally,
    sum : Option<VotesWithSameTransferValue<'a>>,
    papers_came_from_counts : CollectAll<CountIndex>,
    transfer_value : DetectUnique<TransferValue>,
    tv_came_from_count : DetectUnique<Option<CountIndex>>,
}

impl <'a,Tally : Zero> Default for MergeVotesHelper<'a,Tally> {
    fn default() -> Self {
        MergeVotesHelper{
            tally: Tally::zero(),
            sum: None,
            papers_came_from_counts: Default::default(),
            transfer_value: Default::default(),
            tv_came_from_count: Default::default()
        }
    }
}
impl <'a,Tally : AddAssign> MergeVotesHelper<'a,Tally> {
    /// add a set of votes to the data structure.
    fn add(&mut self,tv:TransferValue,prov:PileProvenance<Tally>,votes:VotesWithSameTransferValue<'a>) {
        self.tally+=prov.tally;
        self.papers_came_from_counts.extend(prov.counts_comes_from.iter());
        self.transfer_value.add(tv);
        self.tv_came_from_count.add(prov.when_tv_created);
        match &mut self.sum {
            None => { self.sum=Some(votes);  }
            Some(accum) => { accum.add(&votes.votes); }
        }
    }
    fn extract(mut self) -> (Tally,VotesWithSameTransferValue<'a>, PortionOfReasonBeingDoneThisCount) {
        let res = self.sum.unwrap_or_else(||VotesWithSameTransferValue::default());
        let provenance = PortionOfReasonBeingDoneThisCount{
            transfer_value: self.transfer_value.take(),
            when_tv_created: self.tv_came_from_count.take().flatten(),
            papers_came_from_counts: self.papers_came_from_counts.take(),
        };
        (self.tally,res,provenance)
    }
}