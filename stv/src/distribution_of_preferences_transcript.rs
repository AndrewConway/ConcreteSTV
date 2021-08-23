//! Store the history of the distribution of preferences


use crate::ballot_pile::BallotPaperCount;
use crate::ballot_metadata::{CandidateIndex, ElectionMetadata, NumberOfCandidates};
use crate::transfer_value::TransferValue;
use serde::{Serialize,Deserialize};



/// The index of a count. 0 means the first. This is different from the human readable
/// count, which may be more complex and have sub-counts as well.
#[derive(Copy,Clone,Debug,Ord, PartialOrd, Eq, PartialEq,Hash,Serialize,Deserialize)]
pub struct CountIndex(pub(crate) usize);

/// A value that is primarily per candidate, but may also go to some other source.
/// Generally, this is used for preserved properties such that the sum over all candidates and other destinations is always the same.
/// For instance, ballots, which start out all assigned to candidates, are shifted around between people, but some will get exhausted.
/// Alternatively, votes, which start out all assigned to candidates, but may get lost due to rounding or weird rules or exhaustion.
#[derive(Clone,Serialize,Deserialize)]
pub struct PerCandidate<X> {
    /// the value for a given candidate.
    pub candidate : Vec<X>,
    /// something is exhausted if it can't go to a specific candidate as there are not enough preferences on a particular ballot.
    pub exhausted : X,
    /// something goes to rounding if it can't go to a specific candidate as fractions are not allowed.
    pub rounding : X,
    /// something gets set aside if some feature of the the rules means it doesn't go to a particular candidate. None if not applicable.
    pub set_aside : Option<X>,
}

impl <X:Default> Default for PerCandidate<X> {
    fn default() -> Self {
        PerCandidate{
            candidate: vec![],
            exhausted: X::default(),
            rounding: X::default(),
            set_aside: None
        }
    }
}
/// Record the status of the count at the end of the count.
#[derive(Clone,Serialize,Deserialize)]
pub struct EndCountStatus<Tally> {
    /// tallies for each candidate
    pub tallies : PerCandidate<Tally>,
    /// the number of pieces of paper for each candidate
    pub papers : PerCandidate<BallotPaperCount>,
    /// the number of above pieces of paper that are ATL.
    pub atl_papers : Option<PerCandidate<BallotPaperCount>>,
}

#[derive(Clone,Serialize,Deserialize)]
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
}

#[derive(Copy, Clone,Serialize,Deserialize)]
pub enum ElectionReason {
    ReachedQuota,
    HighestOfLastTwoStanding,
    AllRemainingMustBeElected,
}

#[derive(Copy, Clone,Serialize,Deserialize)]
pub struct CandidateElected {
    pub who : CandidateIndex,
    pub why : ElectionReason,
}

#[derive(Clone,Serialize,Deserialize)]
pub struct PortionOfReasonBeingDoneThisCount {
    pub transfer_value : Option<TransferValue>,
    pub when_tv_created: Option<CountIndex>,
    pub papers_came_from_counts : Vec<CountIndex>,
}

#[derive(Copy,Clone,Serialize,Deserialize)]
pub enum TransferValueSource {
    SurplusOverBallots,
    SurplusOverContinuingBallots,
    SurplusOverVotesTimesOriginalTransfer,
    Limited,
}

#[derive(Clone,Serialize,Deserialize)]
pub struct TransferValueCreation<Tally> {
    pub surplus : Tally,
    pub votes : Tally,
    pub original_transfer_value : Option<TransferValue>,
    /// The number of ballots considered for redistribution. This may be all or a last parcel.
    pub ballots_considered : BallotPaperCount,
    /// The number of the considered ballots that are continuing
    pub continuing_ballots : BallotPaperCount,
    pub transfer_value : TransferValue,
    pub source : TransferValueSource,
}

/// Sometimes the Electoral Commission needs to make a decision, such as tie resolution.
/// Sometimes legislation mandates this be random, sometimes the returning officer.
/// Regardless, this records that the decision needs to be made.
#[derive(Clone,Serialize,Deserialize)]
pub struct DecisionMadeByEC {
    pub affected : Vec<CandidateIndex>
}

#[derive(Clone,Serialize,Deserialize)]
pub struct SingleCount<Tally> {
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
    /// whether the EC needs to make any decisions
    pub decisions : Vec<DecisionMadeByEC>,
    /// status at end of count.
    pub status : EndCountStatus<Tally>
}

#[derive(Clone,Serialize,Deserialize)]
pub struct QuotaInfo<Tally> {
    pub papers : BallotPaperCount,
    pub vacancies : NumberOfCandidates,
    pub quota : Tally,
}

#[derive(Clone,Serialize,Deserialize)]
pub struct Transcript<Tally> {
    pub quota : QuotaInfo<Tally>,
    pub counts : Vec<SingleCount<Tally>>,
    pub elected : Vec<CandidateIndex>,
}

#[derive(Clone,Serialize,Deserialize)]
pub struct TranscriptWithMetadata<Tally> {
    pub metadata : ElectionMetadata,
    pub transcript : Transcript<Tally>,
}


