//! Things to do with piles of ballots.



use crate::ballot_metadata::CandidateIndex;
use crate::ballot_paper::VoteSource;
use std::collections::{HashSet, HashMap};
use crate::history::CountIndex;
use crate::transfer_value::TransferValue;
use num::{Zero};
use std::ops::{AddAssign, Sub, Add};
use serde::Deserialize;
use serde::Serialize;
use std::hash::Hash;
use crate::distribution_of_preferences_transcript::PortionOfReasonBeingDoneThisCount;
use crate::util::{DetectUnique, CollectAll};

/// A number representing a count of pieces of paper.
/// This is distinct from votes which may be fractional in the presence of weights.
#[derive(Copy,Clone,Eq, PartialEq,Debug,Serialize,Deserialize)]
pub struct BallotPaperCount(pub usize);

impl AddAssign for BallotPaperCount {
    fn add_assign(&mut self, rhs: Self) { self.0+=rhs.0; }
}

impl Sub for BallotPaperCount {
    type Output = BallotPaperCount;
    fn sub(self, rhs: Self) -> Self::Output { BallotPaperCount(self.0-rhs.0) }
}

impl Add for BallotPaperCount {
    type Output = BallotPaperCount;
    fn add(self, rhs: Self) -> Self::Output { BallotPaperCount(self.0+rhs.0) }
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
    type KeyToDivide : Eq+Hash+Clone;
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

impl <'a,S:HowSplitByCountNumber,Tally> Default for VotesWithMultipleTransferValues<'a,S,Tally> {
    fn default() -> Self {
        VotesWithMultipleTransferValues{ by_provenance: HashMap::default() }
    }
}

impl <'a,S:HowSplitByCountNumber,Tally:AddAssign+Zero> VotesWithMultipleTransferValues<'a,S,Tally> {
    pub fn add(& mut self,votes:&'_ VotesWithSameTransferValue<'a>,transfer_value:TransferValue,count_index:CountIndex,when_tv_created:Option<CountIndex>,tally:Tally) {
        let key = (S::key(count_index,when_tv_created),transfer_value.clone());
        let entry = self.by_provenance.entry(key).or_insert_with(||
            (PileProvenance{ counts_comes_from: Default::default(),when_tv_created,tally:Tally::zero()}, VotesWithSameTransferValue::default()));
        entry.0.add(count_index,when_tv_created,tally);
        entry.1.add(&votes.votes)
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


    /// Extracts all the ballots, adding all together, ignoring everything but pieces of paper.
    /// Clears this object.
    pub fn extract_all_ballots_ignoring_transfer_value(&'_ mut self) -> (VotesWithSameTransferValue<'a>,PortionOfReasonBeingDoneThisCount) {
        let mut sum : Option<VotesWithSameTransferValue> = None;
        let mut papers_came_from_counts = CollectAll::<CountIndex>::default();
        let mut transfer_value = DetectUnique::<TransferValue>::default();
        let mut tv_came_from_count = DetectUnique::<Option<CountIndex>>::default();
        for ((_,tv),(prov,votes)) in self.by_provenance.drain() {
            papers_came_from_counts.extend(prov.counts_comes_from.iter());
            transfer_value.add(tv);
            tv_came_from_count.add(prov.when_tv_created);
            match &mut sum {
                None => { sum=Some(votes);  }
                Some(accum) => accum.add(&votes.votes),
            }
        }
        let res = sum.unwrap_or_else(||VotesWithSameTransferValue::default());
        let provenance = PortionOfReasonBeingDoneThisCount{
            transfer_value: transfer_value.take(),
            when_tv_created: tv_came_from_count.take().flatten(),
            papers_came_from_counts: papers_came_from_counts.take(),
        };
        (res,provenance)
    }

    /// Extracts all the ballots with a given provenance from this key.
    pub fn extract_all_ballots_with_given_provenance(&'_ mut self, key:&'_ (S::KeyToDivide,TransferValue)) -> Option<(PileProvenance<Tally>, VotesWithSameTransferValue<'a>)> {
        self.by_provenance.remove(key)
    }
}