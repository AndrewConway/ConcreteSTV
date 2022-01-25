// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use std::collections::HashSet;
use crate::ballot_metadata::{ElectionMetadata, CandidateIndex};
use crate::ballot_paper::{ATL, BTL, VoteSource};
use crate::ballot_pile::{PartiallyDistributedVote};
use std::fs::File;
use serde::{Deserialize,Serialize};
use crate::distribution_of_preferences_transcript::Transcript;
use crate::preference_distribution::{distribute_preferences, PreferenceDistributionRules};

/*
/// Complete list of raw ballot markings.
pub struct RawElectionData {
    pub meta : ElectionMetadata,
    pub ballots : Vec<RawBallotMarkings>,
}*/

/// Formal votes for the election.
#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct ElectionData {
    pub metadata : ElectionMetadata,
    pub atl : Vec<ATL>,
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub atl_types : Vec<VoteTypeSpecification>,
    pub btl : Vec<BTL>,
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub btl_types : Vec<VoteTypeSpecification>,
    /// number of informal votes
    pub informal : usize,
}

/// Sometimes votes can have different classes, e.g. in booth on polling day, postal, declaration, internet.
/// Rather than have a string associated with each ATL or BTL structure, there are instead optional
/// annotations on a range of indices of the existing ATL or BTL votes.
#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct VoteTypeSpecification {
    /// what votes in the given range represent. This should match the particular EC's designation.
    pub vote_type : String,
    pub first_index_inclusive : usize,
    pub last_index_exclusive : usize,
}

impl ElectionData {
    /// Number of formal above the line votes
    pub fn num_atl(&self) -> usize {
        self.atl.iter().map(|v|v.n).sum()
    }
    /// Number of formal above the line votes with only one preference listed
    pub fn num_satl(&self) -> usize {
        self.atl.iter().filter(|v|v.parties.len()==1).map(|v|v.n).sum()
    }
    /// Number of formal below the line votes
    pub fn num_btl(&self) -> usize {
        self.btl.iter().map(|v|v.n).sum()
    }
    /// Number of formal votes
    pub fn num_votes(&self) -> usize {
        self.num_atl()+self.num_btl()
    }
    /// Get a list of all votes with ATL votes converted to the corresponding BTL equivalent.
    /// Requires an arena to hold interpreted preference lists. This can be allocated by
    /// ```
    /// use stv::ballot_metadata::CandidateIndex;
    /// let arena = typed_arena::Arena::<CandidateIndex>::new();
    /// ```
    pub fn resolve_atl<'a>(&'a self,arena : &'a typed_arena::Arena<CandidateIndex>) -> Vec<PartiallyDistributedVote<'a>> {
        let mut votes : Vec<PartiallyDistributedVote<'a>> = vec![];
        for a in & self.atl {
            let v : Vec<CandidateIndex> = a.parties.iter().flat_map(|p|self.metadata.party(*p).candidates.iter().map(|c|*c)).collect();
            let slice = arena.alloc_extend(v);
            votes.push(PartiallyDistributedVote::new(a.n,slice,VoteSource::Atl(a)));
        }
        for b in &self.btl {
            votes.push(PartiallyDistributedVote::new(b.n,b.candidates.as_slice(),VoteSource::Btl(b)));
        }
        votes
    }

    pub fn print_summary(&self) {
        println!("Summary for {}",self.metadata.name.human_readable_name());
        println!("{} formal votes, {} informal",self.num_votes(),self.informal);
        println!("{} ATL formal votes, {} unique preference lists",self.num_atl(),self.atl.len());
        println!("{} BTL formal votes, {} unique preference lists",self.num_btl(),self.btl.len());
        for vote_type in self.all_vote_types() {
            let atl = self.atl_types.iter().find(|t|t.vote_type==vote_type).map(|t|self.atl[t.first_index_inclusive..t.last_index_exclusive].iter().map(|v|v.n).sum()).unwrap_or(0);
            let btl = self.btl_types.iter().find(|t|t.vote_type==vote_type).map(|t|self.btl[t.first_index_inclusive..t.last_index_exclusive].iter().map(|v|v.n).sum()).unwrap_or(0);
            println!("  Vote type {} : {} ATL, {} BTL, {} total",vote_type,atl,btl,atl+btl);
        }
    }

    pub fn all_vote_types(&self) -> Vec<&str> {
        self.atl_types.iter().chain(self.btl_types.iter()).map(|s|s.vote_type.as_str()).collect::<HashSet<&str>>().into_iter().collect()
    }
    pub fn save_to_cache(&self) -> std::io::Result<()> {
        let name = self.metadata.name.cache_file_name();
        std::fs::create_dir_all(name.parent().unwrap())?;
        let file = File::create(name)?;
        serde_json::to_writer(file,&self)?;
        Ok(())
    }

    fn is_verifiable(types:&[VoteTypeSpecification],index:usize,ballot_types_considered_unverifiable:&HashSet<String>) -> bool {
        types.iter().find(|t|t.first_index_inclusive<=index && index<t.last_index_exclusive&&!ballot_types_considered_unverifiable.contains(&t.vote_type)).is_some()
    }
    pub fn is_atl_verifiable(&self,atl_index:usize,ballot_types_considered_unverifiable:&HashSet<String>) -> bool { Self::is_verifiable(&self.atl_types,atl_index,ballot_types_considered_unverifiable) }
    pub fn is_btl_verifiable(&self,btl_index:usize,ballot_types_considered_unverifiable:&HashSet<String>) -> bool { Self::is_verifiable(&self.btl_types,btl_index,ballot_types_considered_unverifiable) }

    /// run the distribution of preferences with the values given in the metadata for the number of vacancies, who is ineligible, and EC resolutions. Convenience method.
    pub fn distribute_preferences<Rules:PreferenceDistributionRules>(&self) -> Transcript<Rules::Tally> {
        distribute_preferences::<Rules>(self,self.metadata.vacancies.unwrap(),&self.metadata.excluded.iter().cloned().collect::<HashSet<_>>(),&self.metadata.tie_resolutions,false)
    }

}
