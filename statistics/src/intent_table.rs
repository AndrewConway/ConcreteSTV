// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use stv::election_data::ElectionData;
use serde::{Serialize, Deserialize};
use stv::ballot_metadata::CandidateIndex;
use stv::ballot_pile::BallotPaperCount;

#[derive(Debug,Serialize,Deserialize,Clone)]
/// A table showing who people prefer of some specified candidates/parties given that their first preference is for some other given candidate/party.
/// table[i][j] is the number of ballot papers that (for some specific array of candidates/groups or interest *who*)
///  * have their first preference going to candidate(group) i.
///  * have candidate(group) who[j] in their preference list before any other member of who. If no member of who is in their preference list, j=who.len().
pub struct IntentTable {
    pub table : Vec<Vec<BallotPaperCount>>,
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct IntentTableOptions {
    /// if true, then the first preference distribution is by groups, otherwise by candidates.
    pub first_pref_by_groups : bool,
    /// if true, then the who array represents groups, otherwise candidates.
    pub who_is_groups : bool,
    /// if true, use ATL votes in correlations
    pub use_atl : bool,
    /// if true, use BTL votes
    pub use_btl : bool,
    /// who is of interest. If who_is_groups, then a GroupIndex, else a CandidateIndex.
    /// Serialized oddly to make it easy to work with a URL parameter.
    #[serde(deserialize_with = "crate::util::deserialize_stringified_usize_list",serialize_with="crate::util::serialize_stringified_usize_list")]
    pub who : Vec<usize>,
}

impl IntentTable {
    pub fn compute(data:&ElectionData,options:&IntentTableOptions) -> Self {
        let num_rows = if options.first_pref_by_groups { data.metadata.parties.len() } else { data.metadata.candidates.len() };
        let num_cols = options.who.len()+1;
        let exhausted_column = options.who.len();
        let mut table = vec![vec![BallotPaperCount(0);num_cols];num_rows];

        let mut candidate_to_who_index = vec![exhausted_column;data.metadata.candidates.len()]; // a map from candidate index to the candidate
        for who_index in 0..options.who.len() {
            let who = options.who[who_index];
            if options.who_is_groups {
                for &candidate in &data.metadata.parties[who].candidates {
                    candidate_to_who_index[candidate.0] = who_index;
                }
            } else { // who is candidates
                candidate_to_who_index[who]= who_index;
            }
        }
        let arena = typed_arena::Arena::<CandidateIndex>::new();
        let votes = data.resolve_atl(&arena);
        for vote in votes {
            if vote.prefs.len()>0 && if vote.is_atl() { options.use_atl } else { options.use_btl } {
                let first_preference_candidate = vote.prefs[0];
                if let Some(first_preference) = if options.first_pref_by_groups { data.metadata.candidate(first_preference_candidate).party.map(|p|p.0)} else { Some(first_preference_candidate.0)} {
                    let mut found_col = exhausted_column;
                    for &candidate in vote.prefs {
                        let found = candidate_to_who_index[candidate.0];
                        if found!=exhausted_column {
                            found_col=found;
                            break;
                        }
                    }
                    table[first_preference][found_col]+=vote.n;
                }
            }
        }
        IntentTable{table}
    }
}