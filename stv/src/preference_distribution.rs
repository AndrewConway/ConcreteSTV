//! This is the real STV algorithm.
//! Unlike IRV, there are many ambiguities in the conceptual description of STV, so parameterized


use num::{Zero};
use crate::election_data::ElectionData;
use crate::ballot_pile::{VotesWithMultipleTransferValues, HowSplitByCountNumber, PartiallyDistributedVote, BallotPaperCount, DistributedVotes, VotesWithSameTransferValue};
use std::collections::{HashSet, VecDeque};
use crate::ballot_metadata::CandidateIndex;
use crate::history::CountIndex;
use crate::transfer_value::{TransferValue, LostToRounding};
use std::ops::{AddAssign, Neg, SubAssign, Sub, Range};
use std::fmt::Display;
use crate::distribution_of_preferences_transcript::{ElectionReason, CandidateElected, TransferValueCreation, TransferValueSource, Transcript, ReasonForCount, PortionOfReasonBeingDoneThisCount, SingleCount, EndCountStatus, PerCandidate, QuotaInfo, DecisionMadeByEC};
use crate::util::{DetectUnique, CollectAll};
use crate::tie_resolution::{MethodOfTieResolution, TieResolutionsMadeByEC};
use std::hash::Hash;
use std::iter::Sum;
use std::cmp::min;

/// Many systems have a special rules for termination when there are a small number of
/// candidates left (e.g. equal to the number of if there are exactly 2 candidates left
/// and 1 vacancy. This can be done at a variety of times.
#[derive(Copy, Clone,Debug,Eq, PartialEq)]
pub enum WhenToDoElectCandidateClauseChecking {
    /// Don't do this type of check
    Never,
    /// Check quota, and apply to remaining candidates.
    AfterCheckingQuota,
    /// If there is not undistributed surplus, and there is not an ongoing elimination with more papers to distribute. See Federal 2019 QLD and VIC.
    AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing,
    /// If there is not undistributed surplus. See Federal 2016 NSW, WA, QLD and VIC.
    AfterCheckingQuotaIfNoUndistributedSurplusExists,
    /// If there is not undistributed surplus, and there is not an ongoing elimination with more papers to distribute. See Federal 2019 QLD and VIC.
    AfterCheckingQuotaIfExclusionNotOngoing,
    /// If the distribution of papers should be interrupted by this.
    AfterDeterminingWhoToExcludeButBeforeTransferringAnyPapers,
}
pub trait PreferenceDistributionRules {
    type Tally : Clone+AddAssign+SubAssign+From<usize>+Display+Ord+Sub<Output=Self::Tally>+Zero+Hash+Sum<Self::Tally>;
    type SplitByNumber : HowSplitByCountNumber;
    fn make_transfer_value(surplus:Self::Tally,ballots:BallotPaperCount) -> TransferValue;
    fn use_transfer_value(transfer_value:&TransferValue,ballots:BallotPaperCount) -> (Self::Tally,LostToRounding);

    // ***  Tie resolution issues ***

    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution;
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution;
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution;
    // Note that it is assumed that surplus distribution is done in the same order as election. True for AEC.
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution;

    // *** When the actual counting stops ***

    /// An elimination may involve multiple steps. If all vacancies are filled but not all steps are finished, do you finish all the counts, even though it cannot change the result of the election?
    fn finish_all_counts_in_elimination_when_all_elected() -> bool;
    /// If all vacancies are filled but not all surplus distributions are done, do you finish the surplus distributions, even though it cannot change the result of the election?
    fn finish_all_surplus_distributions_when_all_elected() -> bool;

    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking;
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking;

    // how to do the elimination

    /// Whether the Commonwealth Electoral Act 1918, Section 273, subsection 13A multiple elimination abomination should be used. This is defaulted to false as no one else would do such a terrible thing, and even the AEC has only sometimes done it.
    fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { false }
}

struct PendingTranscript<Tally> {
    elected : Vec<CandidateElected>,
    not_continuing : Vec<CandidateIndex>,
    created_transfer_value : Option<TransferValueCreation<Tally>>,
    decisions : Vec<DecisionMadeByEC>,
}

/// The main workhorse class that does preference distribution.
pub struct PreferenceDistributor<'a,Rules:PreferenceDistributionRules> {
    data : &'a ElectionData,
    ec_resolutions: &'a TieResolutionsMadeByEC,
    original_votes:&'a Vec<PartiallyDistributedVote<'a>>,
    num_candidates : usize,
    candidates_to_be_elected : usize,
    quota : Rules::Tally,
    /// The tally, by candidate.
    tallys : Vec<Rules::Tally>,
    /// the papers that a particular candidate currently has.
    papers : Vec<VotesWithMultipleTransferValues<'a,Rules::SplitByNumber,Rules::Tally>>,
    continuing_candidates : HashSet<CandidateIndex>,
    /// Candidates sorted lowest first, highest last.
    continuing_candidates_sorted_by_tally : Vec<CandidateIndex>,
    exhausted : BallotPaperCount,
    exhausted_atl : BallotPaperCount,
    tally_lost_to_rounding : Rules::Tally,
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
    pub fn new(data : &'a ElectionData,original_votes:&'a Vec<PartiallyDistributedVote<'a>>,candidates_to_be_elected : usize,excluded_candidates:&HashSet<CandidateIndex>,ec_resolutions:&'a TieResolutionsMadeByEC) -> Self {
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
            ec_resolutions,
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
            tally_lost_to_rounding: Rules::Tally::zero(),
            tally_exhausted: Rules::Tally::zero(),
            tally_set_aside: None,
            current_count : CountIndex(0),
            pending_surplus_distribution : VecDeque::default(),
            elected_candidates : vec![],
            in_this_count : PendingTranscript {
                elected: vec![],
                not_continuing: vec![],
                created_transfer_value: None,
                decisions: vec![]
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



    /// See if there are any ties in the tallys for the candidates in
    /// to_check (which should be already sorted by tally). If there are,
    /// resolve them, first using "how", secondly using self.ec_resolutions.
    /// Re-orders to_check to be in the appropriate order.
    pub fn check_for_ties_and_resolve(&mut self,to_check:&mut [CandidateIndex],how:MethodOfTieResolution) {
        // let mut to_check = &mut self.continuing_candidates_sorted_by_tally[to_check];
        let mut i:usize = 0;
        while i<to_check.len() {
            let mut differs = i+1;
            while differs<to_check.len() && self.tally(to_check[i])==self.tally(to_check[differs]) { differs+=1; }
            if differs!=i+1 { // we have a few with identical tallies
                let tied = &mut to_check[i..differs];
                if let Some(decision) = how.resolve(tied,&self.transcript) {
                    self.in_this_count.decisions.push(decision);
                    self.ec_resolutions.resolve(tied);
                }
            }
            i=differs;
        }
    }

    /// Like check_for_ties_and_resolve but do in place on self.continuing_candidates_sorted_by_tally for the indices given in to_check
    pub fn check_for_ties_and_resolve_inplace(&mut self,to_check:Range<usize>,how:MethodOfTieResolution) {
        // can't just pass a mutable reference to self.continuing_candidates_sorted_by_tally[to_check] as there would be 2 mutable refs :-(
        let mut tied_candidates = self.continuing_candidates_sorted_by_tally[to_check.clone()].to_vec();
        self.check_for_ties_and_resolve(&mut tied_candidates,how);
        self.continuing_candidates_sorted_by_tally[to_check].copy_from_slice(&tied_candidates); // copy resolved order back.
    }

    pub fn check_elected_by_quota(&mut self) {
        let mut elected_by_quota : Vec<CandidateIndex> = self.continuing_candidates_sorted_by_tally.iter().rev().take_while(|&&c|self.tally(c)>self.quota).cloned().collect();
        elected_by_quota.reverse(); // make sure low to high so that tie checking ordering is compatible.
        self.check_for_ties_and_resolve(&mut elected_by_quota,Rules::resolve_ties_elected_by_quota());
        for &c in elected_by_quota.iter().rev() {
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
    pub fn check_elected_by_highest_of_remaining_2_when_1_needed_no_tie_resolution(&mut self) {
        if self.continuing_candidates_sorted_by_tally.len()==2 && self.remaining_to_elect()==1 {
            let mut possibilities = self.continuing_candidates_sorted_by_tally.clone();
            self.check_for_ties_and_resolve(&mut possibilities,Rules::resolve_ties_elected_one_of_last_two());
            // elect the highest, Electoral officer resolved ties.
            self.declare_elected(possibilities[1],ElectionReason::HighestOfLastTwoStanding);
        }
    }

    /// federal rule 18
    /// > (18) Notwithstanding any other provision of this section, where the
    /// > number of continuing candidates is equal to the number of
    /// > remaining unfilled vacancies, those candidates shall be elected.
    pub fn check_if_should_elect_all_remaining(&mut self) {
        if self.continuing_candidates_sorted_by_tally.len()==self.remaining_to_elect() {
            let mut elected_group = self.continuing_candidates_sorted_by_tally.clone();
            self.check_for_ties_and_resolve(&mut elected_group,Rules::resolve_ties_elected_all_remaining());
            for &c in elected_group.iter().rev() {
                self.declare_elected(c,ElectionReason::AllRemainingMustBeElected);
            }
        }
    }

    /// See if one should check a particular termination rule
    pub fn should_check(&self,when:WhenToDoElectCandidateClauseChecking,reason : &ReasonForCount,reason_completed : bool) -> bool {
        match when  {
            WhenToDoElectCandidateClauseChecking::Never => false,
            WhenToDoElectCandidateClauseChecking::AfterCheckingQuota => true,
            WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing => reason_completed && self.pending_surplus_distribution.is_empty(),
            WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfExclusionNotOngoing => reason_completed || !reason.is_elimination(),
            WhenToDoElectCandidateClauseChecking::AfterDeterminingWhoToExcludeButBeforeTransferringAnyPapers => true,
            WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExists => self.pending_surplus_distribution.is_empty(),
        }
    }
    pub fn check_elected(&mut self,reason : &ReasonForCount,reason_completed : bool) {
        self.check_elected_by_quota();
        if self.should_check(Rules::when_to_check_if_just_two_standing_for_shortcut_election(),reason,reason_completed) {
            self.check_elected_by_highest_of_remaining_2_when_1_needed_no_tie_resolution();
        }
        if self.should_check(Rules::when_to_check_if_all_remaining_should_get_elected(),reason,reason_completed) {
            self.check_if_should_elect_all_remaining();
        }
    }

    pub fn end_of_count_step(&mut self,reason : ReasonForCount,portion : PortionOfReasonBeingDoneThisCount,reason_completed : bool) {
        self.resort_candidates();
        self.check_elected(&reason,reason_completed);
        self.print_tallys();
        self.current_count=CountIndex(self.current_count.0+1);
        self.transcript.counts.push(SingleCount{
            reason,
            portion,
            reason_completed,
            elected: self.in_this_count.elected.clone(),
            not_continuing: self.in_this_count.not_continuing.clone(),
            created_transfer_value: self.in_this_count.created_transfer_value.take(),
            decisions: self.in_this_count.decisions.clone(),
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
                    rounding:  BallotPaperCount(0),
                    set_aside: None
                },
                atl_papers: Some(PerCandidate {
                    candidate: self.papers.iter().map(|p|p.num_atl_ballots()).collect(),
                    exhausted: self.exhausted_atl,
                    rounding:  BallotPaperCount(0),
                    set_aside: None
                }),
            }
        });
        self.in_this_count.not_continuing=self.in_this_count.elected.drain(..).map(|e|e.who).collect();
        self.in_this_count.decisions.clear();
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
        let exhausted : BallotPaperCount = self.parcel_out_votes_with_given_transfer_value(transfer_value.clone(),ballots,Some(self.current_count),surplus.clone());
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
    pub fn parcel_out_votes_with_given_transfer_value(&mut self,transfer_value:TransferValue,ballots:VotesWithSameTransferValue<'a>,when_tv_created:Option<CountIndex>,original_worth:Rules::Tally) -> BallotPaperCount {
        let distributed = DistributedVotes::distribute(&ballots.votes,&self.continuing_candidates,self.num_candidates);
        let mut tally_distributed = Rules::Tally::zero();
        for (candidate_index,candidate_ballots) in distributed.by_candidate.into_iter().enumerate() {
            if candidate_ballots.num_ballots.0>0 {
                let (worth ,_lost_to_rounding) = Rules::use_transfer_value(&transfer_value,candidate_ballots.num_ballots);
                self.tallys[candidate_index]+=worth.clone();
                tally_distributed +=worth.clone();
                self.papers[candidate_index].add(&candidate_ballots, transfer_value.clone(), self.current_count, when_tv_created, worth);
            }
        }
        if distributed.exhausted.0>0 {
            let (worth ,_lost_to_rounding) = Rules::use_transfer_value(&transfer_value,distributed.exhausted);
            self.tally_exhausted+=worth.clone();
            tally_distributed +=worth.clone();
            self.exhausted+=distributed.exhausted;
            self.exhausted_atl+=distributed.exhausted_atl;
        }
        self.tally_lost_to_rounding+=original_worth;
        self.tally_lost_to_rounding-= tally_distributed;
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

    pub fn find_lowest_candidate(&mut self) -> Vec<CandidateIndex> {
        let lowest_tally = self.tally(self.continuing_candidates_sorted_by_tally[0]);
        let mut possibilities : Vec<CandidateIndex> = self.continuing_candidates_sorted_by_tally.iter().take_while(|&&c|self.tally(c)==lowest_tally).cloned().collect();
        self.check_for_ties_and_resolve(&mut possibilities,Rules::resolve_ties_choose_lowest_candidate_for_exclusion());
        possibilities.truncate(1);
        possibilities
    }

    /// There is a bizarre and horrible section of the federal election
    /// legislation where, in an attempt to make things easier, things are
    /// made much harder with an "optimization" to the process, whereby
    /// multiple candidates can be eliminated simultaneously. It is
    /// clearly designed, ineffectually, to not change the outcome of the
    /// election. It can change the outcome through rounding or through
    /// changing who the next candidate elected is through changing order
    /// of elimination.
    ///
    /// Commonwealth Electoral Act 1918 section 273 subsection 13A:
    /// ```text
    /// The procedure for a bulk exclusion, and the circumstances in
    /// which such an exclusion may be made, are as follows:
    /// (a) a continuing candidate (in this subsection called Candidate
    ///     A) shall be identified, if possible, who, of the continuing
    ///     candidates who each have a number of notional votes equal
    ///     to or greater than the vacancy shortfall, stands lower or
    ///     lowest in the poll;
    /// (b) a continuing candidate (in this subsection called Candidate
    ///     B) shall be identified, if possible, who:
    ///       (i) stands lower in the poll than Candidate A, or if
    ///           Candidate A cannot be identified, has a number of
    ///           notional votes that is fewer than the vacancy shortfall;
    ///      (ii) has a number of notional votes that is fewer than the
    ///           number of votes of the candidate standing immediately
    ///           higher than him or her in the poll; and
    ///     (iii) if 2 or more candidates satisfy subparagraphs (i) and
    ///          (ii)—is the candidate who of those candidates stands
    ///          higher or highest in the poll;
    /// (c) in a case where Candidate B has been identified and has a
    ///     number of notional votes fewer than the leading shortfall—
    ///     Candidate B and any other continuing candidates who stand
    ///     lower in the poll than that candidate may be excluded in a
    ///     bulk exclusion; and
    /// (d) in a case where Candidate B has been identified and has a
    ///     number of notional votes equal to or greater than the leading
    ///     shortfall:
    ///        (i) a continuing candidate (in this subsection called
    ///            Candidate C) shall be identified who:
    ///               (A) has a number of notional votes that is fewer
    ///                   than the leading shortfall; and
    ///               (B) if 2 or more candidates satisfy
    ///                   sub-subparagraph (A)—is the candidate who of
    ///                   those candidates stands higher or highest in the
    ///                   poll; and
    ///        (ii) Candidate C and all other continuing candidates who
    ///             stand lower in the poll than that candidate may be
    ///             excluded in a bulk exclusion.
    /// ```
    /// Commonwealth Electoral Act 1918 section 273 subsection 13B:
    /// ```text
    /// Where, apart from this subsection, the number of continuing
    /// candidates after a bulk exclusion under subsection (13A) would be
    /// fewer than the number of remaining unfilled vacancies,
    /// subsection (13A) shall operate to exclude only the number of
    /// candidates, beginning with the candidate who stands lowest in the
    /// poll, that would leave sufficient continuing candidates to fill the
    /// remaining unfilled vacancies.
    /// ```
    /// There is also subsection 13C, but I believe it is now redundant, as it
    /// deals with the case of a candidate who is elected but has not had votes
    /// distributed. In that case, I believe rule 13 (exclusion) doesn't come into
    /// play at all, but subsections 9, 10 or 14 (all surplus distribution) come into
    /// play and surplus distribution takes place. So we can ignore the concept of
    /// adjusted notional votes.
    ///
    /// Takes a mutable self because of the possibility of tie resolution.
    pub fn find_candidates_for_multiple_elimination_federal_rule_13a(&mut self) -> Option<Vec<CandidateIndex>> {
        // *shortfall*, in relation to a continuing candidate at a particular stage
        // during the scrutiny in a Senate election, means the number of votes
        // that the candidate requires at that stage in order to reach the quota
        // referred to in subsection (8).
        let shortfall = |candidate:CandidateIndex| self.quota.clone()-self.tally(candidate);

        // *leading shortfall*, in relation to a particular stage during the
        // scrutiny in a Senate election, means the shortfall of the continuing
        // candidate standing highest in the poll at that stage.
        let leading_shortfall : Rules::Tally = shortfall(*self.continuing_candidates_sorted_by_tally.last().unwrap());

        // *vacancy shortfall*, in relation to a particular stage during the
        // scrutiny in a Senate election, means the aggregate of the shortfalls
        // of that number of leading candidates equal to the number of
        // remaining unfilled vacancies, the leading candidates being
        // ascertained by taking the continuing candidate who stands highest
        // in the poll, the continuing candidate who stands next highest in the
        // poll, and so on in the order in which the continuing candidates
        // stand in the poll.
        let vacancy_shortfall : Rules::Tally = self.continuing_candidates_sorted_by_tally.iter().rev().take(self.remaining_to_elect()).map(|c|shortfall(*c)).sum();
        //println!("Count {} leading shortfall {} vacancy shortfall {}",self.current_count.0+1,&leading_shortfall,&vacancy_shortfall);

        // *notional vote*, in relation to a continuing candidate, means the
        // aggregate of the votes obtained by that candidate and the votes
        // obtained by each other candidate who stands lower in the poll than
        // him or her.
        let mut notional_votes : Vec<Rules::Tally> = vec![];
        for &candidate in &self.continuing_candidates_sorted_by_tally {
            notional_votes.push(self.tally(candidate)+notional_votes.last().cloned().unwrap_or_else(Rules::Tally::zero))
        }
        //println!("Notional votes {}",notional_votes.iter().map(|v|v.to_string()).collect::<Vec<_>>().join("\t"));
        // Find Candidate B. There is no point finding Candidate A, we merely need to
        // find a candidate B who is the highest ranking candidate with fewer notional
        // votes than the vacancy shortfall, and a number of notional votes < votes of higher person.
        let candidate_b_standing = {
            let num_candidates_with_fewer_notional_votes_than_the_vacancy_shortfall = notional_votes.iter().take_while(|t|**t<vacancy_shortfall).count();
            let mut candidate_b_plus_one = min(num_candidates_with_fewer_notional_votes_than_the_vacancy_shortfall,self.continuing_candidates_sorted_by_tally.len()-1);
            // a candidate passes test b(i) iff lower than num_candidates_with_fewer_notional_votes_than_the_vacancy_shortfall in standing. So find the highest satisfying b(ii).
            // the min in the line above was to ensure that a candidate above exists.
            while candidate_b_plus_one>0 && notional_votes[candidate_b_plus_one-1].clone()>=self.tally(self.continuing_candidates_sorted_by_tally[candidate_b_plus_one]) { candidate_b_plus_one-=1;}
            if candidate_b_plus_one==0 { return None; } // there is no candidate B, and nothing can be done.
            // candidate_b_standing is the index into self.continuing_candidates_sorted_by_tally of candidate b.
            candidate_b_plus_one-1
        };
        // println!("Candidate B standing {} notional votes {} tally {} tally 1 higher {}",candidate_b_standing,notional_votes[candidate_b_standing],self.tally(self.continuing_candidates_sorted_by_tally[candidate_b_standing]),self.tally(self.continuing_candidates_sorted_by_tally[candidate_b_standing+1]));
        // let candidate_b : CandidateIndex = self.continuing_candidates_sorted_by_tally[candidate_b_standing];
        let candidates_to_exclude : usize = if notional_votes[candidate_b_standing]<leading_shortfall { // (c) in a case where Candidate B has been identified and has a number of notional votes fewer than the leading shortfall
            candidate_b_standing+1 // Candidate B and any other continuing candidates who stand lower in the poll than that candidate may be excluded in a bulk exclusion
        } else { // (d) in a case where Candidate B has been identified and has a number of notional votes equal to or greater than the leading shortfall:
            // candidate C is the highest candidate with notional votes < leading shortfall which has to be < B as B has notional votes >=leading shortfall.
            // note that the legislation says "[candidate C] shall be identified" which is not necessarily possible!
            let num_candidates_with_fewer_notional_votes_than_the_leading_shortfall = notional_votes.iter().take_while(|t|**t<leading_shortfall).count();
            if num_candidates_with_fewer_notional_votes_than_the_leading_shortfall==0 { return None } // no such candidate C exists! Legislation fails! Better rehold the election! Or maybe just don't do multiple elimination this round.
            // let candidate_c : CandidateIndex = self.continuing_candidates_sorted_by_tally[num_candidates_with_fewer_notional_votes_than_the_leading_shortfall-1];
            num_candidates_with_fewer_notional_votes_than_the_leading_shortfall
        };
        // now take into account subsection 13B:
        let candidates_to_exclude = min(candidates_to_exclude,self.continuing_candidates_sorted_by_tally.len()-self.remaining_to_elect());
        if candidates_to_exclude==0 { return None; }
        // now need to check for ties. Candidate B cannot tie in a way that matters because of b(ii), but candidate C might.
        let tally_of_highest_excluded : Rules::Tally = self.tally(self.continuing_candidates_sorted_by_tally[candidates_to_exclude-1]);
        let mut tie_end = candidates_to_exclude;
        while tie_end<self.continuing_candidates_sorted_by_tally.len() && tally_of_highest_excluded==self.tally(self.continuing_candidates_sorted_by_tally[tie_end]) { tie_end+=1; }
        if tie_end>candidates_to_exclude { // there is a tie, and at least 1 of tied candidates will be excluded, and at least 1 will not, so the tie matters.
            // Use rule 31(b) to resolve ties.
            let mut tie_start : usize = candidates_to_exclude-1;
            while tie_start>0 && tally_of_highest_excluded==self.tally(self.continuing_candidates_sorted_by_tally[tie_start-1]) { tie_start-=1; }
            self.check_for_ties_and_resolve_inplace(tie_start..tie_end,Rules::resolve_ties_choose_lowest_candidate_for_exclusion());
        }
        // exclude the lowest candidates_to_exclude candidates.
        Some(self.continuing_candidates_sorted_by_tally[0..candidates_to_exclude].to_vec())
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
    pub fn exclude(&mut self, candidates_to_exclude:Vec<CandidateIndex>) {
        for &candidate in &candidates_to_exclude {
            println!("Excluding {}",self.data.metadata.candidate(candidate).name);
            self.no_longer_continuing(candidate,false);
        }
        if Rules::when_to_check_if_all_remaining_should_get_elected()==WhenToDoElectCandidateClauseChecking::AfterDeterminingWhoToExcludeButBeforeTransferringAnyPapers && self.continuing_candidates_sorted_by_tally.len()==self.remaining_to_elect()
           || Rules::when_to_check_if_just_two_standing_for_shortcut_election()==WhenToDoElectCandidateClauseChecking::AfterDeterminingWhoToExcludeButBeforeTransferringAnyPapers && self.continuing_candidates_sorted_by_tally.len()==2 && self.remaining_to_elect()==1 {
            self.end_of_count_step(ReasonForCount::Elimination(candidates_to_exclude.clone()), PortionOfReasonBeingDoneThisCount {
                transfer_value: None,
                when_tv_created : None,
                papers_came_from_counts: vec![],
            }, false);
            return; // Don't transfer any papers!
        }
        let mut provenances : HashSet<(<Rules::SplitByNumber as HowSplitByCountNumber>::KeyToDivide,TransferValue)> = HashSet::default();
        for &candidate in &candidates_to_exclude {
            for prov in self.papers[candidate.0].get_all_provenance_keys() {
                provenances.insert(prov);
            }
        }
        let mut provenances : Vec<(<Rules::SplitByNumber as HowSplitByCountNumber>::KeyToDivide,TransferValue)> = provenances.into_iter().collect();
        // TODO sort by S::KeyToDivide
        provenances.sort_by_key(|f|f.1.0.clone().neg()); // stable sort, will preserve ordering of other key stull
        let mut togo = provenances.len();
        for key in provenances {
            // doing the transfer for this key.
            let mut all_votes = VotesWithSameTransferValue::default();
            let mut original_worth = Rules::Tally::zero();
            let mut when_tv_created = DetectUnique::<Option<CountIndex>>::default();
            let mut papers_came_from_counts = CollectAll::<CountIndex>::default();
            for &candidate in &candidates_to_exclude {
                if let Some((from,votes)) = self.papers[candidate.0].extract_all_ballots_with_given_provenance(&key) {
                    when_tv_created.add(from.when_tv_created);
                    original_worth+=from.tally.clone();
                    papers_came_from_counts.extend(from.counts_comes_from.into_iter());
                    self.tallys[candidate.0]-=from.tally;
                    if all_votes.num_ballots.0==0 { all_votes=votes; }
                    else { all_votes.add(&votes.votes) }
                }
            }
            let when_tv_created=when_tv_created.take().flatten();
            self.parcel_out_votes_with_given_transfer_value(key.1.clone(),all_votes,when_tv_created,original_worth);
            togo-=1;
            self.end_of_count_step(ReasonForCount::Elimination(candidates_to_exclude.clone()), PortionOfReasonBeingDoneThisCount {
                transfer_value: Some(key.1),
                when_tv_created,
                papers_came_from_counts: papers_came_from_counts.take(),
            }, togo==0);
            if self.remaining_to_elect()==0 && !Rules::finish_all_counts_in_elimination_when_all_elected() { break; }
        }
    }


    pub fn distribute_lowest(&mut self) {
        let candidates_to_exclude : Vec<CandidateIndex> =
            if Rules::should_eliminate_multiple_candidates_federal_rule_13a() { self.find_candidates_for_multiple_elimination_federal_rule_13a().unwrap_or_else(||self.find_lowest_candidate()) }
            else { self.find_lowest_candidate() };
        self.exclude(candidates_to_exclude);
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

pub fn distribute_preferences<Rules:PreferenceDistributionRules>(data:&ElectionData,candidates_to_be_elected : usize,excluded_candidates:&HashSet<CandidateIndex>,ec_resolutions:& TieResolutionsMadeByEC) -> Transcript<Rules::Tally> {
    let arena = typed_arena::Arena::<CandidateIndex>::new();
    let votes = data.resolve_atl(&arena);
    let mut work : PreferenceDistributor<'_,Rules> = PreferenceDistributor::new(data,&votes,candidates_to_be_elected,excluded_candidates,ec_resolutions);
    work.go();
    work.transcript
}