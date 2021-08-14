//! Things to do with piles of ballots.



use crate::ballot_metadata::CandidateIndex;
use crate::ballot_paper::VoteSource;
use std::collections::{HashSet, HashMap};
use crate::history::CountIndex;
use crate::transfer_value::TransferValue;
use num::{Zero};
use std::ops::AddAssign;
use serde::Deserialize;
use serde::Serialize;
use std::hash::Hash;

/// A number representing a count of pieces of paper.
/// This is distinct from votes which may be fractional in the presence of weights.
#[derive(Copy,Clone,Eq, PartialEq,Debug,Serialize,Deserialize)]
pub struct BallotPaperCount(usize);

impl AddAssign for BallotPaperCount {
    fn add_assign(&mut self, rhs: Self) { self.0+=rhs.0; }
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
    pub fn next(&self,continuing:HashSet<CandidateIndex>) -> Option<Self> {
        for i in self.upto+1 .. self.prefs.len() {
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

impl <'a> VotesWithSameTransferValue<'a> {
    // number of below the line ballots in this pile
    pub fn num_btl_ballots(&self) -> BallotPaperCount {  BallotPaperCount(self.num_ballots.0-self.num_atl_ballots.0)  }

    pub fn add_vote(&mut self,vote : PartiallyDistributedVote<'a>) {
        self.num_ballots+=vote.n;
        if vote.is_atl() { self.num_atl_ballots+=vote.n; }
        self.votes.push(vote);
    }
    pub fn add(&mut self,votes:&Vec<PartiallyDistributedVote<'a>>) {
        for v in votes {
            self.add_vote(*v);
        }
    }
}

/// Different jurisdictions split up parcels of shares by their provenence in different ways. This abstracts that.
pub trait HowSplitByCountNumber {
    type KeyToDivide : Eq+Hash;
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




/// A set of votes potentially with multiple transfer values or sources.
/// These would typically be the votes given to a particular individual.
pub struct VotesWithMultipleTransferValues<'a,S:HowSplitByCountNumber,Tally> {

    by_provenance : HashMap<(S::KeyToDivide,TransferValue),(PileProvenance<Tally>,VotesWithSameTransferValue<'a>)>

}

impl <'a,S:HowSplitByCountNumber,Tally:AddAssign+Zero> VotesWithMultipleTransferValues<'a,S,Tally> {
    pub fn add(&'a mut self,votes:VotesWithSameTransferValue<'a>,transfer_value:TransferValue,count_index:CountIndex,when_tv_created:Option<CountIndex>,tally:Tally) {
        let key = (S::key(count_index,when_tv_created),transfer_value.clone());
        let entry = self.by_provenance.entry(key).or_insert_with(||
            (PileProvenance{ counts_comes_from: Default::default(),when_tv_created,tally:Tally::zero()}, VotesWithSameTransferValue::default()));
        entry.0.add(count_index,when_tv_created,tally);
        entry.1.add(&votes.votes)
    }
}