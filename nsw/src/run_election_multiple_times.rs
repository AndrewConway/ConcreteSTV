// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Functions to run randomized elections lots of time and see if it changes what happens.



use std::fmt::{Debug, Display};
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use stv::ballot_metadata::{CandidateIndex, ElectionMetadata};
use stv::distribution_of_preferences_transcript::Transcript;
use stv::election_data::ElectionData;
use stv::preference_distribution::PreferenceDistributionRules;
use stv::random_util::Randomness;

pub struct PossibleResults {
    /// The total number of times the election is run.
    pub num_runs : usize,
    /// length equal to the number of candidates, info on who got elected
    pub candidates : Vec<ResultForACandidate>,
}

impl PossibleResults {
    pub fn new(num_candidates: usize) -> Self {
        let mut candidates = Vec::with_capacity(num_candidates);
        for candidate in 0..num_candidates { candidates.push(ResultForACandidate { candidate: CandidateIndex(candidate), num_times_elected: 0, sum_of_elected_positions: 0 }) }
        PossibleResults { num_runs: 0, candidates }
    }
    /// Add the information that the run has occurred.
    pub fn add_run<U: PartialEq + Clone + Display + FromStr + Debug>(&mut self, result: &Transcript<U>) {
        self.num_runs += 1;
        for (i, &candidate) in result.elected.iter().enumerate() {
            self.candidates[candidate.0].num_times_elected += 1;
            self.candidates[candidate.0].sum_of_elected_positions += 1 + i;
        }
    }
    /// Run the election some number of times, and add each
    pub fn add_run_times<R: PreferenceDistributionRules>(&mut self, data: &ElectionData, times: usize,randomness:&mut Randomness) {
        for _ in 0..times {
            let result = data.distribute_preferences::<R>(randomness);
            self.add_run(&result);
        }
    }
    /// Create a new PossibleResults structure from running the rules a given number of times.
    pub fn new_from_runs<R: PreferenceDistributionRules>(data: &ElectionData, times: usize,randomness:&mut Randomness) -> Self {
        let mut res = PossibleResults::new(data.metadata.candidates.len());
        res.add_run_times::<R>(data, times,randomness);
        res
    }
    /// add in other to the cumulative sum of self.
    pub fn merge(&mut self, other: &PossibleResults) {
        self.num_runs += other.num_runs;
        for i in 0..self.candidates.len() {
            self.candidates[i].merge(&other.candidates[i]);
        }
    }
    /// Create a new PossibleResults structure from running the rules a given number of times, split amongst num_threads threads.
    pub fn new_from_runs_multithreaded<R: PreferenceDistributionRules>(data: &ElectionData, times: usize, num_threads: usize) -> Self {
        let mut handles = vec![];
        let data = Arc::new(data.clone());
        for thread_no in 0..num_threads {
            let num_to_do = times / num_threads + (if times % num_threads > thread_no { 1 } else { 0 });
            let data = data.clone();
            let handle = thread::spawn(move || {
                let mut rng = Randomness::PRNG(ChaCha20Rng::seed_from_u64(thread_no as u64));
                Self::new_from_runs::<R>(&data, num_to_do,&mut rng)
            });
            handles.push(handle);
        }
        let mut res = PossibleResults::new(data.metadata.candidates.len());
        for handle in handles {
            let partial = handle.join().unwrap();
            res.merge(&partial);
        }
        res
    }

    pub fn possible_winners(&self) -> Vec<&ResultForACandidate> {
        let mut res: Vec<&ResultForACandidate> = self.candidates.iter().filter(|c| c.num_times_elected > 0).collect();
        // sort. Lowest are people elected the most times. In case of ties, people elected earlier go earlier.
        res.sort_unstable_by(|a, b| {
            b.num_times_elected.cmp(&a.num_times_elected).then_with(|| a.sum_of_elected_positions.cmp(&b.sum_of_elected_positions))
        });
        res
    }
    pub fn print_table_results(&self, metadata: &ElectionMetadata) {
        for winner in self.possible_winners() {
            println!("{:>4} {:>20} {:>8} {:>9.6}", winner.candidate, metadata.candidate(winner.candidate).name, winner.num_times_elected, winner.mean_position_elected());
        }
    }
    /// See if the expected probability of winning is close to the actual observed. Actually checks the number of standard deviations of a binomial distribution is < 5 away from expected, which will almost always be the case. Special case for 1.0 prob winning requires exact match.
    pub fn is_close_to_expected_prob_winning(&self, candidate: CandidateIndex, expected_prob_winning: f64) -> bool {
        self.candidates[candidate.0].is_close_to_expected_prob_winning(expected_prob_winning, self.num_runs)
    }
}

pub struct ResultForACandidate {
    pub candidate : CandidateIndex,
    /// The number of times this candidate has been elected.
    pub num_times_elected : usize,
    /// The sum of positions elected a candidate is (1 is first position). Mean position is this divided by num_times_elected.
    pub sum_of_elected_positions : usize,
}

impl ResultForACandidate {

    pub fn mean_position_elected(&self) -> f64 { (self.sum_of_elected_positions as f64)/(self.num_times_elected as f64) }

    fn merge(&mut self,other:&ResultForACandidate) {
        assert_eq!(self.candidate,other.candidate);
        self.num_times_elected+=other.num_times_elected;
        self.sum_of_elected_positions+=other.sum_of_elected_positions;
    }

    /// See if the expected probability of winning is close to the actual observed. Actually checks the number of standard deviations of a binomial distribution is < 5 away from expected, which will almost always be the case. Special case for 1.0 prob winning requires exact match.
    fn is_close_to_expected_prob_winning(&self,expected_prob_winning:f64,num_runs:usize) -> bool {
        // assume a binomial position.
        let expected_num_wins : f64 = expected_prob_winning*num_runs as f64;
        let expected_sd : f64 = f64::sqrt(expected_num_wins*(1.0-expected_prob_winning));
        let diff = self.num_times_elected as f64 - expected_num_wins;
        let sigmas : f64 = if expected_sd==0.0 { if diff==0.0 {0.0} else {f64::INFINITY} } else { diff/expected_sd};
        println!("Candidate {} expected={:.1} actual={} diff={:.1} sd={:.1} sigmas={:.1}",self.candidate,expected_num_wins,self.num_times_elected,diff,expected_sd,sigmas);
        sigmas.abs()<5.0
    }

}

