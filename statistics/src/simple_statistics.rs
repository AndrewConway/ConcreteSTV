// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use stv::ballot_metadata::DataSource;
use stv::election_data::ElectionData;
use serde::{Serialize,Deserialize};

// TODO Federal 2013 deal explicitly with tickets rather than just converting to BTLs.

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct SimpleStatistics {
    pub num_satl : usize,
    pub num_atl : usize,
    pub num_unique_atl : usize,
    pub num_btl : usize,
    pub num_unique_btl : usize,
    pub num_candidates : usize,
    pub num_formal : usize,
    pub num_informal : usize,
    pub uses_group_voting_tickets : bool,
    pub download_locations : Vec<DataSource>,
    pub vote_types : Vec<ByTypeOfVotes>,
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct ByTypeOfVotes {
    pub name : String,
    pub num_atl : usize,
    pub num_btl : usize,
}

impl SimpleStatistics {
    pub fn new(data:&ElectionData) -> Self {
        let mut vote_types = vec![];
        for t in &data.atl_types {
            let num_atl = data.atl[t.first_index_inclusive..t.last_index_exclusive].iter().map(|v|v.n).sum();
            vote_types.push(ByTypeOfVotes{ name: t.vote_type.to_string(), num_atl, num_btl:0 });
        }
        for t in &data.btl_types {
            let num_btl = data.btl[t.first_index_inclusive..t.last_index_exclusive].iter().map(|v|v.n).sum();
            if let Some (existing) = vote_types.iter_mut().find(|c|c.name.as_str()==t.vote_type.as_str()) {
                existing.num_btl=num_btl
            } else {
                vote_types.push(ByTypeOfVotes{ name: t.vote_type.to_string(), num_atl:0, num_btl });
            }
        }
        SimpleStatistics {
            num_satl: data.num_satl(),
            num_atl: data.num_atl(),
            num_unique_atl: data.atl.len(),
            num_btl: data.num_btl(),
            num_unique_btl: data.btl.len(),
            num_candidates: data.metadata.candidates.len(),
            num_formal: data.num_atl()+data.num_btl(),
            num_informal: data.informal,
            uses_group_voting_tickets: data.metadata.parties.iter().any(|p|!p.tickets.is_empty()),
            download_locations: data.metadata.source.clone(),
            vote_types,
        }
    }
}
