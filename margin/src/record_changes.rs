// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Keep track of the best changes for a given election.



use std::collections::HashSet;
use std::fmt::{Debug, Display};
use std::str::FromStr;
use stv::compare_transcripts::DeltasInCandidateLists;
use stv::election_data::ElectionData;
use crate::vote_changes::BallotChanges;
use serde::Serialize;
use serde::Deserialize;
use stv::ballot_pile::BallotPaperCount;
use stv::preference_distribution::PreferenceDistributionRules;
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
/// Various things that affect how detectable a manipulation is.
/// Each of the following flags are set if there is at least affected ballot
/// for which they apply.
pub struct ChangeTypes {
    pub changed_first_preference : bool, // a first preference (ATL or BTL) was changed. Some jurisdictions manually record first preferences.
    pub changed_atl : bool, // If an above the line vote was changed. Some jurisdictions have(or had) single ATLs that are recorded.
    pub added_ballots : bool, // if ballots were added rather than modified
    pub removed_ballots : bool, // if ballots were removed
    pub changed_ballots : bool, // if ballots were changed (rather than added or removed).
    pub affected_verifiable_ballots: bool, // if ballot_types_considered_unverifiable is not empty, but does not contain the current ballot
    #[serde(default)]
    pub directly_benefited_new_winner: bool, // if a change gives votes to a candidate who ended up winning a seat as a result of the modification
    #[serde(default)]
    pub directly_hurt_new_loser: bool, // if a change takes votes from a candidate who ended up winning a seat as a result of the modification
}

impl ChangeTypes {
    /// true iff self is of no interest given the existence of other.
    pub fn is_dominated_by_or_equivalent_to(&self,other:&Self) -> bool {
        (self.changed_first_preference || !other.changed_first_preference) &&
            (self.changed_atl || !other.changed_atl) &&
            (self.added_ballots || !other.added_ballots) &&
            (self.removed_ballots || !other.removed_ballots) &&
            (self.changed_ballots || !other.changed_ballots) &&
            (self.affected_verifiable_ballots || !other.affected_verifiable_ballots) &&
            (self.directly_benefited_new_winner || !other.directly_benefited_new_winner) &&
            (self.directly_hurt_new_loser || !other.directly_hurt_new_loser)
    }
    /// Deduce what properties the ballots have.
    /// If not empty, ballot_types_considered_unverifiable contains the vote types deemed to be unverifiable.
    pub fn deduce<Tally>(ballots:&BallotChanges<Tally>, data:&ElectionData, ballot_types_considered_unverifiable:&HashSet<String>, outcome: &DeltasInCandidateLists) -> Self {
        let mut res = ChangeTypes{
            changed_first_preference: false,
            changed_atl: false,
            added_ballots: false,
            removed_ballots: false,
            changed_ballots: false,
            affected_verifiable_ballots: false,
            directly_benefited_new_winner: false,
            directly_hurt_new_loser: false
        };
        let num_atl = data.atl.len();
        for bcs in &ballots.changes {
            if bcs.from.is_none() { res.added_ballots=true; }
            if bcs.candidate_to.is_none() { res.removed_ballots=true; }
            if bcs.from.is_some() && bcs.candidate_to.is_some() { res.changed_ballots=true; }
            let mut found_atl_this_batch = false; // only check party equivalence if relevent. An optimization for from, necessary for to.
            if let Some(from) = bcs.from.as_ref() {
                let from_party = data.metadata.candidate(from.candidate).party;
                for b in &from.ballots {
                    if b.from.0<num_atl { // it is an ATL vote
                        res.changed_atl=true;
                        if from_party.unwrap()==data.atl[b.from.0].first_party() { res.changed_first_preference=true; }
                        if !ballot_types_considered_unverifiable.is_empty() {
                            if data.is_atl_verifiable(b.from.0,ballot_types_considered_unverifiable) { res.affected_verifiable_ballots =true; }
                        }
                        found_atl_this_batch=true;
                    } else { // it is a BTL vote
                        if from.candidate==data.btl[b.from.0-num_atl].candidates[0] { res.changed_first_preference=true; }
                        if !ballot_types_considered_unverifiable.is_empty() {
                            if data.is_btl_verifiable(b.from.0-num_atl,ballot_types_considered_unverifiable) { res.affected_verifiable_ballots =true; }
                        }
                    }
                }
                if outcome.list2only.contains(&from.candidate) { res.directly_hurt_new_loser=true; }
                if found_atl_this_batch {
                    if outcome.list2only.iter().any(|&c|data.metadata.candidate(c).party==from_party) { res.directly_hurt_new_loser=true; }
                }
            }
            if let Some(to) = bcs.candidate_to {
                if outcome.list1only.contains(&to) { res.directly_benefited_new_winner=true; }
                if found_atl_this_batch {
                    let to_party = data.metadata.candidate(to).party;
                    if outcome.list1only.iter().any(|&c|data.metadata.candidate(c).party==to_party) { res.directly_benefited_new_winner=true; }
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
        let requires = ChangeTypes::deduce(&ballots,data,ballot_types_considered_unverifiable,&outcome);
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
    pub fn add_change(&mut self,change:ElectionChange<Tally>,verbose:bool) {
        if verbose { println!("Recorder given a change of {} ballots",change.ballots.n); }
        for existing in &self.changes {
            if change.is_dominated_by_or_equivalent_to(existing) { return; } // no point keeping it.
        }
        // see if any existing should be removed
        self.changes.retain(|existing|!existing.is_dominated_by_or_equivalent_to(&change));
        if verbose { println!("This is a new personal best."); }
        self.changes.push(change);
    }

    /// Add in an existing data structure
    pub fn merge(&mut self,other:Self,verbose:bool) {
        for v in other.changes {
            self.add_change(v,verbose);
        }
    }
    /// add an outcome once found.
    pub fn add(&mut self,found:FoundChange<Tally>,verbose:bool) {
        self.add_change(ElectionChange::new(found.deltas,found.changes,&self.original,&self.ballot_types_considered_unverifiable),verbose);
    }

    pub fn smallest_manipulation_found(&self) -> Option<BallotPaperCount> {
        self.changes.iter().map(|c|c.ballots.n).min()
    }

    pub fn sort(&mut self) {
        self.changes.sort_by_key(|c|c.ballots.n);
    }
}

impl <Tally:PartialEq+Clone+Display+FromStr> ElectionChanges<Tally> {
    /// Add in a (suspicious, possible old) extra data structure, reevaluating everything
    pub fn merge_reevaluating<R:PreferenceDistributionRules<Tally=Tally>>(&mut self,other:&[ElectionChange<Tally>],election_data:&ElectionData,ballot_types_considered_unverifiable:&HashSet<String>,verbose:bool) {
        for v in other {
            let deltas  : DeltasInCandidateLists = v.ballots.see_effect::<R>(election_data);
            if !deltas.is_empty() {
                self.add_change(ElectionChange::new(deltas,v.ballots.clone(),election_data,ballot_types_considered_unverifiable),verbose);
            }
        }
    }
}