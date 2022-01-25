// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use stv::election_data::ElectionData;
use serde::{Serialize,Deserialize};
use stv::ballot_metadata::CandidateIndex;
use stv::ballot_pile::BallotPaperCount;

#[derive(Debug,Serialize,Deserialize,Clone,Copy)]
/// Contain the number of votes that mention the party/candidate first, or at all, by ATL or by BTL.
pub struct NumVotesReceived {
    pub first_atl : BallotPaperCount,
    pub first_btl : BallotPaperCount,
    pub mention_atl : BallotPaperCount,
    pub mention_btl : BallotPaperCount
}

impl NumVotesReceived {
    pub fn zero() -> Self {
        NumVotesReceived{
            first_atl: BallotPaperCount(0),
            first_btl: BallotPaperCount(0),
            mention_atl: BallotPaperCount(0),
            mention_btl: BallotPaperCount(0),
        }
    }
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct WhoGotVotes {
    pub candidates : Vec<NumVotesReceived>,
    pub parties : Vec<NumVotesReceived>,
}

impl WhoGotVotes {
    /// compute for each party in the given data
    pub fn compute(data:&ElectionData) -> WhoGotVotes {
        let mut parties = vec![NumVotesReceived::zero();data.metadata.parties.len()];
        let mut candidates = vec![NumVotesReceived::zero();data.metadata.candidates.len()];
        let arena = typed_arena::Arena::<CandidateIndex>::new();
        let votes = data.resolve_atl(&arena);
        let mut seen_someone_else_in_party = vec![usize::MAX;data.metadata.parties.len()];
        for (mention_index,vote) in votes.iter().enumerate() {
            let atl = vote.is_atl();
            if vote.prefs.len()>0 {
                let first_pref = vote.prefs[0];
                if atl { candidates[first_pref.0].first_atl+=vote.n; } else { candidates[first_pref.0].first_btl+=vote.n;}
                if let Some(party) = data.metadata.candidate(first_pref).party {
                    if atl { parties[party.0].first_atl+=vote.n; } else { parties[party.0].first_btl+=vote.n; }
                }
            }
            for &candidate in vote.prefs {
                if atl { candidates[candidate.0].mention_atl+=vote.n; } else { candidates[candidate.0].mention_btl+=vote.n;}
                if let Some(party) = data.metadata.candidate(candidate).party {
                    if mention_index!=seen_someone_else_in_party[party.0] {
                        seen_someone_else_in_party[party.0]=mention_index;
                        if atl { parties[party.0].mention_atl+=vote.n; } else { parties[party.0].mention_btl+=vote.n;}
                    }
                }
            }
        }
        WhoGotVotes{ candidates, parties }
    }
}