// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use crate::distribution_of_preferences_transcript::{CountIndex, PerCandidate, QuotaInfo, Transcript};
use crate::ballot_metadata::CandidateIndex;
use std::cmp::min;
use std::collections::HashSet;
use num::{abs, Zero};
use std::ops::Sub;
use std::fmt::Display;
use std::num::ParseIntError;
use crate::ballot_pile::BallotPaperCount;
use crate::signed_version::SignedVersion;
use std::str::FromStr;
use crate::tie_resolution::TieResolutionExplicitDecision;

/// Information for a particular count from the official transcript.
#[derive(Default)]
pub struct OfficialDOPForOneCount {
    pub transfer_value : Option<f64>,
    pub elected : Vec<CandidateIndex>,
    pub excluded : Vec<CandidateIndex>,
    pub vote_total : Option<PerCandidate<f64>>, // A NaN means unknown
    pub paper_total : Option<PerCandidate<usize>>, // an isize::MAX means unknown
    pub vote_delta : Option<PerCandidate<f64>>, // A NaN means unknown
    pub paper_delta : Option<PerCandidate<isize>>, // an isize::MAX means unknown
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
    pub fn finished_count(&mut self) { self.counts.push(OfficialDOPForOneCount::default())}
    /// Get the current count
    pub fn count(&mut self) -> &mut OfficialDOPForOneCount { self.counts.last_mut().unwrap() }

    /// Gets all elected candidates.
    pub fn all_elected(&self) -> Vec<CandidateIndex> {
        self.counts.iter().flat_map(|c|c.elected.iter()).cloned().collect()
    }
    /// Compare the results from the official transcript to our transcript.
    /// panic if there are differences.
    pub fn compare_with_transcript<Tally:Clone+Zero+PartialEq+Sub<Output=Tally>+Display+FromStr+CanConvertToF64PossiblyLossily>(&self,transcript:&Transcript<Tally>) {
        let ec_decision = self.compare_with_transcript_checking_for_ec_decisions(transcript,true);
        if let Some(decision) = ec_decision {
            panic!("An EC decision was not made the way we expected: {:?} was favoured over {:?}",decision.favoured,decision.disfavoured);
        }
    }
    /// like compare_with_transcript but don't panic if the first difference is caused by a difference in EC decision making. If so, return the decision.
    pub fn compare_with_transcript_checking_for_ec_decisions<Tally:Clone+Zero+PartialEq+Sub<Output=Tally>+Display+FromStr+CanConvertToF64PossiblyLossily>(&self,transcript:&Transcript<Tally>,verbose:bool) -> Option<TieResolutionExplicitDecision> {
        fn decode<Tally : CanConvertToF64PossiblyLossily>(tally:Tally) -> f64 { tally.convert_to_f64() }
        if let Some(quota) = &self.quota {
            assert_eq!(quota.vacancies,transcript.quota.as_ref().unwrap().vacancies,"vacancies official vs me");
            assert_eq!(quota.papers,transcript.quota.as_ref().unwrap().papers,"papers with first preferences official vs me");
            assert_eq!(quota.quota,decode(transcript.quota.as_ref().unwrap().quota.clone()),"quota official vs me");
        }
        for i in 0..min(self.counts.len(),transcript.counts.len()) {
            let assert_papers = |official:usize,our:BallotPaperCount,what:&str|{
                if official!=our.0 {
                    panic!("Count {} Official result {} our result {} for {}",i+1,official,our,what)
                }
            };
            let assert_papers_delta = |official:isize,our1:BallotPaperCount,our_prev:BallotPaperCount,what:&str|{
                let our = our1.0 as isize - our_prev.0 as isize;
                if official!=our {
                    panic!("Count {} Official result {} our result {} for {}",i+1,official,our,what)
                }
            };
            let assert_papers_candidate = |official:usize,our:BallotPaperCount,what:&str,who:CandidateIndex|{
                if official!=our.0 {
                    panic!("Count {} Official result {} our result {} for {} candidate {}",i+1,official,our,what,who)
                }
            };
            let assert_papers_candidate_delta = |official:isize,our1:BallotPaperCount,our_prev:BallotPaperCount,what:&str,who:CandidateIndex|{
                let our = our1.0 as isize - our_prev.0 as isize;
                if official!=our && !(self.missing_negatives_in_papers_delta && our<0) {
                    panic!("Count {} Official result {} our result {} for {} candidate {}",i+1,official,our,what,who)
                }
            };
            let assert_close = |official:f64,our:Tally,what:&str|{
                let our_f64=decode(our);
                if abs(our_f64-official)>1e-7 {
                    panic!("Count {} Official result {} our result {} for {}",i+1,official,our_f64,what)
                }
            };
            let assert_close_signed = |official:f64,our:SignedVersion<Tally>,what:&str|{
                let our_f64=our.convert_f64(&decode);
                if abs(our_f64-official)>1e-7 {
                    panic!("Count {} Official result {} our result {} for {}",i+1,official,our_f64,what)
                }
            };
            let assert_close_delta = |official:f64,our1:Tally,our2:Tally,what:&str|{
                let our_f64=decode(our1)-decode(our2);
                if abs(our_f64-official)>1e-7 {
                    panic!("Count {} Official result {} our result {} for {}",i+1,official,our_f64,what)
                }
            };
            let assert_close_delta_signed = |official:f64,our1:SignedVersion<Tally>,our2:SignedVersion<Tally>,what:&str|{
                let our_f64=our1.convert_f64(&decode)-our2.convert_f64(&decode);
                if abs(our_f64-official)>1e-7 {
                    panic!("Count {} Official result {} our result {} for {}",i+1,official,our_f64,what)
                }
            };
            let assert_close_candidate = |official:f64,our:Tally,what:&str,who:CandidateIndex|{
                let our_f64=decode(our);
                if abs(our_f64-official)>1e-7 {
                    panic!("Count {} Official result {} our result {} for {} candidate {}",i+1,official,our_f64,what,who)
                }
            };
            let assert_close_candidate_delta = |official:f64,our1:Tally,our2:Tally,what:&str,who:CandidateIndex|{
                let our_f64=decode(our1)-decode(our2);
                if abs(our_f64-official)>1e-7 {
                    panic!("Count {} Official result {} our result {} for {} candidate {}",i+1,official,our_f64,what,who)
                }
            };
            let my_count = &transcript.counts[i];
            let official_count = &self.counts[i];
            if verbose { println!("Checking count {} {}",i+1,my_count.count_name.clone().unwrap_or_default()); }
            assert_eq!(my_count.count_name,official_count.count_name);
            if self.elected_candidates_are_in_order {
                assert_eq!(official_count.elected,my_count.elected.iter().map(|e|e.who).collect::<Vec<CandidateIndex>>());
            } else {
                assert_eq!(official_count.elected.iter().cloned().collect::<HashSet<CandidateIndex>>(),my_count.elected.iter().map(|e|e.who).collect::<HashSet<CandidateIndex>>());
            }
            for who in &official_count.excluded {
                if !my_count.not_continuing.contains(who) {
                    if let Some(relevant_decision) = my_count.decisions.iter().find(|d|d.affected.contains(who)) { // may exclude a different candidate because of random decisions.
                        // The EC excluded "who". Work out whom I excluded.
                        if let Some(&_who_was_lucky) = relevant_decision.affected.iter().find(|&c|my_count.not_continuing.contains(c)) {
                            // I excluded "who_was_lucky". They should have a higher priority than "who".
                            let favoured = relevant_decision.affected.iter().filter(|&&c|c!=*who).cloned().collect::<Vec<_>>();
                            return Some(TieResolutionExplicitDecision{favoured,disfavoured:vec![*who], came_up_in: my_count.count_name.clone().or_else(||Some((i+1).to_string())) });
                            // panic!("I excluded candidate {} but the EC excluded candidate {}. This was chosen by lot.",who_was_lucky,who);
                        } else {
                            panic!("{} was not in the list of not continuing. There was a relevant decision involving {:?} but I didn't exclude any.",who,relevant_decision.affected);
                        }
                    }
                    panic!("{} was in the official list of people stopping being continuing candidates for the first time in this round, but not in my list.",who);
                }
                assert!(my_count.not_continuing.contains(who),"{} was not in the list of not continuing",who);
            }
            if let Some(vote_total) = &official_count.vote_total {
                if verbose { println!("Checking tally count {}",i+1); }
                if self.all_exhausted_go_to_rounding {
                    assert_close(vote_total.rounding.assume_positive(),my_count.status.tallies.exhausted.clone()+my_count.status.tallies.rounding.assume_positive(),"votes lost to exhaustion or rounding");
                } else {
                    assert_close(vote_total.exhausted,my_count.status.tallies.exhausted.clone(),"exhausted tallies");
                    assert_close_signed(vote_total.rounding.clone().into(),my_count.status.tallies.rounding.clone(),"rounding tallies");
                }
                for candidate in 0..vote_total.candidate.len() {
                    assert_close_candidate(vote_total.candidate[candidate],my_count.status.tallies.candidate[candidate].clone(),"tally",CandidateIndex(candidate));
                }
            }
            if let Some(vote_delta) = &official_count.vote_delta {
                if verbose { println!("Checking tally delta count {}",i+1); }
                assert_close_delta(vote_delta.exhausted,my_count.status.tallies.exhausted.clone(),if i>0 { transcript.counts[i-1].status.tallies.exhausted.clone()} else {Tally::zero()},"exhausted delta tally");
                assert_close_delta_signed(vote_delta.rounding.clone().into(),my_count.status.tallies.rounding.clone(),if i>0 { transcript.counts[i-1].status.tallies.rounding.clone()} else {SignedVersion::from(Tally::zero())},"rounding delta tally");
                for candidate in 0..vote_delta.candidate.len() {
                    assert_close_candidate_delta(vote_delta.candidate[candidate],my_count.status.tallies.candidate[candidate].clone(),if i>0 { transcript.counts[i-1].status.tallies.candidate[candidate].clone()} else {Tally::zero()},"tally delta",CandidateIndex(candidate));
                }
            }
            if let Some(paper_total) = &official_count.paper_total {
                if verbose { println!("Checking paper count {}",i+1); }
                assert_papers(paper_total.exhausted,my_count.status.papers.exhausted.clone(),"exhausted papers");
                assert_papers(paper_total.rounding.assume_positive(),my_count.status.papers.rounding.assume_positive(),"rounding papers");
                for candidate in 0..paper_total.candidate.len() {
                    assert_papers_candidate(paper_total.candidate[candidate],my_count.status.papers.candidate[candidate].clone(),"papers",CandidateIndex(candidate));
                }
            }
            if let Some(paper_delta) = &official_count.paper_delta {
                if verbose { println!("Checking paper delta {}",i+1); }
                assert_papers_delta(paper_delta.exhausted,my_count.status.papers.exhausted.clone(),if i>0 { transcript.counts[i-1].status.papers.exhausted.clone()} else {BallotPaperCount(0)},"exhausted papers delta");
                assert_papers_delta(paper_delta.rounding.assume_positive(),my_count.status.papers.rounding.assume_positive(),if i>0 { transcript.counts[i-1].status.papers.rounding.assume_positive()} else {BallotPaperCount(0)},"rounding papers delta");
                for candidate in 0..paper_delta.candidate.len() {
                    assert_papers_candidate_delta(paper_delta.candidate[candidate],my_count.status.papers.candidate[candidate].clone(),if i>0 { transcript.counts[i-1].status.papers.candidate[candidate].clone()} else {BallotPaperCount(0)},"papers delta",CandidateIndex(candidate));
                }
            }
            if let Some(papers_came_from_counts) = &official_count.papers_came_from_counts {
                assert_eq!(papers_came_from_counts,&my_count.portion.papers_came_from_counts);
            }
        }
        assert_eq!(self.counts.len(),transcript.counts.len(),"A different number of counts are present.");
        None
    }
}

/// Given a vector, make sure the array is long enough to hold the person's entry, and return a mutable reference to it.
pub fn candidate_elem<T:Default+Clone>(vec : &mut Vec<T>, who:CandidateIndex) -> &mut T {
    if vec.len()<=who.0 {
        vec.resize(who.0+1,T::default())
    }
    &mut vec[who.0]
}



