// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use stv::election_data::ElectionData;
use serde::{Serialize,Deserialize};
use stv::ballot_metadata::CandidateIndex;
use stv::ballot_pile::BallotPaperCount;

#[derive(Debug,Serialize,Deserialize,Clone)]
/// Square matrix with some function of (candidate,candidate) or (party,party).
pub struct SquareMatrix {
    pub matrix : Vec<Vec<f64>>,
    pub first_preferences : Vec<BallotPaperCount>,
}

#[derive(Debug,Serialize,Deserialize,Clone,Copy)]
/// Options for doing correlations.
pub struct CorrelationOptions {
    /// if true, then want to correlate candidates. If false, want to correlate parties.
    pub want_candidates : bool,
    /// if true, use ATL votes in correlations
    pub use_atl : bool,
    /// if true, use BTL votes
    pub use_btl : bool,
    /// if true, do mean subtraction from vectors before correlating.
    pub subtract_mean : bool,
}

impl SquareMatrix {
    /// Get a correlation matrix between how people vote for different groups.
    ///
    /// Suppose voter i gives preference p_i,j to candidate/group j.
    /// * Then p_j is the vector of all preferences for that candidate.
    /// * The element a,b of the correlation matrix is (p_a . p_b)/(|p_a||p_b|)
    /// * If meanSubtractedCorrelations is true, then the mean value of each vector
    ///   p_x is subtracted from each element of p_x before the above formula (this is generally recommended).
    ///
    /// It also computes the number of first preference votes for each candidate/group depending upon the options.
    pub fn compute_correlation_matrix(data:&ElectionData, options:CorrelationOptions) -> SquareMatrix {
        let n = if options.want_candidates { data.metadata.candidates.len() }  else { data.metadata.parties.len() };
        let mut self_dot_product = vec![0.0;n];
        let mut first_preferences = vec![BallotPaperCount(0);n];
        let mut sums = vec![0.0;n];
        let mut cross = vec![vec![0.0;n];n];
        let mut count = 0.0;
        let mut vote_vector_by_candidate = vec![f64::NAN;data.metadata.candidates.len()];
        let mut vote_vector_by_group = vec![f64::NAN;data.metadata.parties.len()];
        let arena = typed_arena::Arena::<CandidateIndex>::new();
        let votes = data.resolve_atl(&arena);
        for vote in votes {
            if vote.prefs.len()>0 && if vote.is_atl() { options.use_atl } else { options.use_btl } {
                let w = vote.n.0 as f64;
                count+=w;
                let first_candidate = vote.prefs[0];
                if options.want_candidates { first_preferences[first_candidate.0]+=vote.n; }
                else if let Some(first_party) = data.metadata.candidate(first_candidate).party { first_preferences[first_party.0]+=vote.n; }
                // Set vote_vector_by_candidate[i] to be the preference given to candidate i starting with 1. Blanks are assigned a mean of remaining preferences.
                {
                    let num_blank = vote_vector_by_candidate.len()-vote.prefs.len();
                    if num_blank>0 {
                        let mean_blank = vote_vector_by_candidate.len() as f64 - (num_blank-1) as f64*0.5;
                        for e in &mut vote_vector_by_candidate { *e=mean_blank; }
                    }
                    for (index,&candidate) in vote.prefs.iter().enumerate() {
                        vote_vector_by_candidate[candidate.0]=(1+index) as f64;
                    }
                }
                // determine what vector p we want to correlate.
                let p : &[f64] = if options.want_candidates { &vote_vector_by_candidate } else {
                    // average vote_vector_by_candidate by group
                    for (group_index,party) in data.metadata.parties.iter().enumerate() {
                        vote_vector_by_group[group_index]=party.candidates.iter().map(|c|vote_vector_by_candidate[c.0]).sum::<f64>()/(party.candidates.len() as f64);
                    }
                    &vote_vector_by_group
                };
                for i in 0..n {
                    self_dot_product[i]+=w*p[i]*p[i];
                    sums[i]+=w*p[i];
                    for j in 0..i {
                        cross[j][i]+=w*p[j]*p[i];
                    }
                }
            }
        }
        if count > 0.0 {
            let weight = |i:usize| 1.0/f64::sqrt(self_dot_product[i]-if options.subtract_mean { sums[i]*sums[i]/count } else {0.0});
            let weights : Vec<f64> = (0..n).map(weight).collect();
            for i in 0..n {
                cross[i][i]=1.0;
                for j in 0..i {
                    let correlation = (cross[j][i]-if options.subtract_mean { sums[i]*sums[j]/count } else {0.0})*weights[i]*weights[j];
                    cross[j][i]=correlation;
                    cross[i][j]=correlation;
                }
            }

        }
        SquareMatrix{ matrix: cross, first_preferences }
    }




    /// convert to a distance matrix by converting each element from x to 1-x.
    /// So a perfect correlation of 1 becomes a distance of 0, no correlation becomes a distance of 1, and a perfect anticorrelation becomes a distance of 2.
    pub fn to_distance_matrix(mut self) -> Self {
        for row in &mut self.matrix {
            for e in row {
                *e=1.0-*e;
            }
        }
        self
    }
}