// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use crate::distribution_of_preferences_transcript::{CountIndex, PerCandidate, QuotaInfo, ReasonForCount, Transcript, TranscriptWithMetadata};
use crate::ballot_metadata::{CandidateIndex, ElectionMetadata, NumberOfCandidates};
use std::cmp::min;
use num::{abs, Zero};
use std::ops::Sub;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::num::ParseIntError;
use crate::ballot_pile::BallotPaperCount;
use crate::signed_version::SignedVersion;
use std::str::FromStr;
use crate::compare_transcripts::{DeltasInCandidateLists, DifferentCandidateLists};
use crate::datasource_description::ElectionDataSource;
use crate::official_dop_transcript::DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyDeltaRounding;
use crate::parse_util::FileFinder;
use crate::preference_distribution::PreferenceDistributionRules;
use crate::tie_resolution::TieResolutionExplicitDecisionInCount;
use crate::verify_official_transcript::distribute_preferences_using_official_results;

#[derive(thiserror::Error, Debug, Eq, PartialEq)]
// The following differences come from a comparison of the official transcript against one created by ConcreteSTV.
pub enum DifferenceBetweenOfficialDoPAndComputed<Tally:Display+Debug> {
    #[error("There are {0} counts in the official DoP and {1} in the ConcreteSTV DoP.")]
    DifferentNumbersOfCounts(usize,usize),
    #[error("There are {0} vacancies in the official DoP quota computation and {1} in the ConcreteSTV case.")]
    DifferentNumbersOfVacanciesInQuota(NumberOfCandidates,NumberOfCandidates),
    #[error("There are {0} formal ballot papers in the official DoP quota computation and {1} in the ConcreteSTV case.")]
    DifferentNumbersOfPapersInQuota(BallotPaperCount,BallotPaperCount),
    #[error("The quota is {0} in the official DoP quota computation and {1} in the ConcreteSTV case.")]
    DifferentQuota(ECTally,Tally),
    #[error("There is a difference on count {} : {2}", count_name(*.0,&.1))]
    DifferentOnCount(CountIndex,Option<String>,DifferenceBetweenOfficialDoPAndComputedOnParticularCount<Tally>),
}

fn count_name(count_index:CountIndex,count_name:&Option<String>) -> String {
    match count_name {
        None => format!("#{}",count_index.0+1),
        Some(name) => format!("#{} a.k.a. {}",count_index.0+1,name),
    }
}

#[derive(thiserror::Error, Debug, Eq, PartialEq)]
// The following differences come from a comparison of the official transcript against one created by ConcreteSTV.
pub enum DifferenceBetweenOfficialDoPAndComputedOnParticularCount<Tally:Display+Debug> {
    #[error("The count name is {0:?} in the official DoP quota computation and {1:?} in the ConcreteSTV case.")]
    CountName(Option<String>,Option<String>),
    #[error("The elected candidates in order of election are {0:?} in the official DoP nd {1:?} in the ConcreteSTV case.")]
    ElectedCandidatesOrdered(Vec<CandidateIndex>,Vec<CandidateIndex>),
    #[error("The elected candidates (ballot paper order) are {0:?} in the official DoP and {1:?} in the ConcreteSTV case.")]
    ElectedCandidatesUnordered(Vec<CandidateIndex>,Vec<CandidateIndex>),
    #[error("The excluded candidates (ballot paper order) are {0:?} in the official DoP and {1:?} in the ConcreteSTV case.")]
    ExcludedCandidatesUnordered(Vec<CandidateIndex>,Vec<CandidateIndex>),
    //#[error("In the official list candidate {0} ceased to be a continuing candidate for the first time this count, but not in the ConcreteSTV case.")]
    //CandidateNotContinuingInOfficalCount(CandidateIndex),
    //#[error("In the official list candidate {0} ceased to be a continuing candidate for the first time this count, but not in the ConcreteSTV case. There was an EC decision involving {1:?} but none were excluded by ConcreteSTV.")]
    //CandidateNotContinuingInOfficialCountWithPointlessDecision(CandidateIndex, Vec<CandidateIndex>),
    #[error("The affected ballots in this count came from counts {0:?} (0 based) in the official DoP and {1:?} in ConcreteSTV.")]
    FromCounts(Vec<CountIndex>,Vec<CountIndex>),
    #[error("The total number of exhausted papers is {0} in the official DoP and {1} in ConcreteSTV.")]
    PaperTotalExhausted(BallotPaperCount,BallotPaperCount),
    #[error("The total number of lost to rounding papers is {0} in the official DoP and {1} in ConcreteSTV.")]
    PaperTotalRounding(BallotPaperCount,BallotPaperCount), // should be 0 in all cases???
    #[error("The total number of papers is {0} in the official DoP and {1} in ConcreteSTV for candidate #{2}.")]
    PaperTotalCandidate(BallotPaperCount,BallotPaperCount,CandidateIndex),
    #[error("The total number of exhausted set aside for quota papers is {0} in the official DoP and {1} in ConcreteSTV.")]
    PaperSetAsideForQuotaExhausted(BallotPaperCount,BallotPaperCount),
    #[error("The total number of lost to rounding set aside for quota papers is {0} in the official DoP and {1} in ConcreteSTV.")]
    PaperSetAsideForQuotaRounding(BallotPaperCount,BallotPaperCount), // should be 0 in all cases???
    #[error("The number of papers set aside for quota is {0} in the official DoP and {1} in ConcreteSTV for candidate #{2}.")]
    PaperSetAsideForQuotaCandidate(BallotPaperCount,BallotPaperCount,CandidateIndex),
    #[error("The change in number of exhausted papers is {0} in the official DoP and {1} in ConcreteSTV.")]
    PaperDeltaExhausted(isize,usize),
    #[error("The change in number of papers lost to rounding papers is {0} in the official DoP and {1} in ConcreteSTV.")]
    PaperDeltaRounding(isize,usize), // should be 0 in all cases???
    #[error("The change in number of papers is {0} in the official DoP and {1} in ConcreteSTV for candidate #{2}.")]
    PaperDeltaCandidate(isize,isize,CandidateIndex),
    #[error("The total number of votes lost to exhaustion or rounding is {0} in the official DoP and {1} from exhaustion and {2} from rounding ConcreteSTV.")]
    TallyTotalExhaustedAndRounding(ECTally,Tally,SignedVersion<Tally>),
    #[error("The total number of exhausted votes is {0} in the official DoP and {1} in ConcreteSTV.")]
    TallyTotalExhausted(ECTally,Tally),
    #[error("The total number of lost to rounding votes is {0} in the official DoP and {1} in ConcreteSTV.")]
    TallyTotalRounding(ECTally,SignedVersion<Tally>), // should be 0 in all cases???
    #[error("The total number of votes is {0} in the official DoP and {1} in ConcreteSTV for candidate #{2}.")]
    TallyTotalCandidate(ECTally,Tally,CandidateIndex),
    #[error("The change in number of exhausted votes is {0} in the official DoP and {2}-{1} in ConcreteSTV.")]
    TallyDeltaExhausted(ECTally,Tally,Tally),
    #[error("The change in number of votes lost to rounding is {0} in the official DoP and {1} in ConcreteSTV.")]
    TallyDeltaRounding(ECTally,SignedVersion<Tally>), // should be 0 in all cases???
    #[error("The change in number of votes is {0} in the official DoP and {2}-{1} in ConcreteSTV for candidate #{3}.")]
    TallyDeltaCandidate(ECTally,Tally,Tally,CandidateIndex),
    #[error("The official dop has a set aside count but the ConcreteSTV transcript does not.")]
    OfficialCountHasSetAsideButComputedDoesnt,
}

/// A vote that is a finite, comparable, not-NaN value
#[derive(Copy,Clone)]
pub struct ECTally(pub f64);

impl Display for ECTally {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.0)
    }
}
impl Debug for ECTally {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.0)
    }
}

impl PartialEq<Self> for ECTally {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}

impl From<ECTally> for f64 {
    fn from(value: ECTally) -> Self { value.0 }
}

impl From<f64> for ECTally {
    fn from(value: f64) -> Self {
        assert!(!value.is_nan());
        ECTally(value)
    }
}

impl Eq for ECTally {}

/// Information for a particular count from the official transcript.
#[derive(Default)]
pub struct OfficialDOPForOneCount {
    pub transfer_value : Option<f64>,
    pub elected : Vec<CandidateIndex>,
    pub excluded : Vec<CandidateIndex>,
    pub vote_total : Option<PerCandidate<f64>>, // A NaN means unknown
    pub paper_total : Option<PerCandidate<usize>>, // an isize::MAX means unknown
    pub vote_delta : Option<PerCandidate<f64>>, // A NaN means unknown.
    pub paper_delta : Option<PerCandidate<isize>>, // an isize::MAX means unknown.
    pub paper_set_aside_for_quota: Option<PerCandidate<usize>>, // an usize::MAX means unknown. This is rarely used (possibly only in the NSW randomized algorithm for legislative council and LGE prior to 2021)
    pub count_name : Option<String>,
    pub papers_came_from_counts : Option<Vec<CountIndex>>, // if present, which were the source for the counts. Should be in ascending order.
}

/// Information from
#[derive(Default)]
pub struct OfficialDistributionOfPreferencesTranscript {
    pub quota : Option<QuotaInfo<f64>>,
    pub counts : Vec<OfficialDOPForOneCount>,
    /// true if the record does not contain negative papers amounts.
    pub missing_negatives_in_papers_delta : bool,
    /// true if members of "elected" are in order of election.
    pub elected_candidates_are_in_order : bool,
    /// true if exhausted votes all go to rounding.
    pub all_exhausted_go_to_rounding : bool,
    /// true if the assignment of negative values to tallies and rounding may be ambiguous. E.g. WA.
    pub negative_values_in_surplus_distributions_and_rounding_may_be_off : bool,
}

impl OfficialDistributionOfPreferencesTranscript {
    pub fn print_table(&self,metadata:&ElectionMetadata) {
        if let Some(quota) = &self.quota {
            println!("Papers : {}, Vacancies : {} -> Quota : {}",quota.papers.0,quota.vacancies.0,quota.quota)
        }
        print!("\t");
        for candidate_index in 0..metadata.candidates.len() {
            print!("{}\t",metadata.candidates[candidate_index].name);
        }
        println!("Exhausted\tRounding");
        for count in &self.counts {
            for candidate in &count.excluded { println!("Excluded {}",metadata.candidate(*candidate).name)}
            let print_row = |heading,row:&PerCandidate<f64>| {
                print!("{}\t",heading);
                for candidate_index in 0..metadata.candidates.len() {
                    print!("{}\t",row.candidate[candidate_index]);
                }
                println!("{}\t{}",row.exhausted,row.rounding.resolve());
            };
            if let Some(row) = &count.vote_delta { print_row("ΔVotes",row); }
            if let Some(row) = &count.vote_total { print_row("Votes",row); }
            for candidate in &count.elected { println!("Elected {}",metadata.candidate(*candidate).name)}
        }
    }
}
impl OfficialDOPForOneCount {
    pub fn vote_total(&mut self) -> &mut PerCandidate<f64> { self.vote_total.get_or_insert_with(Default::default) }
    pub fn paper_total(&mut self) -> &mut PerCandidate<usize> { self.paper_total.get_or_insert_with(Default::default) }
    pub fn vote_delta(&mut self) -> &mut PerCandidate<f64> { self.vote_delta.get_or_insert_with(Default::default) }
    pub fn paper_delta(&mut self) -> &mut PerCandidate<isize> { self.paper_delta.get_or_insert_with(Default::default) }
}

// like From<X> but implemented for usize (if there are more than 2^53 votes, the official transcript checking will have problems).

pub trait CanConvertToF64PossiblyLossily {
    fn convert_to_f64(&self) -> f64;
}

impl CanConvertToF64PossiblyLossily for usize {
    fn convert_to_f64(&self) -> f64 { *self as f64 }
}

impl OfficialDOPForOneCount {
    // given a string containing a comma separated list of 1 based counts, starting with start_count_list_string and ending in suffix,
    // get a list of counts.
    pub fn extract_counts_from_comment(comment:&str, start_count_list_string:&str, suffix:&str) -> Result<Option<Vec<CountIndex>>,ParseIntError> {
        if let Some(pos) = comment.find(start_count_list_string) {
            if let Some(remaining) = comment[pos+start_count_list_string.len()..].strip_suffix(suffix) {
                let count_list : Result<Vec<usize>,_> = remaining.trim().split(',').map(|s|s.trim().parse()).collect();
                let mut count_list = count_list?;
                count_list.sort();
                Ok(Some(count_list.into_iter().map(|v|CountIndex(v-1)).collect()))
            } else { Ok(None) }
        } else { Ok(None) }
    }
}

impl OfficialDistributionOfPreferencesTranscript {
    /// Initialize a new count
    pub fn finished_count(&mut self) { self.counts.push(OfficialDOPForOneCount::default()) }
    /// Get the current count
    pub fn count(&mut self) -> &mut OfficialDOPForOneCount { self.counts.last_mut().unwrap() }

    /// Gets all elected candidates.
    pub fn all_elected(&self) -> Vec<CandidateIndex> {
        self.counts.iter().flat_map(|c| c.elected.iter()).cloned().collect()
    }
    /// Compare the results from the official transcript to our transcript.
    /// panic if there are differences.
    pub fn compare_with_transcript<Tally: Clone + Zero + Debug + PartialEq + Sub<Output=Tally> + Display + Ord + FromStr + CanConvertToF64PossiblyLossily>(&self, transcript: &Transcript<Tally>) {
        let ec_decision = self.compare_with_transcript_checking_for_ec_decisions(transcript, true).unwrap();
        if let Some(decision) = ec_decision {
            panic!("An EC decision was not made the way we expected: {}", decision.decision);
        }
    }
    /// like compare_with_transcript but don't panic if the first difference is caused by a difference in EC decision making. If so, return the decision.
    /// If there is some other error, return the error.
    pub fn compare_with_transcript_checking_for_ec_decisions<Tally: Clone + Zero + Debug + Ord + PartialEq + Sub<Output=Tally> + Display + FromStr + CanConvertToF64PossiblyLossily>(&self, transcript: &Transcript<Tally>, verbose: bool) -> Result<Option<TieResolutionExplicitDecisionInCount>, DifferenceBetweenOfficialDoPAndComputed<Tally>> {
        fn decode<Tally: CanConvertToF64PossiblyLossily>(tally: Tally) -> f64 { tally.convert_to_f64() }
        if let Some(quota) = &self.quota {
            if quota.vacancies != transcript.quota.as_ref().unwrap().vacancies { return Err(DifferenceBetweenOfficialDoPAndComputed::DifferentNumbersOfVacanciesInQuota(quota.vacancies, transcript.quota.as_ref().unwrap().vacancies)) }
            if quota.papers != transcript.quota.as_ref().unwrap().papers { return Err(DifferenceBetweenOfficialDoPAndComputed::DifferentNumbersOfPapersInQuota(quota.papers, transcript.quota.as_ref().unwrap().papers)) }
            if quota.quota != decode(transcript.quota.as_ref().unwrap().quota.clone()) { return Err(DifferenceBetweenOfficialDoPAndComputed::DifferentQuota(quota.quota.into(), transcript.quota.as_ref().unwrap().quota.clone())) }
        }
        for count_number in 0..min(self.counts.len(), transcript.counts.len()) {
            let count_number = CountIndex(count_number);
            let res = self.compare_with_transcript_checking_for_ec_decisions_on_given_count(transcript, count_number, verbose)
                .map_err(|e| DifferenceBetweenOfficialDoPAndComputed::DifferentOnCount(count_number,transcript.counts[count_number.0].count_name.clone(), e))?;
            if res.is_some() { return Ok(res) }
        }
        if self.counts.len() != transcript.counts.len() { return Err(DifferenceBetweenOfficialDoPAndComputed::DifferentNumbersOfCounts(self.counts.len(), transcript.counts.len())) }
        Ok(None)
    }

    /// Compare a specific count on the official count to the actual count.
    fn compare_with_transcript_checking_for_ec_decisions_on_given_count<Tally: Clone + Zero + Ord + Debug + PartialEq + Sub<Output=Tally> + Display + FromStr + CanConvertToF64PossiblyLossily>(&self, transcript: &Transcript<Tally>, count_number: CountIndex, verbose: bool) -> Result<Option<TieResolutionExplicitDecisionInCount>, DifferenceBetweenOfficialDoPAndComputedOnParticularCount<Tally>> {
        fn decode<Tally: CanConvertToF64PossiblyLossily>(tally: Tally) -> f64 { tally.convert_to_f64() }
        fn different<Tally: CanConvertToF64PossiblyLossily>(official:f64,tally: Tally) -> bool {
            let our = tally.convert_to_f64();
            abs(our-official) > 1e-7
        }
        fn different_signed<Tally: CanConvertToF64PossiblyLossily>(official:f64,tally: SignedVersion<Tally>) -> bool where Tally: Clone, Tally: PartialEq {
            let our = tally.convert_f64(&decode);
            abs(our-official) > 1e-7
        }
        let my_count = &transcript.counts[count_number.0];
        let ambiguous_rounding_for_candidate : Option<CandidateIndex> = match my_count.reason { ReasonForCount::ExcessDistribution(c) if self.negative_values_in_surplus_distributions_and_rounding_may_be_off => Some(c), _ => None};
        let my_prior_count = if count_number.0>0 {Some(&transcript.counts[count_number.0-1])} else {None};
        let official_count = &self.counts[count_number.0];
        if verbose { println!("Checking count {} {}", count_number.0 + 1, my_count.count_name.clone().unwrap_or_default()); }
        if my_count.count_name!=official_count.count_name { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::CountName(my_count.count_name.clone(), official_count.count_name.clone()))}
        if self.elected_candidates_are_in_order {
            let my_order = my_count.elected.iter().map(|e| e.who).collect::<Vec<CandidateIndex>>();
            if official_count.elected!=my_order { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ElectedCandidatesOrdered(official_count.elected.clone(),my_order))}
        } else {
            let mut official_order = official_count.elected.clone();
            official_order.sort_by_key(|c|c.0);
            let mut my_order = my_count.elected.iter().map(|e| e.who).collect::<Vec<CandidateIndex>>();
            my_order.sort_by_key(|c|c.0);
            if official_order!=my_order { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ElectedCandidatesUnordered(official_order,my_order))}
        }
        let my_excluded : Vec<CandidateIndex> = if let ReasonForCount::Elimination(eliminated) = &my_count.reason { eliminated.iter().filter(|c|my_count.not_continuing.contains(*c)).cloned().collect() } else { vec![] };
        let excluded_comparison = DifferentCandidateLists{ list1: official_count.excluded.clone(), list2: my_excluded.clone() };
        let excluded_deltas : DeltasInCandidateLists = excluded_comparison.into();
        if !excluded_deltas.is_empty() {
            for decision in &my_count.decisions { // see if a different decision would solve this.
                if let Some(alternate_decision) = decision.could_a_different_decision_have_caused_different_candidates_to_be_excluded(&excluded_deltas) {
                    return Ok(Some(TieResolutionExplicitDecisionInCount { decision: alternate_decision, came_up_in: Some(count_number) }))
                }
            }
            let mut official_order = official_count.excluded.clone();
            official_order.sort_by_key(|c|c.0);
            let mut my_order = my_excluded;
            my_order.sort_by_key(|c|c.0);
            return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ExcludedCandidatesUnordered(official_order,my_order))
        }
        if let Some(vote_total) = &official_count.vote_total {
            if verbose { println!("Checking tally count {}", count_number.0 + 1); }
            if self.all_exhausted_go_to_rounding {
                if different(vote_total.rounding.resolve()-my_count.status.tallies.rounding.convert_f64(&decode),my_count.status.tallies.exhausted.clone()) { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalExhaustedAndRounding(vote_total.rounding.resolve().into(),my_count.status.tallies.exhausted.clone(),my_count.status.tallies.rounding.clone()))}
            } else {
                if different(vote_total.exhausted,my_count.status.tallies.exhausted.clone()) { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalExhausted(vote_total.exhausted.into(),my_count.status.tallies.exhausted.clone()))}
                if ambiguous_rounding_for_candidate.is_none() {
                    if different_signed(vote_total.rounding.resolve(),my_count.status.tallies.rounding.clone()) { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalRounding(vote_total.rounding.resolve().into(),my_count.status.tallies.rounding.clone()))}
                }
            }
            for candidate in 0..vote_total.candidate.len() {
                if ambiguous_rounding_for_candidate!=Some(CandidateIndex(candidate)) {
                    if different(vote_total.candidate[candidate], my_count.status.tallies.candidate[candidate].clone()) { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalCandidate(vote_total.candidate[candidate].into(), my_count.status.tallies.candidate[candidate].clone(), CandidateIndex(candidate))) }
                }
            }
        }
        if let Some(vote_delta) = &official_count.vote_delta {
            if verbose { println!("Checking tally delta count {}", count_number.0 + 1); }
            let tally_exhausted_now = my_count.status.tallies.exhausted.clone();
            let tally_exhausted_prior = my_prior_count.map(|c|c.status.tallies.exhausted.clone()).unwrap_or_else(||Tally::zero());
            if different(vote_delta.exhausted+decode(tally_exhausted_prior.clone()),tally_exhausted_now.clone()) { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyDeltaExhausted(vote_delta.exhausted.into(),tally_exhausted_prior,tally_exhausted_now))}
            if ambiguous_rounding_for_candidate.is_none() {
                let tally_rounding_now = my_count.status.tallies.rounding.clone();
                let tally_rounding_prior = my_prior_count.map(|c| c.status.tallies.rounding.clone()).unwrap_or_else(|| SignedVersion::<Tally>::zero());
                let tally_rounding = tally_rounding_now - tally_rounding_prior;
                if different_signed(vote_delta.rounding.resolve(), tally_rounding.clone()) { return Err(TallyDeltaRounding(vote_delta.rounding.resolve().into(), tally_rounding)) }
            }
            for candidate in 0..vote_delta.candidate.len() {
                let tally_now = my_count.status.tallies.candidate[candidate].clone();
                let tally_prior = my_prior_count.map(|c|c.status.tallies.candidate[candidate].clone()).unwrap_or_else(||Tally::zero());
                if ambiguous_rounding_for_candidate!=Some(CandidateIndex(candidate)) {
                    if different(vote_delta.candidate[candidate] + decode(tally_prior.clone()), tally_now.clone()) { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyDeltaCandidate(vote_delta.candidate[candidate].into(), tally_prior, tally_now, CandidateIndex(candidate))) }
                }
            }
        }
        if let Some(paper_total) = &official_count.paper_total {
            if verbose { println!("Checking paper count {}", count_number.0 + 1); }
            if paper_total.exhausted!=my_count.status.papers.exhausted.0 { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::PaperTotalExhausted(BallotPaperCount(paper_total.exhausted),my_count.status.papers.exhausted))}
            if paper_total.rounding.assume_positive()!=my_count.status.papers.rounding.assume_positive().0 { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::PaperTotalRounding(BallotPaperCount(paper_total.rounding.assume_positive()),my_count.status.papers.rounding.assume_positive()))}
            for candidate in 0..paper_total.candidate.len() {
                if ambiguous_rounding_for_candidate!=Some(CandidateIndex(candidate)) {
                    if paper_total.candidate[candidate]!=my_count.status.papers.candidate[candidate].0 { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::PaperTotalCandidate(BallotPaperCount(paper_total.candidate[candidate]),my_count.status.papers.candidate[candidate],CandidateIndex(candidate)))}
                }
            }
        }
        if let Some(paper_delta) = &official_count.paper_delta {
            if verbose { println!("Checking paper delta {}", count_number.0 + 1); }
            let my_change_exhausted = my_count.status.papers.exhausted.0-my_prior_count.map(|c|c.status.papers.exhausted.0).unwrap_or(0);
            if paper_delta.exhausted!=my_change_exhausted as isize { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::PaperDeltaExhausted(paper_delta.exhausted,my_change_exhausted))}
            let my_change_rounding = my_count.status.papers.rounding.assume_positive().0-my_prior_count.map(|c|c.status.papers.rounding.assume_positive().0).unwrap_or(0);
            if paper_delta.rounding.resolve()!=my_change_rounding as isize { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::PaperDeltaRounding(paper_delta.rounding.resolve(),my_change_rounding))}
            for candidate in 0..paper_delta.candidate.len() {
                if ambiguous_rounding_for_candidate!=Some(CandidateIndex(candidate)) {
                    let my_change_candidate = my_count.status.papers.candidate[candidate].0 as isize-my_prior_count.map(|c|c.status.papers.candidate[candidate].0 as isize).unwrap_or(0);
                    if paper_delta.candidate[candidate]!=isize::MAX && paper_delta.candidate[candidate]!=my_change_candidate && !(self.missing_negatives_in_papers_delta && my_change_candidate < 0) { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::PaperDeltaCandidate(paper_delta.candidate[candidate],my_change_candidate,CandidateIndex(candidate)))}
                }
            }
        }
        if let Some(paper_set_aside) = & official_count.paper_set_aside_for_quota { // compare to
            if verbose { println!("Checking set aside count {}", count_number.0 + 1); }
            if let Some(my_set_aside) = &my_count.set_aside_for_quota {
                if paper_set_aside.exhausted!=my_set_aside.exhausted.0 {
                    // the NSW values are sometimes inaccurate
                    if paper_set_aside.exhausted==my_set_aside.exhausted.0+official_count.paper_delta.as_ref().map(|v|v.exhausted as usize).unwrap_or(0) && official_count.transfer_value==Some(1.0) {
                        println!("The official DoP set aside for quota of exhausted votes includes votes that do not contribute to the quota.");
                        // It could be better to correct the buggy NSWEC data in the DoP parse code where this comes up, but that would make this brittle if NSWEC gets it correct.
                    } else {
                        return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::PaperSetAsideForQuotaExhausted(BallotPaperCount(paper_set_aside.exhausted),my_set_aside.exhausted))
                    }
                }
                for candidate in 0..paper_set_aside.candidate.len() {
                    if paper_set_aside.candidate[candidate]!=my_set_aside.candidate[candidate].0 { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::PaperSetAsideForQuotaCandidate(BallotPaperCount(paper_set_aside.candidate[candidate]),my_set_aside.candidate[candidate],CandidateIndex(candidate)))}
                }

            } else {
                if !paper_set_aside.is_empty() {
                    return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::OfficialCountHasSetAsideButComputedDoesnt);
                }
            }

        }
        if let Some(papers_came_from_counts) = &official_count.papers_came_from_counts {
            if papers_came_from_counts!=&my_count.portion.papers_came_from_counts { return Err(DifferenceBetweenOfficialDoPAndComputedOnParticularCount::FromCounts(papers_came_from_counts.clone(),my_count.portion.papers_came_from_counts.clone()))}
        }
        Ok(None)
    }
}

/// Given a vector, make sure the array is long enough to hold the person's entry, and return a mutable reference to it.
pub fn candidate_elem<T:Default+Clone>(vec : &mut Vec<T>, who:CandidateIndex) -> &mut T {
    if vec.len()<=who.0 {
        vec.resize(who.0+1,T::default())
    }
    &mut vec[who.0]
}


/// Test a particular year & electorate against a particular set of rules.
/// Outermost error is IO type errors.
/// Innermost error is discrepancies with the official DoP.
///
/// If `save_transcripts` is true, transcripts will be saved to the `test_transcripts` folder.
pub fn test_official_dop_without_actual_votes<Rules:PreferenceDistributionRules,Source:ElectionDataSource>(source:&Source,year:&str,state:&str,save_transcripts:bool) -> anyhow::Result<Result<Option<TieResolutionExplicitDecisionInCount>, DifferenceBetweenOfficialDoPAndComputed<Rules::Tally>>> where <Rules as PreferenceDistributionRules>::Tally: Send+Sync+'static {
    let loader = source.get_loader_for_year(year,&FileFinder::find_ec_data_repository())?;
    let metadata = loader.read_raw_metadata(state)?;
    let official_transcript = loader.read_official_dop_transcript(&metadata)?;
    //veryify_official_dop_transcript::<Rules>(&official_transcript,&metadata)?;
    let transcript = distribute_preferences_using_official_results::<Rules>(&official_transcript,&metadata)?;
    let result = Ok(official_transcript.compare_with_transcript_checking_for_ec_decisions(&transcript,false));
    if save_transcripts {
        let transcript = TranscriptWithMetadata{ metadata, transcript };
        std::fs::create_dir_all("test_transcripts")?;
        let file = File::create(format!("test_transcripts/transcript{}{}_{}.json",state,year,Rules::name()))?;
        serde_json::to_writer_pretty(file,&transcript)?;
    }
    result
}

