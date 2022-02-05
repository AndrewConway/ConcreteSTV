// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Look back over a transcript, and answer the question of which candidate's pile a vote is sitting on at a given count, and with what transfer value.


use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;
use stv::ballot_metadata::{CandidateIndex, ElectionMetadata, NumberOfCandidates};
use stv::distribution_of_preferences_transcript::{CountIndex, ReasonForCount, SingleCount};
use stv::election_data::ElectionData;
use stv::transfer_value::TransferValue;
use crate::choose_votes::{ChooseVotes, ChooseVotesOptions};
use serde::Serialize;
use serde::Deserialize;

/// A tool to help rerun an election given a transcript.
/// It lets you tell which candidate a particular person's votes were with, and with what transfer value.
/// You go continuously through the transcript, adding counts, and the retroscope will update the vote's data structures.
pub struct Retroscope {
    /// the count we have processed. The last count sent to "apply()". Fields below are as at the end of this count
    pub count : CountIndex,
    /// information on the votes in candidate order. This says in whose pile the votes are at the end of `count`
    pub votes : RetroscopeVotes,
    /// continuing candidates so far. Candidates elected (e.g. quota) at the end of the current count will NOT be present in this set.
    pub continuing : HashSet<CandidateIndex>,
    /// candidates elected so far. Includes candidates elected at the end of the current count.
    pub elected : Vec<CandidateIndex>,
    /// the transfer values for a given count.
    transfer_values : Vec<TransferValue>,

    /// piles of votes for a given candidate, in order of CandidateIndex.
    pub piles_by_candidate: Vec<RetroscopeVotePileForCandidate>,
}

#[derive(Default)]
pub struct RetroscopeVotes {
    pub atl : Vec<RetroscopeVoteStatus>,
    pub btl : Vec<RetroscopeVoteStatus>,
}

impl RetroscopeVotes {
    fn vote(&mut self, vote_index: RetroscopeVoteIndex) -> &mut RetroscopeVoteStatus {
        if vote_index.0 < self.atl.len() { &mut self.atl[vote_index.0] }
        else { &mut self.btl[vote_index.0-self.atl.len()] }
    }
}
/// Information corresponding to a particular ATL or BTL structure.
pub struct RetroscopeVoteStatus {
    /// Whether this vote is currently in a pile or not.
    pub pile_status : PileStatus,
    /// what count the vote arrived for that candidate. This can be used to get the transfer value.
    pub count_arrived : CountIndex,
    /// what index in prefs the distribution is up to.
    pub(crate) upto : usize,
    /// Preferred candidates, with index 0 being the most favoured candidate.
    pub(crate) prefs : Vec<CandidateIndex>,
}

/// Whether a vote is in a pile or not
#[derive(Copy, Clone,Eq,PartialEq,Debug)]
pub enum PileStatus {
    /// Vote is in a valid continuing candidates' pile.
    InPile,
    /// Vote has been set aside. This mainly happens in the presence of a last parcel surplus distribution mechnanism.
    SetAside,
    /// Vote has been exhausted.
    Exhausted,
}

/// if [0..atl.len()) then an index into atl, otherwise subtract atl.len() and an index into btl.
#[derive(Copy,Clone,Eq,PartialEq,Serialize,Deserialize)]
pub struct RetroscopeVoteIndex(pub usize);

// type alias really, don't want long display
impl fmt::Display for RetroscopeVoteIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}
// type alias really, don't want long display
impl fmt::Debug for RetroscopeVoteIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "#{}", self.0) }
}

#[derive(Default,Debug)]
pub struct RetroscopeVotePileForCandidate {
    pub by_count : HashMap<CountIndex,Vec<RetroscopeVoteIndex>>,
}
struct CandidateDistributor {
    going_to : Vec<Vec<RetroscopeVoteIndex>>
}

impl CandidateDistributor {
    /// make a new distributor with the appropriate number of candidates.
    fn new(num_candidates:NumberOfCandidates) -> Self {
        let mut going_to = vec![];
        for _ in 0..num_candidates.0 {
            going_to.push(vec![]);
        }
        CandidateDistributor{going_to}
    }
}
impl Retroscope {

    /// make a new Retroscope for the provided data
    pub fn new(data:&ElectionData,ineligible:&[CandidateIndex]) -> Self {
        let mut atl = vec![];
        for v in &data.atl {
            atl.push(RetroscopeVoteStatus{
                pile_status: PileStatus::InPile,
                count_arrived: CountIndex(0),
                upto: 0,
                prefs: v.resolve_to_candidates(&data.metadata),
            });
        }
        let mut btl = vec![];
        for v in &data.btl {
            btl.push(RetroscopeVoteStatus{
                pile_status: PileStatus::InPile,
                count_arrived: CountIndex(0),
                upto: 0,
                prefs: v.candidates.clone(),
            });
        }
        let mut piles_by_candidate = vec![];
        let mut continuing = HashSet::default();
        for c in 0..data.metadata.candidates.len() {
            piles_by_candidate.push(RetroscopeVotePileForCandidate::default() );
            if !ineligible.contains(&CandidateIndex(c)) {
                continuing.insert(CandidateIndex(c));
            }
        }
        Retroscope{
            votes: RetroscopeVotes{ atl, btl },
            continuing,
            elected: vec![],
            count: CountIndex(usize::MAX),
            transfer_values: vec![],
            piles_by_candidate,
        }
    }
    /// Apply the given count to the Retroscope. Update all the internal fields.
    pub fn apply<Tally:PartialEq+Clone+Display+FromStr>(&mut self,count:CountIndex,transcript:&SingleCount<Tally>) {
        if count.0>0 && self.count.0+1!=count.0 { panic!("Counts must be processed in order without skipping any.")}
        self.count=count;
        for c in &transcript.not_continuing {
            self.continuing.remove(c);
        }
        self.transfer_values.push(transcript.created_transfer_value.as_ref().map(|c|c.transfer_value.clone()).or(transcript.portion.transfer_value.clone()).unwrap_or(TransferValue::one()));
        match &transcript.reason {
            ReasonForCount::FirstPreferenceCount => { self.first_preferences(count); }
            ReasonForCount::ExcessDistribution(c) => { self.update(count,transcript.reason_completed,&[*c],&transcript.portion.papers_came_from_counts); }
            ReasonForCount::Elimination(c) => { self.update(count,transcript.reason_completed,c,&transcript.portion.papers_came_from_counts); }
        }
        // elections occur after distribution.
        for c in &transcript.elected {
            self.continuing.remove(&c.who);
            self.elected.push(c.who);
        }
    }

    /// Get the transfer value that votes arrived at in a given count.
    pub fn transfer_value(&self,count:CountIndex) -> &TransferValue {
        &self.transfer_values[count.0]
    }

    /// Apply the count to all VoteStatus people.
    /// Take all votes sitting on candidate from, and move them to the appropriate place.
    fn update(&mut self,count:CountIndex,reason_finished:bool,froms:&[CandidateIndex],from_counts:&[CountIndex]) {
        self.count=count;
        let mut distributor = CandidateDistributor::new(self.num_candidates());
        for &from in froms {
            for from_count in from_counts {
                if let Some(pile) = self.piles_by_candidate[from.0].by_count.remove(from_count) {
                    for vote_index in pile {
                        self.votes.vote(vote_index).next(count,&self.continuing,&mut distributor,vote_index)
                    }
                }
            }
        }
        self.add(distributor,count);
        if reason_finished { // set aside any remaining votes for said candidate.
            for from in froms {
                for (_,votes) in self.piles_by_candidate[from.0].by_count.drain() {
                    for vote in votes {
                        self.votes.vote(vote).pile_status=PileStatus::SetAside;
                    }
                }
            }
        }
    }

    fn distribute(&mut self, vote_index: RetroscopeVoteIndex, count:CountIndex, distributor:&mut CandidateDistributor) {
        self.votes.vote(vote_index).next(count,&self.continuing,distributor,vote_index)
    }

    fn first_preferences(&mut self,count:CountIndex) {
        let mut distributor = CandidateDistributor::new(self.num_candidates());
        for vote_index in 0..(self.votes.atl.len()+self.votes.btl.len()) {
            self.distribute(RetroscopeVoteIndex(vote_index), count, &mut distributor);
        }
        self.add(distributor,count);
    }

    fn add(&mut self,mut distributor:CandidateDistributor,count:CountIndex) {
        for (candidate,votes) in distributor.going_to.drain(..).enumerate() {
            if !votes.is_empty() {
                self.piles_by_candidate[candidate].by_count.insert(count, votes);
            }
        }
    }
    fn num_candidates(&self) -> NumberOfCandidates { NumberOfCandidates(self.piles_by_candidate.len()) }

    pub fn get_chooser<'a>(&'a self, candidate:CandidateIndex, election_data:&'a ElectionData, options:&ChooseVotesOptions) -> ChooseVotes<'a> {
        ChooseVotes::new(self,candidate,election_data,options)
    }

    /// Return true if an ATL vote for the candidate's party (if any) would be sitting on the candidate.
    pub fn is_highest_continuing_member_party_ticket(&self,candidate:CandidateIndex,metadata:&ElectionMetadata) -> bool {
        if let Some(party) = metadata.candidate(candidate).party {
            if !metadata.party(party).atl_allowed { return false; }
            for c in &metadata.party(party).candidates {
                if *c==candidate { return true }
                else if self.continuing.contains(c) { return false }
            }
            panic!("Candidate {} is not in their own party!",metadata.candidate(candidate).name);
        } else { false } // not in a party
    }
}


impl RetroscopeVoteStatus {
    fn next(&mut self, count:CountIndex, continuing:&HashSet<CandidateIndex>,distributor:&mut CandidateDistributor,vote_index: RetroscopeVoteIndex)  {
        self.count_arrived=count;
        if self.pile_status==PileStatus::InPile {
            while !continuing.contains(&self.prefs[self.upto]) {
                self.upto+=1;
                if self.upto==self.prefs.len() { self.pile_status=PileStatus::Exhausted; break; }
            }
        }
        if let Some(new_owner) = self.candidate() {
            distributor.going_to[new_owner.0].push(vote_index);
        }
    }

    /// Get the candidate in whose pile this ballot is.
    pub fn candidate(&self) -> Option<CandidateIndex> {
        if self.pile_status==PileStatus::InPile { Some(self.prefs[self.upto]) } else { None }
    }
}
