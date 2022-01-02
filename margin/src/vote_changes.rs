// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Describe a possible change in votes that may change the outcome of the election.
//! An estimate of the margin is the smallest such change that one could find.


use stv::ballot_metadata::CandidateIndex;
use serde::Serialize;
use serde::Deserialize;

/// A list of vote changes that may change the outcome of the election
/// These are conceptual, measured in votes. There may be a larger number of ballot papers involved.
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct VoteChanges<Tally> {
    pub changes : Vec<VoteChange<Tally>>,
}

impl <Tally:Clone> VoteChanges<Tally> {
    /// Add a command to transfer n votes from candidate `from` to candidate `to`.
    pub fn transfer(&mut self,n:Tally,from:CandidateIndex,to:CandidateIndex) {
        self.changes.push(VoteChange{
            n: n.clone(),
            from: Some(from),
            to: Some(to),
        })
    }
    /// Add a command to add n votes to candidate `to`.
    pub fn add(&mut self,n:Tally,to:CandidateIndex) {
        self.changes.push(VoteChange{
            n: n.clone(),
            from: None,
            to: Some(to),
        })
    }
    /// Add a command to remove n votes from candidate `from`.
    pub fn remove(&mut self,n:Tally,from:CandidateIndex) {
        self.changes.push(VoteChange{
            n: n.clone(),
            from: Some(from),
            to: None,
        })
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct VoteChange<Tally> {
    /// The number of votes to move
    pub n : Tally,
    /// The candidate to move from (or None, if the votes are just to be added)
    from : Option<CandidateIndex>,
    /// The candidate to move to (or None, if the votes are just to be added).
    to : Option<CandidateIndex>,
}

