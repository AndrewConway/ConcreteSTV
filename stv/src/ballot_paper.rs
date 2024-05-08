// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Information about a raw vote. That is, something written on a ballot paper.
//! This may or may not be formal.

use crate::ballot_metadata::{CandidateIndex, ElectionMetadata, PartyIndex};
use serde::{Deserialize,Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use anyhow::anyhow;
use crate::election_data::{ElectionData, VoteTypeSpecification};

/// A marking on a particular square in a ballot. This may or may not be a number.
#[derive(Copy,Clone,Debug,Eq, PartialEq)]
pub enum RawBallotMarking {
    Number(u16),
    /// A marking that is legislatively considered the same as a 1, such as a tick in some jurisdictions.
    OneEquivalent,
    Blank,
    Other,
}

impl RawBallotMarking {
    /// interpret a marking as a number between 1 and num_candidates inclusive, if possible.
    pub fn as_preference(self,num_candidates:usize) -> Option<usize> {
        match self {
            RawBallotMarking::Number(n) if n>0 && n as usize <=num_candidates => Some(n as usize),
            RawBallotMarking::Number(_) => None,
            RawBallotMarking::OneEquivalent => Some(1),
            RawBallotMarking::Blank => None,
            RawBallotMarking::Other => None,
        }
    }
}

impl Display for RawBallotMarking {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            RawBallotMarking::Number(n) => { write!(f,"{}",n) }
            RawBallotMarking::OneEquivalent => { write!(f,"X") }
            RawBallotMarking::Blank => { Ok(()) }
            RawBallotMarking::Other => { write!(f,"?") }
        }
    }
}

pub fn parse_marking(marking:&str) -> RawBallotMarking {
    if marking.is_empty() { RawBallotMarking::Blank }
    else if marking=="X" || marking=="*" || marking=="/" { RawBallotMarking::OneEquivalent }
    else if let Ok(num) = marking.parse::<u16>() { RawBallotMarking::Number(num) }
    else {
        // println!("Found other marking : {}",marking);
        RawBallotMarking::Other
    }
}

/// The collection of numbers written by the voter on the ballot.
pub struct RawBallotMarkings<'a> {
    /// atl[i] is the marking for party atl_parties[i].
    pub atl : &'a [RawBallotMarking],
    /// btl[i] is the marking for CandidateIndex(i).
    pub btl : &'a [RawBallotMarking],
    pub atl_parties : &'a[PartyIndex],
}

/// A formal vote, may be above the line or below the line.
#[derive(Clone,Debug)]
pub enum FormalVote {
    Btl(BTL),
    Atl(ATL)
}

/// Where a vote came from.
#[derive(Clone, Copy,Debug)]
pub enum VoteSource<'a> {
    Btl(&'a BTL),
    Atl(&'a ATL)
}

/// Below the line vote.
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct BTL {
    /// Candidate ids, in preference order
    pub candidates : Vec<CandidateIndex>,
    /// Number of people who voted in this way.
    pub n : usize,
}

/// Above the line vote, usually for multiple parties.
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ATL {
    /// Party ids, in preference order
    pub parties : Vec<PartyIndex>,
    /// Number of people who voted in this way.
    pub n : usize,
    /// if tickets are being used, which one these votes apply to. If a party has 3 tickets, their ATL vote will be divided into 3 ATL structures.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub ticket_index : Option<usize>
}

impl ATL {
    pub fn first_party(&self) -> PartyIndex { self.parties[0] }
    pub fn resolve_to_candidates(&self,metadata:&ElectionMetadata) -> Vec<CandidateIndex> {
        if let Some(ticket_index) = self.ticket_index {
            metadata.party(self.first_party()).tickets[ticket_index].clone()
        } else {
            self.parties.iter().flat_map(|p|metadata.party(*p).candidates.iter().cloned()).collect()
        }
    }
}

impl<'a> RawBallotMarkings<'a> {

    /// Interpret an array of markings, atls first then btls, possibly truncated if blank.
    pub fn new(parties_that_can_get_atls:&'a Vec<PartyIndex>,markings:&'a Vec<RawBallotMarking>) -> Self {
        let cutoff = parties_that_can_get_atls.len().min(markings.len());
        RawBallotMarkings{
            atl: &markings[..cutoff],
            btl: &markings[cutoff..],
            atl_parties: parties_that_can_get_atls.as_slice()
        }
    }

    /// Given a raw vote, interpret it as a list of preferences.
    /// Using AEC style rules,
    pub fn interpret_vote(&self,min_atl_prefs_needed:usize,min_btl_prefs_needed:usize) -> Option<FormalVote> {
        if let Some(btl) = self.interpret_vote_as_btl(min_btl_prefs_needed) {
            Some(FormalVote::Btl(btl))
        } else if let Some(atl)  = self.interpret_vote_as_atl(min_atl_prefs_needed) {
            Some(FormalVote::Atl(atl))
        } else {None}
    }

    /// Interpret a list of markings as preferences.
    /// * Ignore all repeated numbers. E.g. 1 2 2 ignore the 2s.
    /// * Ignore all numbers after a gap. E.g. 1 3 4 ignore the 3 and 4 (unless allow_gaps) is true
    /// * Treat a cross as a 1 iff consider_cross_as_one true
    /// Otherwise take the longest list of preferences starting at 1.
    /// The return type is given by a (provided) function
    fn look_for_continuous_streams<T:Copy,F : Fn(usize)->T>(markings:&[RawBallotMarking],result_generator:F,consider_cross_as_one:bool,allow_gaps:bool) -> Vec<T> {
        let mut times_seen = vec![0 as usize;markings.len()];
        let mut prefs = vec![result_generator(0);markings.len()];
        for i in 0..markings.len() {
            match markings[i] {
                RawBallotMarking::Number(n) if n>0 && n as usize<= markings.len() => {
                    prefs[n as usize-1]=result_generator(i);
                    times_seen[n as usize-1]+=1;
                }
                RawBallotMarking::OneEquivalent if consider_cross_as_one => {
                    prefs[1-1]=result_generator(i);
                    times_seen[1-1]+=1;
                }
                _ => {}
            }
        }
        if allow_gaps {
            let mut num_good = 0;
            let mut num_up_to = 0;
            while num_up_to<times_seen.len() && times_seen[num_up_to]<2 {
                if times_seen[num_up_to]==0 { prefs.remove(num_good); } else { num_good+=1; }
                num_up_to+=1;
            }
            prefs.truncate(num_good);
        } else {
            let mut num_good = 0;
            while num_good<times_seen.len() && times_seen[num_good]==1 { num_good+=1; }
            prefs.truncate(num_good);
        }
        prefs
    }

    fn interpret_vote_as_atl(&'a self,min_atl_prefs_needed:usize) -> Option<ATL> {
        let prefs = RawBallotMarkings::look_for_continuous_streams(self.atl,|i|self.atl_parties[i],true,false);
        if prefs.len()>=min_atl_prefs_needed { Some(ATL{ parties: prefs, n: 1, ticket_index: None })} else { None }
    }
    pub fn interpret_vote_as_btl(&'a self, min_btl_prefs_needed:usize) -> Option<BTL> {
        let prefs = RawBallotMarkings::look_for_continuous_streams(self.btl,|i|CandidateIndex(i),true,false);
        if prefs.len()>=min_btl_prefs_needed { Some(BTL{ candidates: prefs, n: 1 })} else { None }
    }
    /// intepret 1 2 4 5 as a valid sequence.
    pub fn interpret_vote_as_btl_allowing_gaps(&'a self, min_btl_prefs_needed:usize) -> Option<BTL> {
        let prefs = RawBallotMarkings::look_for_continuous_streams(self.btl,|i|CandidateIndex(i),true,true);
        if prefs.len()>=min_btl_prefs_needed { Some(BTL{ candidates: prefs, n: 1 })} else { None }
    }
}

/// A utility for building up a BTL list and simplifying duplicate votes.
#[derive(Default)]
pub struct UniqueBTLBuilder {
    btls : HashMap<Vec<CandidateIndex>,usize>,
}

impl UniqueBTLBuilder {
    /// Add a vote with a given preference list
    pub fn add(&mut self,prefs:Vec<CandidateIndex>) {
        *self.btls.entry(prefs).or_insert(0)+=1;
    }
    pub fn add_vote(&mut self,vote:BTL) {
        *self.btls.entry(vote.candidates).or_insert(0)+=vote.n;
    }
    /// Convert to a list of BTL votes.
    pub fn to_btls(self) -> Vec<BTL> {
        self.btls.into_iter().map(|(candidates,n)|BTL{ candidates , n }).collect()
    }
}


/// A utility for building up an ATL list and simplifying duplicate votes.
#[derive(Default)]
pub struct UniqueATLBuilder {
    atls : HashMap<Vec<PartyIndex>,usize>,
}

impl UniqueATLBuilder {
    /// Add a vote with a given preference list
    pub fn add(&mut self,prefs:Vec<PartyIndex>) {
        *self.atls.entry(prefs).or_insert(0)+=1;
    }
    pub fn add_vote(&mut self,vote:ATL) {
        *self.atls.entry(vote.parties).or_insert(0)+=vote.n;
    }
    /// Convert to a list of BTL votes.
    pub fn to_atls(self) -> Vec<ATL> {
        self.atls.into_iter().map(|(parties,n)|ATL{ parties , n, ticket_index: None }).collect()
    }
}

#[derive(Default)]
/// A utility for building up a list of unique votes given a list of votes.
pub struct UniqueVoteBuilder {
    pub atl : UniqueATLBuilder,
    pub btl : UniqueBTLBuilder,
    pub informal : usize,
}

impl UniqueVoteBuilder {
    pub fn add_vote(&mut self,vote:Option<FormalVote>) {
        match vote {
            None => { self.informal+=1 }
            Some(FormalVote::Btl(btl)) => { self.btl.add_vote(btl) }
            Some(FormalVote::Atl(atl)) => { self.atl.add_vote(atl) }
        }
    }
}

#[derive(Default)]
/// A utility for building up a list of unique votes of various types given a list of votes.
pub struct UniqueVoteBuilderMultipleTypes {
    pub by_type : HashMap<String,UniqueVoteBuilder>,
    pub no_type : UniqueVoteBuilder,
}

impl UniqueVoteBuilderMultipleTypes {
    pub fn add_vote(&mut self,vote:Option<FormalVote>,vote_type:Option<&str>) {
        let destination = match vote_type {
            Some(t) => self.by_type.entry(t.to_string()).or_insert_with(||UniqueVoteBuilder::default()),
            None => &mut self.no_type,
        };
        destination.add_vote(vote);
    }
    pub fn into_election_data(self, metadata:ElectionMetadata) -> ElectionData {
        let mut atl = self.no_type.atl.to_atls();
        let mut btl = self.no_type.btl.to_btls();
        let mut atl_types = vec![];
        let mut btl_types = vec![];
        let mut informal = self.no_type.informal;
        for (vote_type,builder) in self.by_type {
            let mut new_atl = builder.atl.to_atls();
            let mut new_btl = builder.btl.to_btls();
            informal+=builder.informal;
            if !new_atl.is_empty() {
                atl_types.push(VoteTypeSpecification{
                    vote_type: vote_type.clone(),
                    first_index_inclusive: atl.len(),
                    last_index_exclusive: atl.len()+new_atl.len(),
                });
                atl.append(&mut new_atl)
            }
            if !new_btl.is_empty() {
                btl_types.push(VoteTypeSpecification{
                    vote_type: vote_type.clone(),
                    first_index_inclusive: btl.len(),
                    last_index_exclusive: btl.len()+new_btl.len(),
                });
                btl.append(&mut new_btl)
            }
        }
        ElectionData{ metadata, atl, atl_types, atl_transfer_values: vec![], btl, btl_types, btl_transfer_values: vec![], informal }
    }
}

/// A utility for dealing with A/BTL votes coming in in a random order.
pub struct PreferencesComingOutOfOrder<T:Copy> {
    /// received[i] = Some(x) iff preference i+1 was for entity x.
    received : Vec<Option<T>>,
}

impl <T:Copy> Default for PreferencesComingOutOfOrder<T> {
    fn default() -> Self {
        PreferencesComingOutOfOrder{ received: vec![] }
    }
}

impl <T:Copy> PreferencesComingOutOfOrder<T> {
    // add a marking for `who_for` with preference `preference` starting from 1.
    pub fn add(&mut self,preference:usize,who_for:T) -> anyhow::Result<()> {
        if preference==0 { return Err(anyhow!("Can't have a preference of 0"))}
        if self.received.len()<preference { self.received.resize(preference,None) }
        if self.received[preference-1].is_some() { return Err(anyhow!("Already got preference {}",preference))}
        self.received[preference-1] = Some(who_for);
        Ok(())
    }

    /// Get the first contiguous list of votes.
    pub fn drain_pref_list(&mut self) -> Vec<T> {
        self.received.drain(..).take_while(Option::is_some).flatten().collect()
    }

    pub fn is_empty(&self) -> bool { self.received.is_empty() || self.received[0].is_none() }

    pub fn clear(&mut self) { self.received.clear(); }
}

/// A helper structure for getting votes coming of the form "paper X had a preference of Y for Z"
pub struct PreferencesComingOutOfOrderHelper {
    atls : UniqueATLBuilder,
    btls : UniqueBTLBuilder,
    atls_by_vote_type : HashMap<String,UniqueATLBuilder>,
    btls_by_vote_type : HashMap<String,UniqueBTLBuilder>,
    informal : usize,
    current_paper_id : Option<String>,
    current_atls : PreferencesComingOutOfOrder<PartyIndex>,
    current_btls : PreferencesComingOutOfOrder<CandidateIndex>,
    current_vote_type : Option<String>,
}

impl Default for PreferencesComingOutOfOrderHelper {
    fn default() -> Self {
        PreferencesComingOutOfOrderHelper{
            atls: Default::default(),
            btls: Default::default(),
            atls_by_vote_type: Default::default(),
            btls_by_vote_type: Default::default(),
            informal: 0,
            current_paper_id: None,
            current_atls: PreferencesComingOutOfOrder::default(),
            current_btls: PreferencesComingOutOfOrder::default(),
            current_vote_type: None
        }
    }
}

impl PreferencesComingOutOfOrderHelper {
    pub fn done_current_paper(&mut self) {
        let vote_type = self.current_vote_type.take();
        if !self.current_btls.is_empty() { // is BTL
            let btls = match vote_type {
                None => &mut self.btls,
                Some(vote_type) => self.btls_by_vote_type.entry(vote_type).or_insert_with(||Default::default()),
            };
            btls.add(self.current_btls.drain_pref_list());
        } else if !self.current_atls.is_empty() { // is ATL
            let atls = match vote_type {
                None => &mut self.atls,
                Some(vote_type) => self.atls_by_vote_type.entry(vote_type).or_insert_with(||Default::default()),
            };
            atls.add(self.current_atls.drain_pref_list());
        } else { self.informal+=1; }
        self.current_paper_id=None;
        self.current_atls.clear();
        self.current_btls.clear();
    }
    /// Set the current vote's type. Call after set_current_paper.
    pub fn set_vote_type(&mut self,vote_type:&str) {
        self.current_vote_type=Some(vote_type.to_string());
    }
    pub fn set_current_paper(&mut self,paper_id:&str) {
        if self.current_paper_id.is_some() && self.current_paper_id.as_ref().unwrap()!=paper_id {
            self.done_current_paper();
        }
        if self.current_paper_id.is_none() {
            self.current_paper_id = Some(paper_id.to_string());
        }
    }
    pub fn add_atl_pref(&mut self, preference:usize, party:PartyIndex) -> anyhow::Result<()> {
        self.current_atls.add(preference,party)
    }
    pub fn add_btl_pref(&mut self, preference:usize, candidate:CandidateIndex) -> anyhow::Result<()> {
        self.current_btls.add(preference,candidate)
    }

    pub fn done(mut self,metadata:ElectionMetadata) -> ElectionData {
        if self.current_paper_id.is_some() {
            self.done_current_paper();
        }
        let mut atl = self.atls.to_atls();
        let mut atl_types : Vec<VoteTypeSpecification> = vec![];
        for (vote_type,builder) in self.atls_by_vote_type.drain() {
            let extra_atls = builder.to_atls();
            atl_types.push(VoteTypeSpecification{
                vote_type,
                first_index_inclusive: atl.len(),
                last_index_exclusive: atl.len()+extra_atls.len(),
            });
            atl.extend(extra_atls.into_iter());
        }
        let mut btl = self.btls.to_btls();
        let mut btl_types : Vec<VoteTypeSpecification> = vec![];
        for (vote_type,builder) in self.btls_by_vote_type.drain() {
            let extra_btls = builder.to_btls();
            btl_types.push(VoteTypeSpecification{
                vote_type,
                first_index_inclusive: btl.len(),
                last_index_exclusive: btl.len()+extra_btls.len(),
            });
            btl.extend(extra_btls.into_iter());
        }
        ElectionData{
            metadata,
            atl,
            atl_types,
            atl_transfer_values: vec![],
            btl,
            btl_types,
            btl_transfer_values: vec![],
            informal: self.informal,
        }
    }
}