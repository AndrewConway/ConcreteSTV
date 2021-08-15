//! This is the real STV algorithm.
//! Unlike IRV, there are many ambiguities in the conceptual description of STV, so parameterized


use num::{Zero};
use crate::election_data::ElectionData;
use crate::ballot_pile::{VotesWithMultipleTransferValues, HowSplitByCountNumber, PartiallyDistributedVote, BallotPaperCount, DistributedVotes, VotesWithSameTransferValue};
use std::collections::{HashSet, VecDeque};
use crate::ballot_metadata::CandidateIndex;
use crate::history::CountIndex;
use crate::transfer_value::{TransferValue, LostToRounding};
use std::ops::{AddAssign, Neg, SubAssign, Sub};
use std::fmt::Display;
use crate::distribution_of_preferences_transcript::{ElectionReason, CandidateElected, TransferValueCreation, TransferValueSource, Transcript, ReasonForCount, PortionOfReasonBeingDoneThisCount, SingleCount, EndCountStatus, PerCandidate, QuotaInfo};
use crate::util::{DetectUnique, CollectAll};

pub trait PreferenceDistributionRules {
    type Tally : Clone+AddAssign+SubAssign+From<usize>+Display+Ord+Sub<Output=Self::Tally>+Zero;
    type SplitByNumber : HowSplitByCountNumber;
    fn make_transfer_value(surplus:Self::Tally,ballots:BallotPaperCount) -> TransferValue;
    fn use_transfer_value(transfer_value:&TransferValue,ballots:BallotPaperCount) -> (Self::Tally,LostToRounding);


}

struct PendingTranscript<Tally> {
    elected : Vec<CandidateElected>,
    not_continuing : Vec<CandidateIndex>,
    created_transfer_value : Option<TransferValueCreation<Tally>>,
}

/// The main workhorse class that does preference distribution.
pub struct PreferenceDistributor<'a,Rules:PreferenceDistributionRules> {
    data : &'a ElectionData,
    original_votes:&'a Vec<PartiallyDistributedVote<'a>>,
    num_candidates : usize,
    candidates_to_be_elected : usize,
    quota : Rules::Tally,
    /// The tally, by candidate.
    tallys : Vec<Rules::Tally>,
    /// the papers that a particular candidate currently has.
    papers : Vec<VotesWithMultipleTransferValues<'a,Rules::SplitByNumber,Rules::Tally>>,
    continuing_candidates : HashSet<CandidateIndex>,
    continuing_candidates_sorted_by_tally : Vec<CandidateIndex>,
    exhausted : BallotPaperCount,
    exhausted_atl : BallotPaperCount,
    tally_lost_to_rounding : Option<Rules::Tally>,
    tally_exhausted : Rules::Tally,
    tally_set_aside : Option<Rules::Tally>,
    current_count : CountIndex,
    pending_surplus_distribution : VecDeque<CandidateIndex>,
    elected_candidates : Vec<CandidateIndex>,

    // information about what is going on in this count.
    in_this_count : PendingTranscript<Rules::Tally>,
    transcript : Transcript<Rules::Tally>,
}

impl <'a,Rules:PreferenceDistributionRules> PreferenceDistributor<'a,Rules>
{
    pub fn new(data : &'a ElectionData,original_votes:&'a Vec<PartiallyDistributedVote<'a>>,candidates_to_be_elected : usize,excluded_candidates:&HashSet<CandidateIndex>) -> Self {
        let num_candidates = data.metadata.candidates.len();
        let tallys = vec![Rules::Tally::zero();num_candidates];
        let mut papers = vec![];
        for _ in 0..num_candidates { papers.push(VotesWithMultipleTransferValues::<'a,Rules::SplitByNumber,Rules::Tally>::default()); }
        let mut continuing_candidates : HashSet<CandidateIndex> = HashSet::default();
        let mut continuing_candidates_sorted_by_tally = vec![];
        for i in 0..num_candidates {
            if !excluded_candidates.contains(&CandidateIndex(i)) {
                continuing_candidates.insert(CandidateIndex(i));
                continuing_candidates_sorted_by_tally.push(CandidateIndex(i));
            }
        }
        PreferenceDistributor{
            data,
            original_votes,
            num_candidates,
            candidates_to_be_elected,
            quota : Rules::Tally::zero(), // dummy until computed.
            tallys,
            papers,
            continuing_candidates,
            continuing_candidates_sorted_by_tally,
            exhausted : BallotPaperCount(0),
            exhausted_atl : BallotPaperCount(0),
            tally_lost_to_rounding: None,
            tally_exhausted: Rules::Tally::zero(),
            tally_set_aside: None,
            current_count : CountIndex(0),
            pending_surplus_distribution : VecDeque::default(),
            elected_candidates : vec![],
            in_this_count : PendingTranscript {
                elected: vec![],
                not_continuing: vec![],
                created_transfer_value: None
            },
            transcript : Transcript {
                quota: QuotaInfo { // dummy values
                    papers: BallotPaperCount(0),
                    vacancies: 0,
                    quota: Rules::Tally::zero(),
                },
                counts: vec![],
                elected: vec![]
            }
        }
    }

    pub fn distribute_first_preferences(& mut self) {
        let distributed = DistributedVotes::distribute(self.original_votes,&self.continuing_candidates,self.num_candidates);
        let mut total_first_preferences : BallotPaperCount = BallotPaperCount(0);
        for i in 0..self.num_candidates {
            let votes = &distributed.by_candidate[i];
            let tally = Rules::Tally::from(votes.num_ballots.0);
            total_first_preferences+=votes.num_ballots;
            self.tallys[i]+=tally.clone();
            self.papers[i].add(votes, TransferValue::one(), self.current_count, Some(self.current_count), tally);
        }
        self.exhausted+=distributed.exhausted;
        self.exhausted_atl+=distributed.exhausted_atl;
        self.compute_quota(total_first_preferences);
        self.end_of_count_step(ReasonForCount::FirstPreferenceCount, PortionOfReasonBeingDoneThisCount {
            transfer_value: None,
            when_tv_created: None,
            papers_came_from_counts: vec![]
        }, true);
    }

    pub fn resort_candidates(&mut self) {
        let tallies = &self.tallys;
        let key = |c:&CandidateIndex|tallies[c.0].clone();
        self.continuing_candidates_sorted_by_tally.sort_by_key(key);
    }

    /// quota = round_down(first_preferences/(1+num_to_elect))+1
    pub fn compute_quota(&mut self,total_first_preferences:BallotPaperCount) {
        self.quota = Rules::Tally::from(total_first_preferences.0/(1+self.candidates_to_be_elected)+1);
        self.transcript.quota = QuotaInfo{
            papers: total_first_preferences,
            vacancies: self.candidates_to_be_elected,
            quota: self.quota.clone(),
        };
        println!("Quota = {}",self.quota);
    }

    pub fn tally(&self,candidate:CandidateIndex) -> Rules::Tally { self.tallys[candidate.0].clone() }

    // declare that a candidate is no longer continuing.
    fn no_longer_continuing(&mut self,candidate:CandidateIndex,used_in_current_count:bool) {
        if !used_in_current_count { self.in_this_count.not_continuing.push(candidate); }
        self.continuing_candidates_sorted_by_tally.retain(|&e|e!=candidate);
        self.continuing_candidates.remove(&candidate);
    }
    fn declare_elected(&mut self,who:CandidateIndex,why:ElectionReason) {
        self.in_this_count.elected.push(CandidateElected{who,why});
        println!("Elected {}",self.data.metadata.candidate(who).name);
        self.elected_candidates.push(who);
        self.transcript.elected.push(who);
        self.no_longer_continuing(who,true);
    }

    pub fn check_elected_by_quota(&mut self) {
        let elected_by_quota : Vec<CandidateIndex> = self.continuing_candidates_sorted_by_tally.iter().rev().take_while(|&&c|self.tally(c)>self.quota).cloned().collect();
        // TODO check for ties and do countbacks. See AEC rules 21-23
        for c in elected_by_quota {
            self.declare_elected(c,ElectionReason::ReachedQuota);
            if self.tally(c)>self.quota { self.pending_surplus_distribution.push_back(c); }
        }
    }

    pub fn remaining_to_elect(&self) -> usize { self.candidates_to_be_elected-self.elected_candidates.len() }

    /// federal rule 17
    /// > (17) In respect of the last vacancy for which two continuing candidates
    /// > remain, the continuing candidate who has the larger number of
    /// > votes shall be elected notwithstanding that that number is below
    /// > the quota, and if those candidates have an equal number of votes
    /// > the Australian Electoral Officer for the State shall have a casting
    /// > vote but shall not otherwise vote at the election.
    pub fn check_elected_by_highest_of_remaining_2_when_1_needed(&mut self) {
        if self.continuing_candidates_sorted_by_tally.len()==2 && self.remaining_to_elect()==1 {
            // elect the highest, Electoral officer resolved ties.
            let candidate1 = self.continuing_candidates_sorted_by_tally[0];
            let candidate2 = self.continuing_candidates_sorted_by_tally[1];
            if self.tally(candidate1)==self.tally(candidate2) {
                // TODO mark that this is a decision by the EC.
            }
            self.declare_elected(candidate1,ElectionReason::HighestOfLastTwoStanding);
        }
    }

    /// federal rule 18
    /// > (18) Notwithstanding any other provision of this section, where the
    /// > number of continuing candidates is equal to the number of
    /// > remaining unfilled vacancies, those candidates shall be elected.
    pub fn check_if_should_elect_all_remaining(&mut self) {
        if self.continuing_candidates_sorted_by_tally.len()==self.remaining_to_elect() {
            let elected_group = self.continuing_candidates_sorted_by_tally.clone();
            // TODO check for ties and do countbacks. See AEC rules 21-23
            for c in elected_group {
                self.declare_elected(c,ElectionReason::AllRemainingMustBeElected);
            }
        }
    }

    pub fn check_elected(&mut self) {
        self.check_elected_by_quota();
        self.check_elected_by_highest_of_remaining_2_when_1_needed();
        self.check_if_should_elect_all_remaining();
    }

    pub fn end_of_count_step(&mut self,reason : ReasonForCount,portion : PortionOfReasonBeingDoneThisCount,reason_completed : bool) {
        self.resort_candidates();
        self.check_elected();
        self.print_tallys();
        self.current_count=CountIndex(self.current_count.0+1);
        self.transcript.counts.push(SingleCount{
            reason,
            portion,
            reason_completed,
            elected: self.in_this_count.elected.clone(),
            not_continuing: self.in_this_count.not_continuing.clone(),
            created_transfer_value: self.in_this_count.created_transfer_value.take(),
            status: EndCountStatus {
                tallies: PerCandidate {
                    candidate: self.tallys.clone(),
                    exhausted: self.tally_exhausted.clone(),
                    rounding:  self.tally_lost_to_rounding.clone(),
                    set_aside: self.tally_set_aside.clone(),
                },
                papers: PerCandidate {
                    candidate: self.papers.iter().map(|p|p.num_ballots()).collect(),
                    exhausted: self.exhausted,
                    rounding: None,
                    set_aside: None
                },
                atl_papers: Some(PerCandidate {
                    candidate: self.papers.iter().map(|p|p.num_atl_ballots()).collect(),
                    exhausted: self.exhausted_atl,
                    rounding: None,
                    set_aside: None
                }),
            }
        });
        self.in_this_count.not_continuing=self.in_this_count.elected.drain(..).map(|e|e.who).collect();
    }

    /// Transfer votes using a single transfer value. Used for Federal and Victoria
    ///
    /// Federal Legislation:
    /// > (9) Unless all the vacancies have been filled, the number (if any) of
    /// > votes in excess of the quota (in this section referred to as surplus
    /// > votes) of each elected candidate shall be transferred to the
    /// > continuing candidates as follows:
    /// > (a) the number of surplus votes of the elected candidate shall be
    /// > divided by the number of first preference votes received by
    /// > the candidate and the resulting fraction shall be the transfer
    /// > value;
    /// > (b) the total number of ballot papers of the elected candidate that
    /// > express the first preference vote for that candidate and the
    /// > next available preference for a particular continuing
    /// > candidate shall be multiplied by the transfer value, the
    /// > number so obtained (disregarding any fraction) shall be
    /// > added to the number of first preference votes of the
    /// > continuing candidate and all those ballot papers shall be
    /// > transferred to the continuing candidate;
    pub fn distribute_surplus_all_with_same_transfer_value(&mut self,candidate_to_distribute:CandidateIndex) -> PortionOfReasonBeingDoneThisCount {
        let votes : Rules::Tally = self.tally(candidate_to_distribute);
        let surplus: Rules::Tally  = votes.clone()-self.quota.clone();
        self.tallys[candidate_to_distribute.0]=self.quota.clone();
        let (ballots,provenance) = self.papers[candidate_to_distribute.0].extract_all_ballots_ignoring_transfer_value();
        let ballots_considered : BallotPaperCount = ballots.num_ballots;
        let transfer_value : TransferValue = Rules::make_transfer_value(surplus.clone(),ballots.num_ballots);
        let exhausted : BallotPaperCount = self.parcel_out_votes_with_given_transfer_value(transfer_value.clone(),ballots,Some(self.current_count));
        let continuing_ballots = ballots_considered-exhausted;
        self.in_this_count.created_transfer_value=Some(TransferValueCreation{
            surplus,
            votes,
            original_transfer_value: None,
            ballots_considered,
            continuing_ballots,
            transfer_value,
            source: TransferValueSource::SurplusOverBallots
        });
        provenance
    }

    /// Parcel out votes by next continuing candidate with a given transfer value.
    /// Return the number of exhausted votes.
    pub fn parcel_out_votes_with_given_transfer_value(&mut self,transfer_value:TransferValue,ballots:VotesWithSameTransferValue<'a>,when_tv_created:Option<CountIndex>) -> BallotPaperCount {
        let distributed = DistributedVotes::distribute(&ballots.votes,&self.continuing_candidates,self.num_candidates);
        for (candidate_index,candidate_ballots) in distributed.by_candidate.into_iter().enumerate() {
            if candidate_ballots.num_ballots.0>0 {
                let (worth ,_lost_to_rounding) = Rules::use_transfer_value(&transfer_value,candidate_ballots.num_ballots);
                self.tallys[candidate_index]+=worth.clone();
                self.papers[candidate_index].add(&candidate_ballots, transfer_value.clone(), self.current_count, when_tv_created, worth);
            }
        }
        distributed.exhausted
    }

    pub fn distribute_surplus(&mut self,candidate_to_distribute:CandidateIndex) {
        println!("Distributing surplus for {}",self.data.metadata.candidate(candidate_to_distribute).name);
        let provenance = self.distribute_surplus_all_with_same_transfer_value(candidate_to_distribute);
        self.end_of_count_step(ReasonForCount::ExcessDistribution(candidate_to_distribute), provenance, true);
    }

    pub fn print_candidates_names(&self) {
        println!("{}",self.data.metadata.candidates.iter().map(|c|c.name.clone()).collect::<Vec<String>>().join("\t")+"\tExhausted");
    }
    pub fn print_tallys(&self) {
        println!("{}",self.tallys.iter().map(|t|t.to_string()).collect::<Vec<String>>().join("\t")+"\t"+&self.exhausted.0.to_string());
    }

    pub fn get_lowest_candidate(&self) -> Vec<CandidateIndex> {
        // TODO check for ties.
        vec![*self.continuing_candidates_sorted_by_tally.first().unwrap()]
    }

    /// Federal legislation:
    /// > (13AA) Where a candidate is, or candidates are, excluded in accordance
    /// > with this section, the ballot papers of the excluded candidate or
    /// > candidates must be transferred as follows:
    /// > (a) the total number of ballot papers:
    /// > (i) expressing a first preference for an excluded candidate;
    /// > or
    /// > (ii) received by an excluded candidate on distribution from
    /// > another excluded candidate at a transfer value of 1 vote;
    /// > being ballot papers expressing the next available preference
    /// > for a particular continuing candidate must be transferred at a
    /// > transfer value of 1 vote to the continuing candidate and added
    /// > to the number of votes of the continuing candidate;
    /// > (b) the total number (if any) of other ballot papers obtained by an
    /// > excluded candidate or the excluded candidates, as the case
    /// > may be, must be transferred beginning with the ballot papers
    /// > received by that candidate or those candidates at the highest
    /// > transfer value and ending with the ballot papers received at
    /// > the lowest transfer value, as follows:
    /// > (i) the total number of ballot papers received by the
    /// > excluded candidate or candidates, as the case may be, at
    /// > a particular transfer value and expressing the next
    /// > available preference for a particular continuing
    /// > candidate must be multiplied by that transfer value;
    /// > (ii) the number so obtained (disregarding any fraction) must
    /// > be added to the number of votes of the continuing
    /// > candidate;
    /// > (iii) all those ballot papers must be transferred to the
    /// > continuing candidate.
    pub fn eliminate(&mut self,candidates_to_eliminate:Vec<CandidateIndex>) {
        let mut provenances : HashSet<(<Rules::SplitByNumber as HowSplitByCountNumber>::KeyToDivide,TransferValue)> = HashSet::default();
        for &candidate in &candidates_to_eliminate {
            println!("Excluding {}",self.data.metadata.candidate(candidate).name);
            self.no_longer_continuing(candidate,false);
            for prov in self.papers[candidate.0].get_all_provenance_keys() {
                provenances.insert(prov);
            }
        }
        // TODO check for interrupt at this point.
        let mut provenances : Vec<(<Rules::SplitByNumber as HowSplitByCountNumber>::KeyToDivide,TransferValue)> = provenances.into_iter().collect();
        // TODO sort by S::KeyToDivide
        provenances.sort_by_key(|f|f.1.0.clone().neg()); // stable sort, will preserve ordering of other key stull
        let mut togo = provenances.len();
        for key in provenances {
            // doing the transfer for this key.
            let mut all_votes = VotesWithSameTransferValue::default();
            let mut when_tv_created = DetectUnique::<Option<CountIndex>>::default();
            let mut papers_came_from_counts = CollectAll::<CountIndex>::default();
            for &candidate in &candidates_to_eliminate {
                if let Some((from,votes)) = self.papers[candidate.0].extract_all_ballots_with_given_provenance(&key) {
                    when_tv_created.add(from.when_tv_created);
                    papers_came_from_counts.extend(from.counts_comes_from.into_iter());
                    self.tallys[candidate.0]-=from.tally;
                    if all_votes.num_ballots.0==0 { all_votes=votes; }
                    else { all_votes.add(&votes.votes) }
                }
            }
            let when_tv_created=when_tv_created.take().flatten();
            self.parcel_out_votes_with_given_transfer_value(key.1.clone(),all_votes,when_tv_created);
            togo-=1;
            self.end_of_count_step(ReasonForCount::Elimination(candidates_to_eliminate.clone()), PortionOfReasonBeingDoneThisCount {
                transfer_value: Some(key.1),
                when_tv_created,
                papers_came_from_counts: papers_came_from_counts.take(),
            }, togo==0);
        }
    }


    pub fn distribute_lowest(&mut self) {
        // TODO do the absurd Federal 13(A) multiple elimination
        let candidates_to_remove = self.get_lowest_candidate();
        self.eliminate(candidates_to_remove);
    }
    pub fn go(&mut self) {
        self.print_candidates_names();
        self.distribute_first_preferences();
        while self.remaining_to_elect()>0 {
            if let Some(candidate) = self.pending_surplus_distribution.pop_front() {
                self.distribute_surplus(candidate);
            } else {
                self.distribute_lowest();
            }
        }
    }
}

pub fn distribute_preferences<Rules:PreferenceDistributionRules>(data:&ElectionData,candidates_to_be_elected : usize,excluded_candidates:&HashSet<CandidateIndex>) -> Transcript<Rules::Tally> {
    let arena = typed_arena::Arena::<CandidateIndex>::new();
    let votes = data.resolve_atl(&arena);
    let mut work : PreferenceDistributor<'_,Rules> = PreferenceDistributor::new(data,&votes,candidates_to_be_elected,excluded_candidates);
    work.go();
    work.transcript
}