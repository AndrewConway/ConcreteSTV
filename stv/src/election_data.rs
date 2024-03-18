// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use std::collections::{HashMap, HashSet};
use crate::ballot_metadata::{ElectionMetadata, CandidateIndex};
use crate::ballot_paper::{ATL, BTL, VoteSource};
use crate::ballot_pile::{PartiallyDistributedVote};
use std::fs::File;
use std::ops::{Mul, Range};
use num::{BigInt, ToPrimitive, Zero};
use serde::{Deserialize,Serialize};
use crate::distribution_of_preferences_transcript::Transcript;
use crate::preference_distribution::{BigRational, distribute_preferences, PreferenceDistributionRules};
use crate::random_util::Randomness;
use crate::transfer_value::TransferValue;

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
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub atl_transfer_values : Vec<VoteValueSpecification>,
    pub btl : Vec<BTL>,
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub btl_types : Vec<VoteTypeSpecification>,
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub btl_transfer_values : Vec<VoteValueSpecification>,
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

/// Sometimes votes can have different values, e.g. the ACT's casual vacancy rules
/// start off with the votes that made a particular candidate be elected, as if they
/// had transfer values equivalent to the ones that they counted for the candidate who
/// is being replaced.
///
/// Rather than have a value associated with each ATL or BTL structure, there are instead optional
/// annotations on a range of indices of the existing ATL or BTL votes.
#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct VoteValueSpecification {
    pub value : TransferValue,
    pub first_index_inclusive : usize,
    pub last_index_exclusive : usize,
}

impl VoteValueSpecification {
    pub fn range(&self) -> Range<usize> { self.first_index_inclusive..self.last_index_exclusive }
}

impl VoteTypeSpecification {
    /// Find the indices of votes that pass restrictions on the types used.
    ///
    /// If `vote_types` is None, then no restrictions. return just [0..largest_index].
    /// If `vote_types` is Some(), then only take votes that match something in vote_types. IF the empty string is in the list, then take votes that are not covered by specs.
    ///
    /// specs must be in order.
    /// #Example
    ///
    /// ```
    /// use stv::election_data::VoteTypeSpecification;
    /// let spec_a = VoteTypeSpecification{ vote_type : "A".to_string(), first_index_inclusive:5, last_index_exclusive:10 };
    /// let spec_b = VoteTypeSpecification{ vote_type : "B".to_string(), first_index_inclusive:10, last_index_exclusive:15 };
    /// let specs = vec![spec_a,spec_b];
    ///
    /// assert_eq!(VoteTypeSpecification::restrict(None,&specs,20),vec![0..20]);
    /// assert_eq!(VoteTypeSpecification::restrict(Some(&["A".to_string(),"".to_string()]),&specs,20)
    ///                ,vec![0..5,5..10,15..20]);
    /// assert_eq!(VoteTypeSpecification::restrict(Some(&["A".to_string()]),&specs,20)
    ///                ,vec![5..10]);
    /// ```
    pub fn restrict(vote_types : Option<&[String]>,specs:&[VoteTypeSpecification],largest_index:usize) -> Vec<Range<usize>> {
        match vote_types {
            None => vec![ 0..largest_index ],
            Some(ok_types) => {
                let mut specs = specs.iter().collect::<Vec<_>>(); // make sure in order.
                specs.sort_by_key(|s|s.first_index_inclusive);
                let mut res = vec![];
                let contains_blank = ok_types.iter().any(|e|e.is_empty());
                let mut upto = 0;
                for spec in specs {
                    if contains_blank && upto<spec.first_index_inclusive { res.push(upto..spec.first_index_inclusive); }
                    upto=spec.last_index_exclusive;
                    if ok_types.contains(&spec.vote_type) {
                        res.push(spec.first_index_inclusive..spec.last_index_exclusive);
                    }
                }
                if contains_blank && upto<largest_index { res.push(upto..largest_index); }
                res
            }
        }
    }
    pub fn range(&self) -> Range<usize> { self.first_index_inclusive..self.last_index_exclusive }

}

impl ElectionData {
    /// Number of formal above the line votes
    pub fn num_atl(&self) -> usize {
        self.atl.iter().map(|v|v.n).sum()
    }
    pub fn num_atl_range(&self,range:Range<usize>) -> usize {
        self.atl[range].iter().map(|v|v.n).sum()
    }
    /// Number of formal above the line votes with only one preference listed
    pub fn num_satl(&self) -> usize {
        self.atl.iter().filter(|v|v.parties.len()==1).map(|v|v.n).sum()
    }
    /// Number of formal below the line votes
    pub fn num_btl(&self) -> usize {
        self.btl.iter().map(|v|v.n).sum()
    }
    pub fn num_btl_range(&self,range:Range<usize>) -> usize {
        self.btl[range].iter().map(|v|v.n).sum()
    }
    /// Number of formal votes
    pub fn num_votes(&self) -> usize {
        self.num_atl()+self.num_btl()
    }
    /// Get a list of all votes with ATL votes converted to the corresponding BTL equivalent.
    /// Requires an arena to hold interpreted preference lists. This can be allocated by
    /// If vote_types is None, use all votes.
    /// otherwise only use vote types specified in it.
    /// ```
    /// use stv::ballot_metadata::CandidateIndex;
    /// let arena = typed_arena::Arena::<CandidateIndex>::new();
    /// ```
    pub fn resolve_atl<'a>(&'a self,arena : &'a typed_arena::Arena<CandidateIndex>,vote_types : Option<&[String]>) -> Vec<PartiallyDistributedVote<'a>> {
        let mut votes : Vec<PartiallyDistributedVote<'a>> = vec![];
        for range in VoteTypeSpecification::restrict(vote_types,&self.atl_types,self.atl.len()) {
            for a in &self.atl[range] {
                let v : Vec<CandidateIndex> = a.resolve_to_candidates(&self.metadata);
                let slice = arena.alloc_extend(v);
                votes.push(PartiallyDistributedVote::new(a.n,slice,VoteSource::Atl(a)));
            }
        }
        for range in VoteTypeSpecification::restrict(vote_types,&self.btl_types,self.btl.len()) {
            for b in &self.btl[range] {
                votes.push(PartiallyDistributedVote::new(b.n,b.candidates.as_slice(),VoteSource::Btl(b)));
            }
        }
        votes
    }
    /// Get a list of all votes with ATL votes converted to the corresponding BTL equivalent,
    /// taking into account the weights/transfer values stored in election data.
    ///
    /// The resulting transfer values are all unique and stored in descending order.
    ///
    /// Requires an arena to hold interpreted preference lists. This can be allocated by
    /// If vote_types is None, use all votes.
    /// otherwise only use vote types specified in it.
    /// ```
    /// use stv::ballot_metadata::CandidateIndex;
    /// let arena = typed_arena::Arena::<CandidateIndex>::new();
    /// ```
    pub fn resolve_atl_including_weights<'a>(&'a self,arena : &'a typed_arena::Arena<CandidateIndex>,vote_types : Option<&[String]>) -> Vec<(TransferValue,Vec<PartiallyDistributedVote<'a>>)> {
        if self.btl_transfer_values.is_empty() && self.atl_transfer_values.is_empty() { // most common scenario
            return vec![(TransferValue::one(),self.resolve_atl(arena,vote_types))];
        }
        let mut votes_by_tv : HashMap<TransferValue,Vec<PartiallyDistributedVote<'a>>> = HashMap::new();
        fn intersection(r1:Range<usize>,r2:Range<usize>) -> Range<usize> {
            let start = r1.start.max(r2.start);
            let end = r1.end.min(r2.end);
            start..end
        }
        let mut add_votes_atl = |tv:TransferValue,range_with_tv:Range<usize>| {
            let votes = votes_by_tv.entry(tv).or_default();
            for range in VoteTypeSpecification::restrict(vote_types,&self.atl_types,self.atl.len()) {
                for a in &self.atl[intersection(range,range_with_tv.clone())] {
                    let v : Vec<CandidateIndex> = a.resolve_to_candidates(&self.metadata);
                    let slice = arena.alloc_extend(v);
                    votes.push(PartiallyDistributedVote::new(a.n,slice,VoteSource::Atl(a)));
                }
            }
        };
        if self.atl_transfer_values.is_empty() { add_votes_atl(TransferValue::one(),0..self.atl.len())}
        else {
            for v in &self.atl_transfer_values {
                add_votes_atl(v.value.clone(),v.range());
            }
        }
        let mut add_votes_btl = |tv:TransferValue,range_with_tv:Range<usize>| {
            let votes = votes_by_tv.entry(tv).or_default();
            for range in VoteTypeSpecification::restrict(vote_types,&self.btl_types,self.btl.len()) {
                for b in &self.btl[intersection(range,range_with_tv.clone())] {
                    votes.push(PartiallyDistributedVote::new(b.n,b.candidates.as_slice(),VoteSource::Btl(b)));
                }
            }
        };
        if self.btl_transfer_values.is_empty() { add_votes_btl(TransferValue::one(),0..self.btl.len())}
        else {
            for v in &self.btl_transfer_values {
                add_votes_btl(v.value.clone(),v.range());
            }
        }
        let mut res : Vec<(TransferValue,Vec<PartiallyDistributedVote<'a>>)> = votes_by_tv.into_iter().collect();
        res.sort_by(|(tv1,_),(tv2,_)|tv2.cmp(&tv1));
        res
    }

    pub fn print_summary(&self) {
        println!("Summary for {}",self.metadata.name.human_readable_name());
        println!("{} formal votes, {} informal",self.num_votes(),self.informal);
        println!("{} ATL formal votes, {} unique preference lists",self.num_atl(),self.atl.len());
        if !self.atl_transfer_values.is_empty() {
            let mut sum : BigRational = BigRational::zero();
            for atv in &self.atl_transfer_values {
                println!("TV {}≈{} {} ATL formal votes, {} unique preference lists",atv.value,atv.value.0.to_f64().unwrap(),self.num_atl_range(atv.range()),atv.range().len());
                sum+=BigRational::from_integer(BigInt::from(self.num_atl_range(atv.range()))).mul(&atv.value.0);
            }
            println!("Total value of ATL votes {}≈{}",sum,sum.to_f64().unwrap());
        }
        println!("{} BTL formal votes, {} unique preference lists",self.num_btl(),self.btl.len());
        if !self.btl_transfer_values.is_empty() {
            let mut sum : BigRational = BigRational::zero();
            for btv in &self.btl_transfer_values {
                println!("TV {}≈{} {} BTL formal votes, {} unique preference lists",btv.value,btv.value.0.to_f64().unwrap(),self.num_btl_range(btv.range()),btv.range().len());
                sum+=BigRational::from_integer(BigInt::from(self.num_btl_range(btv.range()))).mul(&btv.value.0);
            }
            println!("Total value of BTL votes {}≈{}",sum,sum.to_f64().unwrap());
        }
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
    pub fn distribute_preferences<Rules:PreferenceDistributionRules>(&self,randomness:&mut Randomness) -> Transcript<Rules::Tally> {
        distribute_preferences::<Rules>(self,self.metadata.vacancies.unwrap(),&self.metadata.excluded.iter().cloned().collect::<HashSet<_>>(),&self.metadata.tie_resolutions,None,false,randomness)
    }

}
