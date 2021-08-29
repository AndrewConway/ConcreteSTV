// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Information about a raw vote. That is, something written on a ballot paper.
//! This may or may not be formal.

use crate::ballot_metadata::{CandidateIndex, PartyIndex};
use serde::{Deserialize,Serialize};

/// A marking on a particular square in a ballot. This may or may not be a number.
#[derive(Copy,Clone,Debug,Eq, PartialEq)]
pub enum RawBallotMarking {
    Number(u16),
    /// A marking that is legislatively considered the same as a 1, such as a tick in some jurisdictions.
    OneEquivalent,
    Blank,
    Other,
}

pub fn parse_marking(marking:&str) -> RawBallotMarking {
    if marking.is_empty() { RawBallotMarking::Blank }
    else if marking=="X" || marking=="*" || marking=="/" { RawBallotMarking::OneEquivalent }
    else if let Ok(num) = marking.parse::<u16>() { RawBallotMarking::Number(num) }
    else {
        println!("Found other marking : {}",marking);
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
#[derive(Clone,Debug,Serialize,Deserialize)]
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
    /// * Ignore all numbers after a gap. E.g. 1 3 4 ignore the 3 and 4
    /// * Treat a cross as a 1 iff consider_cross_as_one true
    /// Otherwise take the longest list of preferences starting at 1.
    /// The return type is given by a (provided) function
    fn look_for_continuous_streams<T:Copy,F : Fn(usize)->T>(markings:&[RawBallotMarking],result_generator:F,consider_cross_as_one:bool) -> Vec<T> {
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
        let mut num_good = 0;
        while num_good<times_seen.len() && times_seen[num_good]==1 { num_good+=1; }
        prefs.truncate(num_good);
        prefs
    }

    fn interpret_vote_as_atl(&'a self,min_atl_prefs_needed:usize) -> Option<ATL> {
        let prefs = RawBallotMarkings::look_for_continuous_streams(self.atl,|i|self.atl_parties[i],true);
        if prefs.len()>=min_atl_prefs_needed { Some(ATL{ parties: prefs, n: 1 })} else { None }
    }
    pub fn interpret_vote_as_btl(&'a self, min_btl_prefs_needed:usize) -> Option<BTL> {
        let prefs = RawBallotMarkings::look_for_continuous_streams(self.btl,|i|CandidateIndex(i),true);
        if prefs.len()>=min_btl_prefs_needed { Some(BTL{ candidates: prefs, n: 1 })} else { None }
    }
}
