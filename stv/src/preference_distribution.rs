//! This is the real STV algorithm.
//! Unlike IRV, there are many ambiguities in the conceptual description of STV, so parameterized


use num::Num;
use crate::election_data::ElectionData;
use crate::ballot_pile::{VotesWithMultipleTransferValues, HowSplitByCountNumber, PartiallyDistributedVote, BallotPaperCount, DistributedVotes, VotesWithSameTransferValue};
use std::collections::{HashSet, VecDeque};
use crate::ballot_metadata::CandidateIndex;
use crate::history::CountIndex;
use crate::transfer_value::{TransferValue, LostToRounding};
use std::ops::{AddAssign, Neg, SubAssign};
use std::fmt::Display;

pub trait PreferenceDistributionRules<Tally> {
    fn make_transfer_value(surplus:Tally,ballots:BallotPaperCount) -> TransferValue;
    fn use_transfer_value(transfer_value:&TransferValue,ballots:BallotPaperCount) -> (Tally,LostToRounding);
}

/// The main workhorse class that does preference distribution.
pub struct PreferenceDistributor<'a,S:HowSplitByCountNumber,Tally:Num> {
    data : &'a ElectionData,
    original_votes:&'a Vec<PartiallyDistributedVote<'a>>,
    num_candidates : usize,
    candidates_to_be_elected : usize,
    quota : Tally,
    /// The tally, by candidate.
    tallys : Vec<Tally>,
    /// the papers that a particular candidate currently has.
    papers : Vec<VotesWithMultipleTransferValues<'a,S,Tally>>,
    continuing_candidates : HashSet<CandidateIndex>,
    continuing_candidates_sorted_by_tally : Vec<CandidateIndex>,
    exhausted : BallotPaperCount,
    current_count : CountIndex,
    pending_surplus_distribution : VecDeque<CandidateIndex>,
    elected_candidates : Vec<CandidateIndex>,
}

impl <'a,S:HowSplitByCountNumber,Tally:Num+Clone+AddAssign+SubAssign+From<usize>+Display+Ord> PreferenceDistributor<'a,S,Tally>
{
    pub fn new(data : &'a ElectionData,original_votes:&'a Vec<PartiallyDistributedVote<'a>>,candidates_to_be_elected : usize,excluded_candidates:&HashSet<CandidateIndex>) -> Self {
        let num_candidates = data.metadata.candidates.len();
        let tallys = vec![Tally::zero();num_candidates];
        let mut papers = vec![];
        for _ in 0..num_candidates { papers.push(VotesWithMultipleTransferValues::<'a,S,Tally>::default()); }
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
            quota : Tally::zero(), // dummy until computed.
            tallys,
            papers,
            continuing_candidates,
            continuing_candidates_sorted_by_tally,
            exhausted : BallotPaperCount(0),
            current_count : CountIndex(0),
            pending_surplus_distribution : VecDeque::default(),
            elected_candidates : vec![],
        }
    }

    pub fn distribute_first_preferences(& mut self) {
        let distributed = DistributedVotes::distribute(self.original_votes,&self.continuing_candidates,self.num_candidates);
        let mut total_first_preferences : BallotPaperCount = BallotPaperCount(0);
        for i in 0..self.num_candidates {
            let votes = &distributed.by_candidate[i];
            let tally = Tally::from(votes.num_ballots.0);
            total_first_preferences+=votes.num_ballots;
            self.tallys[i]+=tally.clone();
            self.papers[i].add(votes, TransferValue::one(), self.current_count, Some(self.current_count), tally);
        }
        self.exhausted+=distributed.exhausted;
        self.compute_quota(total_first_preferences);
        self.end_of_count_step();
    }

    pub fn resort_candidates(&mut self) {
        let tallies = &self.tallys;
        let key = |c:&CandidateIndex|tallies[c.0].clone();
        self.continuing_candidates_sorted_by_tally.sort_by_key(key);
    }

    /// quota = round_down(first_preferences/(1+num_to_elect))+1
    pub fn compute_quota(&mut self,total_first_preferences:BallotPaperCount) {
        self.quota = Tally::from(total_first_preferences.0/(1+self.candidates_to_be_elected)+1);
        println!("Quota = {}",self.quota);
    }

    pub fn tally(&self,candidate:CandidateIndex) -> Tally { self.tallys[candidate.0].clone() }

    // declare that a candidate is no longer continuing.
    fn no_longer_continuing(&mut self,candidate:CandidateIndex) {
        self.continuing_candidates_sorted_by_tally.retain(|&e|e!=candidate);
        self.continuing_candidates.remove(&candidate);
    }
    fn declare_elected(&mut self,candidate:CandidateIndex) {
        println!("Elected {}",self.data.metadata.candidate(candidate).name);
        self.elected_candidates.push(candidate);
        self.no_longer_continuing(candidate);
    }

    pub fn check_elected_by_quota(&mut self) {
        let elected_by_quota : Vec<CandidateIndex> = self.continuing_candidates_sorted_by_tally.iter().rev().take_while(|&&c|self.tally(c)>self.quota).cloned().collect();
        // TODO check for ties and do countbacks. See AEC rules 21-23
        for c in elected_by_quota {
            self.declare_elected(c);
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
            self.declare_elected(candidate1);
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
                self.declare_elected(c);
            }
        }
    }

    pub fn check_elected(&mut self) {
        self.check_elected_by_quota();
        self.check_elected_by_highest_of_remaining_2_when_1_needed();
        self.check_if_should_elect_all_remaining();
    }

    pub fn end_of_count_step(&mut self) {
        self.resort_candidates();
        self.check_elected();
        self.print_tallys();
        self.current_count=CountIndex(self.current_count.0+1);
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
    pub fn distribute_surplus_all_with_same_transfer_value<Rules:PreferenceDistributionRules<Tally>>(&mut self,candidate_to_distribute:CandidateIndex) {
        let surplus : Tally = self.tally(candidate_to_distribute)-self.quota.clone();
        self.tallys[candidate_to_distribute.0]=self.quota.clone();
        let ballots : VotesWithSameTransferValue<'a> = self.papers[candidate_to_distribute.0].extract_all_ballots_ignoring_transfer_value();
        let transfer_value : TransferValue = Rules::make_transfer_value(surplus,ballots.num_ballots);
        self.parcel_out_votes_with_give_transfer_value::<Rules>(transfer_value,ballots,Some(self.current_count));
    }

    pub fn parcel_out_votes_with_give_transfer_value<Rules:PreferenceDistributionRules<Tally>>(&mut self,transfer_value:TransferValue,ballots:VotesWithSameTransferValue<'a>,when_tv_created:Option<CountIndex>) {
        let distributed = DistributedVotes::distribute(&ballots.votes,&self.continuing_candidates,self.num_candidates);
        for (candidate_index,candidate_ballots) in distributed.by_candidate.into_iter().enumerate() {
            if candidate_ballots.num_ballots.0>0 {
                let (worth ,_lost_to_rounding) = Rules::use_transfer_value(&transfer_value,candidate_ballots.num_ballots);
                self.tallys[candidate_index]+=worth.clone();
                self.papers[candidate_index].add(&candidate_ballots, transfer_value.clone(), self.current_count, when_tv_created, worth);
            }
        }
    }

    pub fn distribute_surplus<Rules:PreferenceDistributionRules<Tally>>(&mut self,candidate_to_distribute:CandidateIndex) {
        println!("Distributing surplus for {}",self.data.metadata.candidate(candidate_to_distribute).name);
        self.distribute_surplus_all_with_same_transfer_value::<Rules>(candidate_to_distribute);
        self.end_of_count_step();
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
    pub fn eliminate<Rules:PreferenceDistributionRules<Tally>>(&mut self,candidates_to_eliminate:Vec<CandidateIndex>) {
        let mut provenances : HashSet<(S::KeyToDivide,TransferValue)> = HashSet::default();
        for &candidate in &candidates_to_eliminate {
            println!("Excluding {}",self.data.metadata.candidate(candidate).name);
            self.no_longer_continuing(candidate);
            for prov in self.papers[candidate.0].get_all_provenance_keys() {
                provenances.insert(prov);
            }
        }
        // TODO check for interrupt at this point.
        let mut provenances : Vec<(S::KeyToDivide,TransferValue)> = provenances.into_iter().collect();
        // TODO sort by S::KeyToDivide
        provenances.sort_by_key(|f|f.1.0.clone().neg()); // stable sort, will preserve ordering of other key stull
        for key in provenances {
            // doing the transfer for this key.
            let mut all_votes = VotesWithSameTransferValue::default();
            for &candidate in &candidates_to_eliminate {
                if let Some((from,votes)) = self.papers[candidate.0].extract_all_ballots_with_given_provenance(&key) {
                    self.tallys[candidate.0]-=from.tally;
                    if all_votes.num_ballots.0==0 { all_votes=votes; }
                    else { all_votes.add(&votes.votes) }
                }
            }
            self.parcel_out_votes_with_give_transfer_value::<Rules>(key.1,all_votes,None); // TODO do better with when_tv_created
            self.end_of_count_step();
        }
    }


    pub fn distribute_lowest<Rules:PreferenceDistributionRules<Tally>>(&mut self) {
        // TODO do the absurd Federal 13(A) multiple elimination
        let candidates_to_remove = self.get_lowest_candidate();
        self.eliminate::<Rules>(candidates_to_remove);
    }
    pub fn go<Rules:PreferenceDistributionRules<Tally>>(&mut self) {
        self.print_candidates_names();
        self.distribute_first_preferences();
        while self.remaining_to_elect()>0 {
            if let Some(candidate) = self.pending_surplus_distribution.pop_front() {
                self.distribute_surplus::<Rules>(candidate);
            } else {
                self.distribute_lowest::<Rules>();
            }
        }
    }
}

pub fn distribute_preferences<S:HowSplitByCountNumber,Tally:Num+Clone+AddAssign+SubAssign+From<usize>+Display+Ord,Rules:PreferenceDistributionRules<Tally>>(data:&ElectionData,candidates_to_be_elected : usize,excluded_candidates:&HashSet<CandidateIndex>) {
    let arena = typed_arena::Arena::<CandidateIndex>::new();
    let votes = data.resolve_atl(&arena);
    let mut work : PreferenceDistributor<'_, S, Tally> = PreferenceDistributor::new(data,&votes,candidates_to_be_elected,excluded_candidates);
    work.go::<Rules>();
}