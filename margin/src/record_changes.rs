// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Keep track of the best changes for a given election.



use std::collections::HashSet;
use std::fmt::Debug;
use stv::compare_transcripts::DeltasInCandidateLists;
use stv::election_data::ElectionData;
use crate::vote_changes::BallotChanges;
use serde::Serialize;
use serde::Deserialize;
use stv::ballot_pile::BallotPaperCount;
use crate::evaluate_and_optimize_vote_changes::FoundChange;

/// Sufficient information to document one or more changes to an election completely.
/// Keeps track of the best change, or changes if they are not comparable (e.g cause different candidates to be elected).
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ElectionChanges<Tally:Clone> {
    pub original : ElectionData,
    pub changes : Vec<ElectionChange<Tally>>,
    pub ballot_types_considered_unverifiable : HashSet<String>,
}

#[derive(Clone,Copy,Debug,Serialize,Deserialize)]
pub struct ChangeTypes {
    pub changed_first_preference : bool,
    pub changed_atl : bool,
    pub added_ballots : bool,
    pub removed_ballots : bool,
    pub changed_ballots : bool,
    pub affected_verifiable_ballots: bool,
}

impl ChangeTypes {
    /// true iff self is of no interest given the existence of other.
    pub fn is_dominated_by_or_equivalent_to(&self,other:&Self) -> bool {
        (self.changed_first_preference || !other.changed_first_preference) &&
            (self.changed_atl || !other.changed_atl) &&
            (self.added_ballots || !other.added_ballots) &&
            (self.removed_ballots || !other.removed_ballots) &&
            (self.changed_ballots || !other.changed_ballots) &&
            (self.affected_verifiable_ballots || !other.affected_verifiable_ballots)
    }
    /// Deduce what properties the ballots have.
    /// If not empty, ballot_types_considered_unverifiable contains the vote types deemed to be unverifiable.
    pub fn deduce<Tally>(ballots:&BallotChanges<Tally>,data:&ElectionData,ballot_types_considered_unverifiable:&HashSet<String>) -> Self {
        let mut res = ChangeTypes{
            changed_first_preference: false,
            changed_atl: false,
            added_ballots: false,
            removed_ballots: false,
            changed_ballots: false,
            affected_verifiable_ballots: false
        };
        let num_atl = data.atl.len();
        for bcs in &ballots.changes {
            if bcs.from.is_none() { res.added_ballots=true; }
            if bcs.candidate_to.is_none() { res.removed_ballots=true; }
            if bcs.from.is_some() && bcs.candidate_to.is_some() { res.changed_ballots=true; }
            if let Some(from) = bcs.from.as_ref() {
                for b in &from.ballots {
                    if b.from.0<num_atl { // it is an ATL vote
                        res.changed_atl=true;
                        if data.metadata.candidate(from.candidate).party.unwrap()==data.atl[b.from.0].parties[0] { res.changed_first_preference=true; }
                        if !ballot_types_considered_unverifiable.is_empty() {
                            if data.is_atl_verifiable(b.from.0,ballot_types_considered_unverifiable) { res.affected_verifiable_ballots =true; }
                        }
                    } else { // it is a BTL vote
                        if from.candidate==data.btl[b.from.0-num_atl].candidates[0] { res.changed_first_preference=true; }
                        if !ballot_types_considered_unverifiable.is_empty() {
                            if data.is_btl_verifiable(b.from.0-num_atl,ballot_types_considered_unverifiable) { res.affected_verifiable_ballots =true; }
                        }
                    }
                }
            }
        }
        res
    }
}

/// A single instance of some change to the ballots that affects who is elected.
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ElectionChange<Tally:Clone> {
    pub outcome : DeltasInCandidateLists,
    pub requires : ChangeTypes,
    pub ballots : BallotChanges<Tally>,
}


impl <Tally:Clone> ElectionChange<Tally> {
    /// true iff self is of no interest given the existence of other.
    pub fn is_dominated_by_or_equivalent_to(&self,other:&Self) -> bool {
        self.outcome==other.outcome &&
            self.requires.is_dominated_by_or_equivalent_to(&other.requires) &&
            self.ballots.n>=other.ballots.n
    }

    pub fn new(outcome:DeltasInCandidateLists,ballots:BallotChanges<Tally>,data:&ElectionData,ballot_types_considered_unverifiable:&HashSet<String>) -> Self {
        let requires = ChangeTypes::deduce(&ballots,data,ballot_types_considered_unverifiable);
        ElectionChange{
            outcome,
            requires,
            ballots,
        }
    }
}

impl <Tally:Clone> ElectionChanges<Tally> {
    pub fn new(data:&ElectionData,ballot_types_considered_unverifiable:&HashSet<String>) -> Self { ElectionChanges { original: data.clone(), changes: vec![] , ballot_types_considered_unverifiable:ballot_types_considered_unverifiable.clone()} }

    /// Add a change, if there is no strictly better one already known.
    pub fn add_change(&mut self,change:ElectionChange<Tally>) {
        println!("Recorder given a change of {} ballots",change.ballots.n);
        for existing in &self.changes {
            if change.is_dominated_by_or_equivalent_to(existing) { return; } // no point keeping it.
        }
        // see if any existing should be removed
        self.changes.retain(|existing|!existing.is_dominated_by_or_equivalent_to(&change));
        println!("This is a new personal best.");
        self.changes.push(change);
    }

    pub fn merge(&mut self,other:Self) {
        for v in other.changes {
            self.add_change(v);
        }
    }
    /// add an outcome once found.
    pub fn add(&mut self,found:FoundChange<Tally>) {
        self.add_change(ElectionChange::new(found.deltas,found.changes,&self.original,&self.ballot_types_considered_unverifiable));
    }

    pub fn smallest_manipulation_found(&self) -> Option<BallotPaperCount> {
        self.changes.iter().map(|c|c.ballots.n).min()
    }

    pub fn sort(&mut self) {
        self.changes.sort_by_key(|c|c.ballots.n);
    }
}

