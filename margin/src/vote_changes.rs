// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Describe a possible change in votes that may change the outcome of the election.
//! An estimate of the margin is the smallest such change that one could find.


use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::iter::Sum;
use std::ops::{AddAssign, Sub, SubAssign};
use std::str::FromStr;
use num_traits::Zero;
use stv::ballot_metadata::{CandidateIndex, PartyIndex};
use serde::Serialize;
use serde::Deserialize;
use stv::ballot_paper::{ATL, BTL};
use stv::ballot_pile::BallotPaperCount;
use stv::compare_transcripts::{DeltasInCandidateLists, DifferentCandidateLists};
use stv::election_data::ElectionData;
use stv::preference_distribution::{PreferenceDistributionRules, RoundUpToUsize};
use stv::transfer_value::TransferValue;
use crate::choose_votes::{BallotsWithGivenTransferValue, ChooseVotes, ChooseVotesOptions, TakeVotes};
use crate::retroscope::{Retroscope};

/// A list of vote changes that may change the outcome of the election
/// These are conceptual, measured in votes. There may be a larger number of ballot papers involved.
/// They can be turned into concrete actual ballots by calling [`Self::make_concrete()`].
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct VoteChanges<Tally> {
    pub changes : Vec<VoteChange<Tally>>,
}

impl <Tally:Display> Display for VoteChanges<Tally> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.changes.len() {
            if i!=0 { write!(f," & ")?}
            write!(f,"{}",self.changes[i])?;
        }
        Ok(())
    }
}

impl <Tally:Clone+RoundUpToUsize> VoteChanges<Tally> {
    /// Add a command to transfer n votes from candidate `from` to candidate `to`.
    pub fn transfer(&mut self, n: Tally, from: CandidateIndex, to: CandidateIndex) {
        self.changes.push(VoteChange {
            vote_value: n.clone(),
            from: Some(from),
            to: Some(to),
        })
    }
    /// Add a command to add n votes to candidate `to`.
    pub fn add(&mut self, n: Tally, to: CandidateIndex) {
        self.changes.push(VoteChange {
            vote_value: n.clone(),
            from: None,
            to: Some(to),
        })
    }
    /// Add a command to remove n votes from candidate `from`.
    pub fn remove(&mut self, n: Tally, from: CandidateIndex) {
        self.changes.push(VoteChange {
            vote_value: n.clone(),
            from: Some(from),
            to: None,
        })
    }
}

impl <Tally:Clone+AddAssign+SubAssign+From<usize>+Display+PartialEq+Serialize+FromStr+Ord+Sub<Output=Tally>+Zero+Hash+Sum<Tally>+RoundUpToUsize> VoteChanges<Tally> {
    pub fn make_concrete<R:PreferenceDistributionRules<Tally=Tally>>(&self,retroscope:&Retroscope,election_data:&ElectionData,options:&ChooseVotesOptions) -> Option<BallotChanges<Tally>> {
        let mut builder = BallotChangesBuilder{ map: HashMap::new() };
        let mut choosers : HashMap<CandidateIndex,ChooseVotes> = HashMap::new();
        let (atl_ok_changes,btl_only_changes):(Vec<_>,Vec<_>) = self.changes.iter().partition(|vc|vc.to.map(|c|retroscope.is_highest_continuing_member_party_ticket(c,&election_data.metadata)).unwrap_or(true));
        for (change,allow_atl) in btl_only_changes.iter().map(|&x|(x,false)).chain(atl_ok_changes.iter().map(|&x|(x,true))) {
            if change.vote_value==Tally::zero() { continue; }
            if let Some(from) = change.from {
                let chooser = choosers.entry(from).or_insert_with(||retroscope.get_chooser(from,election_data,options));
                // println!("Trying to find {} votes from {} allowing ATL: {} Total available : {}   BTL available : {}",change.vote_value,from,allow_atl,chooser.votes_available_total::<R>(),chooser.votes_available_btl::<R>());
                if let Some(ballots) = chooser.get_votes::<R>(change.vote_value.clone(),allow_atl) {
                    for b in ballots {
                        builder.add(change.from,change.to,b);
                    }
                } else {
                    return None;
                } // could not find the requisite votes.
            } else {
                if let Some(_to) = change.to { // insert votes
                    builder.add(change.from,change.to,BallotsWithGivenTransferValue{
                        n: BallotPaperCount(change.vote_value.ceil()),
                        tally: change.vote_value.clone(),
                        tv: TransferValue::one(),
                        ballots: vec![],
                    });
                } else { eprintln!("Trying to do a vote change that does nothing."); } // don't actually do anything...
            }
        }
        Some(builder.to_ballot_changes())
    }

    /// utility used in optimization that changes one of the values to some new value.
    pub(crate) fn change_single_value<TallyLike:Into<Tally>>(&self,which_subelem:usize,new_vote_value:TallyLike) -> Self {
        let mut res = self.clone();
        res.changes[which_subelem].vote_value=new_vote_value.into();
        res
    }
}

#[derive(Clone,Debug,Eq,PartialEq,Hash)]
struct BallotChangesKey {
    from : Option<(TransferValue,CandidateIndex)>,
    to : Option<CandidateIndex>,
}
/// Utility to build a BallotChanges object.
struct BallotChangesBuilder<Tally> {
    map : HashMap<BallotChangesKey,BallotsWithGivenTransferValue<Tally>>,
}

impl <Tally:AddAssign> BallotChangesBuilder<Tally> {
    fn to_ballot_changes(self) -> BallotChanges<Tally> {
        let mut changes : Vec<_> = self.map.into_iter().map(|(key,value)|{
            BallotChangeSimilar{
                n: value.n,
                tally: value.tally,
                from: if let Some((tv,candidate)) = key.from { Some(BallotsFromCandidateWithGivenTransferValue{ candidate,ballots:value.ballots,tv})} else {None},
                candidate_to: key.to
            }
        }).collect();
        // do a series of stable sorts to sort by first who from, then who to, then TV.
        changes.sort_by_key(|c|c.from.as_ref().map(|f|f.tv.clone()));
        changes.reverse();
        changes.sort_by_key(|c|c.candidate_to.map(|c|c.0));
        changes.sort_by_key(|c|c.from.as_ref().map(|f|f.candidate.0));
        let n = if changes.is_empty() {BallotPaperCount(0)} else { changes.iter().map(|c|c.n).sum()};
        BallotChanges{ changes,n }
    }
    fn add(&mut self,from:Option<CandidateIndex>, to:Option<CandidateIndex>,found:BallotsWithGivenTransferValue<Tally>) {
        let entry = self.map.entry(BallotChangesKey{from:from.map(|c|(found.tv.clone(),c)),to});
        match &entry {
            Entry::Occupied(_) => {entry.and_modify(|f|f.add(found)); }
            Entry::Vacant(_) => {entry.or_insert(found); }
        }
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
/// A "vote level" change - take a number of votes from one candidate and give to another.
pub struct VoteChange<Tally> {
    /// The number of votes to move
    pub vote_value : Tally,
    /// The candidate to move from (or None, if the votes are just to be added)
    pub from : Option<CandidateIndex>,
    /// The candidate to move to (or None, if the votes are just to be added).
    pub to : Option<CandidateIndex>,
}

impl <Tally:Display> Display for VoteChange<Tally> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{} votes {} → {}",self.vote_value,self.from.map(|c|c.to_string()).unwrap_or("-".to_string()),self.to.map(|c|c.to_string()).unwrap_or("-".to_string()))
    }
}
/// A bunch of votes taken from the same candidate with the same transfer value.
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct BallotsFromCandidateWithGivenTransferValue {
    // the candidate whose votes are being taken away.
    pub candidate : CandidateIndex,
    pub ballots : Vec<TakeVotes>,
    pub tv : TransferValue,
}

/// A concrete set of ballot level changes that are all similar - same TV, same source candidate, same destination candidate.
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct BallotChangeSimilar<Tally> {
    pub n : BallotPaperCount,
    pub tally: Tally,
    pub from : Option<BallotsFromCandidateWithGivenTransferValue>,
    pub candidate_to : Option<CandidateIndex>,
}

/// A concrete set of ballot level changes.
/// They can be applied to vote data by calling [`Self::apply_to_votes`].
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct BallotChanges<Tally> {
    pub changes : Vec<BallotChangeSimilar<Tally>>,
    pub n : BallotPaperCount,
}

impl <Tally> BallotChanges<Tally> {
    pub fn apply_to_votes(&self, election_data: &ElectionData, verbose: bool) -> ElectionData {
        let mut data = election_data.clone();
        let num_atl = data.atl.len();
        for change in &self.changes {
            if let Some(from) = change.from.as_ref() {
                for wv in &from.ballots {
                    if wv.from.0 < num_atl { // It is an ATL vote
                        data.atl[wv.from.0].n -= wv.n;
                        if let Some(to) = change.candidate_to {
                            let from_party = election_data.metadata.candidate(from.candidate).party.unwrap(); // must have a party or couldn't be in an ATL vote.
                            if let Some(to_party) = election_data.metadata.candidate(to).party {
                                let new_parties: Vec<PartyIndex> = data.atl[wv.from.0].parties.iter().filter(|&&c| c != to_party).map(|&c| if c == from_party { to_party } else { c }).collect();
                                if verbose {
                                    println!("Changed {} ATL from [{}] to [{}]", wv.n, data.metadata.party_list_to_string(&data.atl[wv.from.0].parties), data.metadata.party_list_to_string(&new_parties));
                                }
                                data.atl.push(ATL { parties: new_parties, n: wv.n, ticket_index: if data.atl[wv.from.0].ticket_index.is_some() { Some(0)} else {None} }) // the ticket index is a hack, and is not accurate. The margin computation is not designed for ticket ATL modifications.
                            } else {
                                panic!("Candidate {} got ATL vote but doesn't have a party.", election_data.metadata.candidate(from.candidate).name);
                            }
                        } else if verbose {
                            println!("Removed {} ATL votes [{}]", wv.n, data.metadata.party_list_to_string(&data.atl[wv.from.0].parties));
                        }
                    } else { // It is a BTL vote.
                        data.btl[wv.from.0 - num_atl].n -= wv.n;
                        if let Some(to) = change.candidate_to {
                            let new_candidates: Vec<CandidateIndex> = data.btl[wv.from.0 - num_atl].candidates.iter().filter(|&&c| c != to).map(|&c| if c == from.candidate { to } else { c }).collect();
                            if verbose {
                                println!("Changed {} BTL from [{}] to [{}]", wv.n, data.metadata.candidate_list_to_string(&data.btl[wv.from.0 - num_atl].candidates), data.metadata.candidate_list_to_string(&new_candidates));
                            }
                            data.btl.push(BTL { candidates: new_candidates, n: wv.n })
                        } else if verbose {
                            println!("Removed {} BTL votes [{}]", wv.n, data.metadata.candidate_list_to_string(&data.btl[wv.from.0 - num_atl].candidates));
                        }
                    }
                }
            } else {
                if let Some(to) = change.candidate_to { // insert votes
                    data.btl.push(BTL { candidates: vec![to], n: change.n.0 });
                    if verbose {}
                } else { eprintln!("Trying to do a vote change that does nothing."); } // don't actually do anything...
            }
        }
        data
    }
}
impl <Tally:PartialEq+Clone+Display+FromStr+Debug> BallotChanges<Tally> {
    pub fn see_effect<R:PreferenceDistributionRules<Tally=Tally>>(&self, election_data:&ElectionData) -> DeltasInCandidateLists {
        let changed_data = self.apply_to_votes(election_data,false);
        let transcript = changed_data.distribute_preferences::<R>();
        let diffs  : DeltasInCandidateLists = DifferentCandidateLists{ list1: transcript.elected.clone(), list2: election_data.metadata.results.as_ref().unwrap().clone() }.into();
        diffs
    }
}