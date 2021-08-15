use crate::ballot_metadata::{ElectionMetadata, CandidateIndex};
use crate::ballot_paper::{ATL, BTL, VoteSource};
use crate::ballot_pile::{PartiallyDistributedVote};
use std::fs::File;
use serde::{Deserialize,Serialize};

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
    pub btl : Vec<BTL>,
    /// number of informal votes
    pub informal : usize,
}

impl ElectionData {
    /// Number of formal above the line votes
    pub fn num_atl(&self) -> usize {
        self.atl.iter().map(|v|v.n).sum()
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
    }

    pub fn save_to_cache(&self) -> std::io::Result<()> {
        let name = self.metadata.name.cache_file_name();
        std::fs::create_dir_all(name.parent().unwrap())?;
        let file = File::create(name)?;
        serde_json::to_writer(file,&self)?;
        Ok(())
    }
}
