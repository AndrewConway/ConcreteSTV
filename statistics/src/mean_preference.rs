// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use stv::election_data::ElectionData;
use serde::{Serialize, Deserialize};
use stv::ballot_metadata::CandidateIndex;
use stv::ballot_pile::{BallotPaperCount, PartiallyDistributedVote};

#[derive(Debug,Serialize,Deserialize,Clone)]
/// The mean preference assigned to each candidate.
pub struct MeanPreferenceByCandidate {
    pub num_atl : BallotPaperCount,
    pub num_btl : BallotPaperCount,
    /// mean_position[i] = average preference assigned to candidate i. Unassigned counts as the mean of the remaining votes.
    pub mean_preference: Vec<f64>,
}

impl MeanPreferenceByCandidate {
    fn zero(num_candidates:usize) -> Self {
        MeanPreferenceByCandidate {
            num_atl: BallotPaperCount(0),
            num_btl: BallotPaperCount(0),
            mean_preference: vec![0.0; num_candidates],
        }
    }
    /// The unnormalized version of a mean preference has mean_position be sum_position.
    /// Add a vote to it is this situation.
    fn add_to_unnormalized(&mut self,vote:PartiallyDistributedVote<'_>) {
        let num_candidates = self.mean_preference.len();
        if vote.is_atl() { self.num_atl+=vote.n } else { self.num_btl+=vote.n; }
        let blank_surrogate_preference = (num_candidates+1+vote.prefs.len()) as f64*0.5;
        let n = vote.n.0 as f64;
        let no_vote_given_score = blank_surrogate_preference*n;
        // the easiest way to work out who doesn't have a preference is to give the blank score to everyone, then subtract it from those who don't deserve it.
        for i in 0..num_candidates { self.mean_preference[i]+=no_vote_given_score; }
        for (i,&who) in vote.prefs.iter().enumerate() {
            self.mean_preference[who.0]+=(i+1) as f64*n-no_vote_given_score;
        }
    }

    fn normalize(&mut self) {
        let n = self.num_btl.0 + self.num_atl.0;
        if n>0 {
            let mul = 1.0 / n as f64;
            for p in &mut self.mean_preference { *p *= mul; }
        }
    }
}

#[derive(Debug,Serialize,Deserialize,Clone)]
/// mean preferences assigned to candidates, depending upon who the first preference was.
pub struct MeanPreferences {
    pub all : MeanPreferenceByCandidate,
    /// all_by_first_preference[i] = the mean for votes whose first preference is for candidate i.
    pub all_by_first_preference : Vec<MeanPreferenceByCandidate>,
    /// just BTL votes
    pub btl : MeanPreferenceByCandidate,
    /// all_by_first_preference[i] = the mean for BTL votes whose first preference is for candidate i.
    pub btl_by_first_preference : Vec<MeanPreferenceByCandidate>,
}

impl MeanPreferences {
    pub fn compute(data:&ElectionData) -> Self {
        let num_candidates = data.metadata.candidates.len();
        let mut res = MeanPreferences {
            all: MeanPreferenceByCandidate::zero(num_candidates),
            all_by_first_preference: vec![MeanPreferenceByCandidate::zero(num_candidates);num_candidates],
            btl: MeanPreferenceByCandidate::zero(num_candidates),
            btl_by_first_preference: vec![MeanPreferenceByCandidate::zero(num_candidates);num_candidates]
        };
        let arena = typed_arena::Arena::<CandidateIndex>::new();
        let votes = data.resolve_atl(&arena,None);
        for vote in votes {
            res.add_to_unnormalized(vote);
        }
        res.normalize();
        res
    }
    fn add_to_unnormalized(&mut self,vote:PartiallyDistributedVote<'_>) {
        self.all.add_to_unnormalized(vote);
        self.all_by_first_preference[vote.prefs[0].0].add_to_unnormalized(vote);
        if vote.is_atl() {
            // do nothing
        } else {
            self.btl.add_to_unnormalized(vote);
            self.btl_by_first_preference[vote.prefs[0].0].add_to_unnormalized(vote);
        }
    }
    fn normalize(&mut self) {
        self.all.normalize();
        for s in &mut self.all_by_first_preference { s.normalize(); }
        self.btl.normalize();
        for s in &mut self.btl_by_first_preference { s.normalize(); }
    }
}