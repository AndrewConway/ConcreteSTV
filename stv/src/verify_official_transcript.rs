// Copyright 2022-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! This module checks the official DOP transcript to some extent against some rules.
//! It tries to use the rules to check what should be done.


use std::collections::HashSet;
use std::fmt::{Debug, Display};
use std::iter;
use thiserror::Error;
use crate::ballot_metadata::{CandidateIndex, ElectionMetadata};
use crate::ballot_paper::BTL;
use crate::ballot_pile::BallotPaperCount;
use crate::distribution_of_preferences_transcript::{CountIndex, PerCandidate, QuotaInfo, Transcript};
use crate::election_data::ElectionData;
use crate::official_dop_transcript::{CanConvertToF64PossiblyLossily, OfficialDistributionOfPreferencesTranscript};
use crate::preference_distribution::{PreferenceDistributionRules, PreferenceDistributor};
use crate::tie_resolution::{TieResolutionAtom, TieResolutionExplicitDecision, TieResolutionExplicitDecisionInCount, TieResolutionGranularityNeeded, TieResolutionsMadeByEC};

#[derive(Error, Debug)]
pub enum IssueWithOfficialDOPTranscript<Tally:Display+Debug> {
    // The first set come from some general sanity checks.
    #[error("No counts present in official transcript, not even first preference distribution.")]
    DoesntHaveFirstCount,
    #[error("No vote counts present in official transcript first count.")]
    FirstCountHasNoVotes,
    #[error("Metadata does not state number of vacancies.")]
    MetadataMissingVacancies,
    #[error("The first preferences counts are not all integers")]
    FirstPreferenceVoteCountNotInteger,
    #[error("Official quota {0}, computed quota {1}")]
    QuotaWrong(QuotaInfo<f64>, QuotaInfo<Tally>),
    #[error("Count #{0} had the wrong number of candidates in the vote delta array")]
    WrongNumberOfCandidatesVoteDelta(CountIndex),
    #[error("Count #{0} had the wrong number of candidates in the vote total array")]
    WrongNumberOfCandidatesVoteTotal(CountIndex),
    #[error("Count #{0} had the wrong number of candidates in the paper delta array")]
    WrongNumberOfCandidatesPaperDelta(CountIndex),
    #[error("Count #{0} had the wrong number of candidates in the paper total array")]
    WrongNumberOfCandidatesPaperTotal(CountIndex),
    #[error("Count #{0} has the wrong total number of votes for candidate #{1} not equal to sum of prior count plus change")]
    VoteSumIncorrect(CountIndex, CandidateIndex),
    #[error("Count #{0} has the wrong total number of votes for exhausted votes not equal to sum of prior count plus change")]
    VoteSumIncorrectExhausted(CountIndex),
    #[error("Count #{0} has the wrong total number of votes for votes lost to rounding not equal to sum of prior count plus change")]
    VoteSumIncorrectLostToRounding(CountIndex),
    #[error("Count #{0} has the wrong total number of votes for set aside votes not equal to sum of prior count plus change")]
    VoteSumIncorrectSetAside(CountIndex),
    #[error("Count #{0} has the wrong total number of papers for candidate #{1} not equal to sum of prior count plus change")]
    PaperSumIncorrect(CountIndex, CandidateIndex),
    #[error("Count #{0} has the wrong total number of papers for exhausted votes not equal to sum of prior count plus change")]
    PaperSumIncorrectExhausted(CountIndex),
    #[error("Count #{0} has the wrong total number of papers for set aside votes not equal to sum of prior count plus change")]
    PaperSumIncorrectSetAside(CountIndex),
    #[error("There are exhausted votes at the first preference count")]
    ExhaustedAtFirstPrefCount,
    #[error("There are set aside votes at the first preference count")]
    SetAsideAtFirstPrefCount,
    #[error("There are lost to rounding votes at the first preference count")]
    LostToRoundingAtFirstPrefCount,
    #[error("First preference count has different values for votes received and total")]
    FirstPreferenceDeltasAndTotalsDifferentVotes,
    #[error("First preference count has different values for papers received and total")]
    FirstPreferenceDeltasAndTotalsDifferentPapers,
    #[error("First preference count has different values for votes and papers")]
    FirstPreferenceVotesAndPapersDifferent,
}


struct VerifyOfficialDopTranscript<'a,Rules:PreferenceDistributionRules> {
    metadata : &'a ElectionMetadata,
    official : &'a OfficialDistributionOfPreferencesTranscript,
    num_candidates : usize,
    first_preference_votes : usize,
    quota : Rules::Tally,
}

impl <'a,Rules:PreferenceDistributionRules> VerifyOfficialDopTranscript<'a,Rules> {

    // check quota and make new verifier
    fn new(official:&'a OfficialDistributionOfPreferencesTranscript,metadata:&'a ElectionMetadata) -> Result<Self,IssueWithOfficialDOPTranscript<Rules::Tally>> {
        if official.counts.is_empty() { return Err(IssueWithOfficialDOPTranscript::DoesntHaveFirstCount)}
        let num_candidates = metadata.candidates.len();
        // check quota
        let first_preference_votes  = {
            let vote_source = official.counts[0].vote_total.as_ref().or(official.counts[0].vote_delta.as_ref()).ok_or_else(||IssueWithOfficialDOPTranscript::FirstCountHasNoVotes)?;
            (vote_source.candidate.iter().sum::<f64>() + (if Rules::should_exhausted_votes_count_for_quota_computation() { vote_source.exhausted } else {0.0})) as usize
        };
        let candidates_to_be_elected = metadata.vacancies.ok_or_else(||IssueWithOfficialDOPTranscript::MetadataMissingVacancies)?;
        let quota = Rules::Tally::from(first_preference_votes/(1+candidates_to_be_elected.0)+1);
        if let Some(official_quota) = &official.quota  {
            if official_quota.quota!=quota.convert_to_f64() || official_quota.vacancies!=candidates_to_be_elected || official_quota.papers.0 as f64!=first_preference_votes as f64 {
                return Err(IssueWithOfficialDOPTranscript::QuotaWrong(official_quota.clone(),QuotaInfo{
                    papers: BallotPaperCount(first_preference_votes),
                    vacancies: candidates_to_be_elected,
                    quota,
                }))
            }
        }
        Ok(VerifyOfficialDopTranscript{
            metadata,
            official,
            num_candidates,
            first_preference_votes,
            quota,
        })
    }

    fn check_basic_arithmetic_adding_deltas_to_totals(&self) -> Result<(),IssueWithOfficialDOPTranscript<Rules::Tally>> {
        let mut last_vote_total : Option<&PerCandidate<f64>> = None;
        let mut last_paper_total : Option<&PerCandidate<usize>> = None;
        for count_index in 0..self.official.counts.len() {
            let count = &self.official.counts[count_index];
            let count_index=CountIndex(count_index);
            if let Some(vote_delta) = count.vote_delta.as_ref() {
                if vote_delta.candidate.len()!=self.num_candidates { return Err(IssueWithOfficialDOPTranscript::WrongNumberOfCandidatesVoteDelta(count_index))}
            }
            if let Some(vote_total) = count.vote_total.as_ref() {
                if vote_total.candidate.len()!=self.num_candidates { return Err(IssueWithOfficialDOPTranscript::WrongNumberOfCandidatesVoteTotal(count_index))}
                if let Some(vote_delta) = count.vote_delta.as_ref() {
                    for c in 0..self.num_candidates {
                        if vote_total.candidate[c]!=vote_delta.candidate[c] + last_vote_total.map(|t|t.candidate[c]).unwrap_or(0.0) { return Err(IssueWithOfficialDOPTranscript::VoteSumIncorrect(count_index,CandidateIndex(c))); }
                    }
                    if vote_total.exhausted!=vote_delta.exhausted+last_vote_total.map(|t|t.exhausted).unwrap_or(0.0) { return Err(IssueWithOfficialDOPTranscript::VoteSumIncorrectExhausted(count_index)); }
                    if vote_total.rounding.resolve()!=vote_delta.rounding.resolve()+last_vote_total.map(|t|t.rounding.resolve()).unwrap_or(0.0) { return Err(IssueWithOfficialDOPTranscript::VoteSumIncorrectLostToRounding(count_index)); }
                    if let Some(total_set_aside) = vote_total.set_aside {
                        if total_set_aside!=last_vote_total.and_then(|t|t.set_aside).unwrap_or(0.0)+vote_delta.set_aside.unwrap_or(0.0) { return Err(IssueWithOfficialDOPTranscript::VoteSumIncorrectSetAside(count_index)); }
                    }
                }
            }
            last_vote_total = count.vote_total.as_ref();
            if let Some(paper_delta) = count.paper_delta.as_ref() {
                if paper_delta.candidate.len()!=self.num_candidates { return Err(IssueWithOfficialDOPTranscript::WrongNumberOfCandidatesPaperDelta(count_index))}
            }
            if let Some(paper_total) = count.paper_total.as_ref() {
                if paper_total.candidate.len()!=self.num_candidates { return Err(IssueWithOfficialDOPTranscript::WrongNumberOfCandidatesPaperTotal(count_index))}
                if let Some(paper_delta) = count.paper_delta.as_ref() {
                    for c in 0..self.num_candidates {
                        if paper_total.candidate[c] as isize!=paper_delta.candidate[c] + (last_paper_total.map(|t|t.candidate[c]).unwrap_or(0) as isize) { return Err(IssueWithOfficialDOPTranscript::PaperSumIncorrect(count_index,CandidateIndex(c))); }
                    }
                    if paper_total.exhausted as isize!=paper_delta.exhausted+(last_paper_total.map(|t|t.exhausted).unwrap_or(0) as isize) { return Err(IssueWithOfficialDOPTranscript::PaperSumIncorrectExhausted(count_index)); }
                    if let Some(total_set_aside) = paper_total.set_aside {
                        if total_set_aside as isize!=(last_paper_total.and_then(|t|t.set_aside).unwrap_or(0) as isize)+paper_delta.set_aside.unwrap_or(0) { return Err(IssueWithOfficialDOPTranscript::PaperSumIncorrectSetAside(count_index)); }
                    }
                }
            }
            last_paper_total = count.paper_total.as_ref();
        }
        Ok(())
    }

    fn check_first_preferences_count(&self) -> Result<(),IssueWithOfficialDOPTranscript<Rules::Tally>> {
        if let Some(vote_delta) = self.official.counts[0].vote_delta.as_ref() {
            if self.metadata.excluded.is_empty() && vote_delta.exhausted!=0.0 { return Err(IssueWithOfficialDOPTranscript::ExhaustedAtFirstPrefCount)}
            if vote_delta.set_aside.unwrap_or_default()!=0.0 { return Err(IssueWithOfficialDOPTranscript::SetAsideAtFirstPrefCount)}
            if vote_delta.rounding.resolve()!=0.0 { return Err(IssueWithOfficialDOPTranscript::LostToRoundingAtFirstPrefCount)}
        }
        if let Some(vote_total) = self.official.counts[0].vote_total.as_ref() {
            if self.metadata.excluded.is_empty() && vote_total.exhausted!=0.0 { return Err(IssueWithOfficialDOPTranscript::ExhaustedAtFirstPrefCount)}
            if vote_total.set_aside.unwrap_or_default()!=0.0 { return Err(IssueWithOfficialDOPTranscript::SetAsideAtFirstPrefCount)}
            if vote_total.rounding.resolve()!=0.0 { return Err(IssueWithOfficialDOPTranscript::LostToRoundingAtFirstPrefCount)}
        }
        if let Some(paper_delta) = self.official.counts[0].paper_delta.as_ref() {
            if self.metadata.excluded.is_empty() && paper_delta.exhausted!=0 { return Err(IssueWithOfficialDOPTranscript::ExhaustedAtFirstPrefCount)}
            if paper_delta.set_aside.unwrap_or_default()!=0 { return Err(IssueWithOfficialDOPTranscript::SetAsideAtFirstPrefCount)}
            if paper_delta.rounding.resolve()!=0 { return Err(IssueWithOfficialDOPTranscript::LostToRoundingAtFirstPrefCount)}
        }
        if let Some(paper_total) = self.official.counts[0].paper_total.as_ref() {
            if self.metadata.excluded.is_empty() && paper_total.exhausted!=0 { return Err(IssueWithOfficialDOPTranscript::ExhaustedAtFirstPrefCount)}
            if paper_total.set_aside.unwrap_or_default()!=0 { return Err(IssueWithOfficialDOPTranscript::SetAsideAtFirstPrefCount)}
            if paper_total.rounding.value!=0 { return Err(IssueWithOfficialDOPTranscript::LostToRoundingAtFirstPrefCount)}
        }
        Ok(())
    }
/*
    pub fn resort_candidates(&mut self) {
        let tallies = &self.tallys;
        let key = |c:&CandidateIndex|tallies[c.0].clone();
        self.continuing_candidates_sorted_by_tally.sort_by_key(key);
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
        if self.print_progress_to_stdout { println!("Elected {}", self.data.metadata.candidate(who).name); }
        self.elected_candidates.push(who);
        self.transcript.elected.push(who);
        self.no_longer_continuing(who,true);
    }
*/

}


pub fn veryify_official_dop_transcript<Rules:PreferenceDistributionRules>(official:&OfficialDistributionOfPreferencesTranscript,metadata:&ElectionMetadata) -> Result<(),IssueWithOfficialDOPTranscript<Rules::Tally>> {
    let work: VerifyOfficialDopTranscript<'_, Rules> = VerifyOfficialDopTranscript::new(official,metadata)?;
    work.check_basic_arithmetic_adding_deltas_to_totals()?;
    work.check_first_preferences_count()?;
    println!("{} {}",work.quota,work.first_preference_votes);
    // do the STV algorithm.
    Ok(())
}


pub fn distribute_preferences_using_official_results<Rules:PreferenceDistributionRules>(official:&OfficialDistributionOfPreferencesTranscript,metadata:&ElectionMetadata) -> Result<Transcript<Rules::Tally>,IssueWithOfficialDOPTranscript<Rules::Tally>> {
    if official.counts.is_empty() { return Err(IssueWithOfficialDOPTranscript::DoesntHaveFirstCount)}
    let excluded_candidates : HashSet<CandidateIndex> = metadata.excluded.iter().cloned().collect();
    let print_progress_to_stdout:bool = false;
    let num_candidates = metadata.candidates.len();
    // check quota
    let candidates_to_be_elected = metadata.vacancies.ok_or_else(||IssueWithOfficialDOPTranscript::MetadataMissingVacancies)?;


    // set up votes just as first preferences based on first preference distribution.
    let first_preference_vote_source = official.counts[0].vote_total.as_ref().or(official.counts[0].vote_delta.as_ref()).ok_or_else(||IssueWithOfficialDOPTranscript::FirstCountHasNoVotes)?;
    let mut btl = vec![];
    for c in 0..num_candidates {
        let votes = first_preference_vote_source.candidate[c];
        if votes!=votes.floor() { return Err(IssueWithOfficialDOPTranscript::FirstPreferenceVoteCountNotInteger) }
        btl.push(BTL{ candidates: vec![CandidateIndex(c)], n: votes as usize })
    }
    // deal with exhausted votes (due to disqualification of a candidate)
    {
        let votes = first_preference_vote_source.exhausted;
        if votes!=votes.floor() { return Err(IssueWithOfficialDOPTranscript::FirstPreferenceVoteCountNotInteger) }
        btl.push(BTL{ candidates: vec![], n: votes as usize })
    }
    let data = ElectionData{
        metadata: metadata.clone(),
        atl: vec![],
        atl_types: vec![],
        btl,
        btl_types: vec![],
        informal: 0,
    };
    let ec_resolutions = metadata.tie_resolutions.clone(); // TODO make EC resolutions correct.
    let arena = typed_arena::Arena::<CandidateIndex>::new();
    let votes = data.resolve_atl(&arena,None);
    let oracle = OracleFromOfficialDOP{official, tie_resolutions: Default::default() };
    let mut work : PreferenceDistributor<'_,Rules> = PreferenceDistributor::new(&data,&votes,candidates_to_be_elected,&excluded_candidates,&ec_resolutions,print_progress_to_stdout,Some(oracle));
    work.go();
    Ok(work.transcript)
}

pub struct OracleFromOfficialDOP<'a> {
    official:&'a OfficialDistributionOfPreferencesTranscript,
    // this is built up as they are encountered.
    tie_resolutions : TieResolutionsMadeByEC,
}

impl <'a> OracleFromOfficialDOP<'a> {
    /// The Oracle declares that verily votes shall be distributed as it shall say.
    pub fn get_distribution_by_candidate(&mut self, current_count:CountIndex) -> Option<Vec<BallotPaperCount>> {
        if current_count.0>=self.official.counts.len() { return None }
        let count = &self.official.counts[current_count.0];
        if let Some(paper_delta) = &count.paper_delta {
            let mut res : Vec<BallotPaperCount> = paper_delta.candidate.iter().chain(iter::once(&paper_delta.exhausted)).map(|v|if *v>=0 {BallotPaperCount(*v as usize)} else {BallotPaperCount(0)}).collect();
            if let Some(paper_set_aside) = &count.paper_set_aside_for_quota {
                for (candidate_index,set_aside) in paper_set_aside.candidate.iter().enumerate() {
                    if *set_aside!=usize::MAX {
                        res[candidate_index]+=BallotPaperCount(*set_aside);
                    }
                }
                *res.last_mut().unwrap()+=BallotPaperCount(paper_set_aside.exhausted);
            }
            Some(res)
        } else {
            None
        }
    }

    pub fn resolve_tie_resolution(&mut self, current_count:CountIndex,granularity:TieResolutionGranularityNeeded,tied_candidates:&[CandidateIndex]) -> Option<TieResolutionAtom> {
        if current_count.0>self.official.counts.len() { return None }
        let count = &self.official.counts[current_count.0];
        let mut elected_this_count : Vec<CandidateIndex> = count.elected.iter().filter(|c|tied_candidates.contains(c)).cloned().collect();
        let mut excluded_this_count : Vec<CandidateIndex> = count.excluded.iter().filter(|c|tied_candidates.contains(c)).cloned().collect();
        let mut remaining : Vec<CandidateIndex> = tied_candidates.iter().filter(|c|!(elected_this_count.contains(*c)||excluded_this_count.contains(*c))).cloned().collect();
        assert_eq!(tied_candidates.len(),elected_this_count.len()+excluded_this_count.len()+remaining.len());
        let decision = match granularity {
            TieResolutionGranularityNeeded::Total => {
                if remaining.len()>1 { return None } // not enough information
                let mut favoured = vec![];
                for c in excluded_this_count { favoured.push(c) }
                for c in remaining { favoured.push(c) }
                for &c in elected_this_count.iter().rev() { favoured.push(c) }
                TieResolutionAtom::IncreasingFavour(favoured)
            }
            TieResolutionGranularityNeeded::LowestSeparated(n) => {
                if excluded_this_count.len()==n {
                    elected_this_count.append(&mut remaining);
                } else if excluded_this_count.len()+remaining.len()==n {
                    excluded_this_count.append(&mut remaining);
                } else { return None }
                TieResolutionAtom::ExplicitDecision(TieResolutionExplicitDecisionInCount {
                    decision: TieResolutionExplicitDecision::two_lists(excluded_this_count,elected_this_count),
                    came_up_in: Some(current_count),
                })
            }
        };
        self.tie_resolutions.tie_resolutions.push(decision.clone());
        Some(decision)
    }
    /*
    pub fn add_vote(&self,old_prefs:&[CandidateIndex],add:CandidateIndex) -> &[CandidateIndex] {
        let mut v = old_prefs.to_vec();
        v.push(add);
        self.arena.alloc_extend(v)
    }*/
}