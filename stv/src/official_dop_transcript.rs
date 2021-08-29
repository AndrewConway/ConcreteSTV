// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use crate::distribution_of_preferences_transcript::{PerCandidate, QuotaInfo, Transcript};
use crate::ballot_metadata::CandidateIndex;
use std::cmp::min;
use num::{abs, Zero};
use std::ops::Sub;
use std::fmt::Display;
use crate::ballot_pile::BallotPaperCount;

/// Information for a particular count from the official transcript.
#[derive(Default)]
pub struct OfficialDOPForOneCount {
    pub transfer_value : Option<f64>,
    pub elected : Vec<CandidateIndex>,
    pub excluded : Vec<CandidateIndex>,
    pub vote_total : Option<PerCandidate<f64>>,
    pub paper_total : Option<PerCandidate<usize>>,
    pub vote_delta : Option<PerCandidate<f64>>,
    pub paper_delta : Option<PerCandidate<isize>>,
}

/// Information from
#[derive(Default)]
pub struct OfficialDistributionOfPreferencesTranscript {
    pub quota : Option<QuotaInfo<f64>>,
    pub counts : Vec<OfficialDOPForOneCount>,
}

impl OfficialDOPForOneCount {
    pub fn vote_total(&mut self) -> &mut PerCandidate<f64> { self.vote_total.get_or_insert_with(Default::default) }
    pub fn paper_total(&mut self) -> &mut PerCandidate<usize> { self.paper_total.get_or_insert_with(Default::default) }
    pub fn vote_delta(&mut self) -> &mut PerCandidate<f64> { self.vote_delta.get_or_insert_with(Default::default) }
    pub fn paper_delta(&mut self) -> &mut PerCandidate<isize> { self.paper_delta.get_or_insert_with(Default::default) }
}

impl OfficialDistributionOfPreferencesTranscript {
    /// Initialize a new count
    pub fn finished_count(&mut self) { self.counts.push(OfficialDOPForOneCount::default())}
    /// Get the current count
    pub fn count(&mut self) -> &mut OfficialDOPForOneCount { self.counts.last_mut().unwrap() }

    /// Compare the results from the official transcript to our transcript.
    /// panic if there are differences.
    pub fn compare_with_transcript<Tally:Clone+Zero+PartialEq+Sub<Output=Tally>+Display,F:Fn(Tally)->f64>(&self,transcript:&Transcript<Tally>,decode:F) {
        if let Some(quota) = &self.quota {
            assert_eq!(quota.vacancies,transcript.quota.vacancies);
            assert_eq!(quota.papers,transcript.quota.papers);
            assert_eq!(quota.quota,decode(transcript.quota.quota.clone()));
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
                if official!=our {
                    panic!("Count {} Official result {} our result {} for {} candidate {}",i+1,official,our,what,who)
                }
            };
            let assert_close = |official:f64,our:Tally,what:&str|{
                let our_f64=decode(our);
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
            println!("Checking count {}",i+1);
            let my_count = &transcript.counts[i];
            let official_count = &self.counts[i];
            assert_eq!(official_count.elected,my_count.elected.iter().map(|e|e.who).collect::<Vec<CandidateIndex>>());
            for who in &official_count.excluded {
                assert!(my_count.not_continuing.contains(who),"{} was not in the list of not continuing",who);
            }
            if let Some(vote_total) = &official_count.vote_total {
                println!("Checking tally count {}",i+1);
                assert_close(vote_total.exhausted,my_count.status.tallies.exhausted.clone(),"exhausted tallies");
                assert_close(vote_total.rounding,my_count.status.tallies.rounding.clone(),"rounding tallies");
                for candidate in 0..vote_total.candidate.len() {
                    assert_close_candidate(vote_total.candidate[candidate],my_count.status.tallies.candidate[candidate].clone(),"tally",CandidateIndex(candidate));
                }
            }
            if let Some(vote_delta) = &official_count.vote_delta {
                println!("Checking tally delta count {}",i+1);
                assert_close_delta(vote_delta.exhausted,my_count.status.tallies.exhausted.clone(),if i>0 { transcript.counts[i-1].status.tallies.exhausted.clone()} else {Tally::zero()},"exhausted delta tally");
                assert_close_delta(vote_delta.rounding,my_count.status.tallies.rounding.clone(),if i>0 { transcript.counts[i-1].status.tallies.rounding.clone()} else {Tally::zero()},"rounding delta tally");
                for candidate in 0..vote_delta.candidate.len() {
                    assert_close_candidate_delta(vote_delta.candidate[candidate],my_count.status.tallies.candidate[candidate].clone(),if i>0 { transcript.counts[i-1].status.tallies.candidate[candidate].clone()} else {Tally::zero()},"tally delta",CandidateIndex(candidate));
                }
            }
            if let Some(paper_total) = &official_count.paper_total {
                println!("Checking paper count {}",i+1);
                assert_papers(paper_total.exhausted,my_count.status.papers.exhausted.clone(),"exhausted papers");
                assert_papers(paper_total.rounding,my_count.status.papers.rounding.clone(),"rounding papers");
                for candidate in 0..paper_total.candidate.len() {
                    assert_papers_candidate(paper_total.candidate[candidate],my_count.status.papers.candidate[candidate].clone(),"papers",CandidateIndex(candidate));
                }
            }
            if let Some(paper_delta) = &official_count.paper_delta {
                println!("Checking paper delta {}",i+1);
                assert_papers_delta(paper_delta.exhausted,my_count.status.papers.exhausted.clone(),if i>0 { transcript.counts[i-1].status.papers.exhausted.clone()} else {BallotPaperCount(0)},"exhausted papers delta");
                assert_papers_delta(paper_delta.rounding,my_count.status.papers.rounding.clone(),if i>0 { transcript.counts[i-1].status.papers.rounding.clone()} else {BallotPaperCount(0)},"rounding papers delta");
                for candidate in 0..paper_delta.candidate.len() {
                    assert_papers_candidate_delta(paper_delta.candidate[candidate],my_count.status.papers.candidate[candidate].clone(),if i>0 { transcript.counts[i-1].status.papers.candidate[candidate].clone()} else {BallotPaperCount(0)},"papers delta",CandidateIndex(candidate));
                }
            }
        }
        assert_eq!(self.counts.len(),transcript.counts.len())
    }
}

/// Given a vector, make sure the array is long enough to hold the person's entry, and return a mutable reference to it.
pub fn candidate_elem<T:Default+Clone>(vec : &mut Vec<T>, who:CandidateIndex) -> &mut T {
    if vec.len()<=who.0 {
        vec.resize(who.0+1,T::default())
    }
    &mut vec[who.0]
}



