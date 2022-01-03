// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Choose specific votes that were attached to a candidate at a particular count.

use std::ops::{AddAssign};
use num_traits::Zero;
use stv::ballot_metadata::CandidateIndex;
use stv::ballot_pile::BallotPaperCount;
use stv::distribution_of_preferences_transcript::CountIndex;
use stv::election_data::ElectionData;
use stv::preference_distribution::PreferenceDistributionRules;
use stv::transfer_value::TransferValue;
use crate::retroscope::{Retroscope, RetroscopeVoteIndex};
use serde::Serialize;
use serde::Deserialize;

#[derive(Clone,Copy,Debug,Serialize,Deserialize)]
pub struct TakeVotes {
    /// Which vote to take
    pub from : RetroscopeVoteIndex,
    /// The number of them to take. Guaranteed to be <= the n in the corresponding ATL or BTL structure.
    pub n : usize,
}
/// Allow extraction of votes for a particular candidate.
pub struct ChooseVotes<'a> {
    // options : ChooseVotesOptions,
    //retroscope : &'a Retroscope,
    election_data : &'a ElectionData,
    // sources sorted by transfer value. Take from the top first.
    sources : Vec<ChooseVotesUpTo<'a>>,
    // sources that have not been considered yet, sorted by transfer value. Take from top first.
    //remaining_sources : Vec<(CountIndex,&'a Vec<RetroscopeVoteIndex>)>,
    //upto : Option<ChooseVotesUpTo<'a>>
}
struct ChooseVotesUpTo<'a> {
    current_transfer_value : &'a TransferValue,
    atl : ChooseVotesIndistinguishableSet,
    btl : ChooseVotesIndistinguishableSet,
}
/// A set of votes all with the same TV, all arrived on same count, either all ATL or all BTL.
struct ChooseVotesIndistinguishableSet {
    current_votes : Vec<RetroscopeVoteIndex>,
    used_from_last_of_current_votes: usize,
    ballots_remaining : BallotPaperCount,
}

#[derive(Clone,Copy,Debug)]
/// What votes are allowed to be chosen
pub struct ChooseVotesOptions {
    /// Allow above the line votes to be taken
    pub allow_atl : bool,
    /// Allow votes that are sitting on the first preference to be taken
    pub allow_first_pref : bool,
}
/*
pub struct VotesAvailable<Tally> {
    atl : Tally,
    btl : Tally,
    total : Tally,
}*/
impl ChooseVotesIndistinguishableSet {
    /*
    fn votes_available<R:PreferenceDistributionRules>(&self,tv:&TransferValue) -> R::Tally {
        R::use_transfer_value(tv,self.ballots_remaining)
    }*/
    /// Get a specific number of ballots.
    fn take_votes(&mut self,election_data:&'_ ElectionData,to_take:BallotPaperCount,where_to_go:&mut Vec<TakeVotes>) {
        let mut togo = to_take.0;
        assert!(togo <= self.ballots_remaining.0);
        let num_atl = election_data.atl.len();
        while togo>0 {
            let source = self.current_votes.pop().unwrap();
            let is_atl = source.0 < num_atl;
            let n = (if is_atl { election_data.atl[source.0].n } else { election_data.btl[source.0-num_atl].n})-self.used_from_last_of_current_votes;
            let n_to_use = n.min(togo);
            where_to_go.push(TakeVotes{ from: source, n:n_to_use });
            togo-=n_to_use;
            self.ballots_remaining-=BallotPaperCount(n_to_use);
            if n>n_to_use {
                self.current_votes.push(source);
                self.used_from_last_of_current_votes +=n_to_use;
            } else {
                self.used_from_last_of_current_votes =0;
            }
        }
    }

}
impl <'a> ChooseVotesUpTo<'a> {
    fn votes_available_total<R:PreferenceDistributionRules>(&self) -> R::Tally { R::use_transfer_value(self.current_transfer_value,self.atl.ballots_remaining+self.btl.ballots_remaining) }
    fn votes_available_btl<R:PreferenceDistributionRules>(&self) -> R::Tally { R::use_transfer_value(self.current_transfer_value,self.btl.ballots_remaining) }
    /*
    fn votes_available<R:PreferenceDistributionRules>(&self) -> VotesAvailable<R::Tally> {
        VotesAvailable{
            atl: self.atl.votes_available(self.current_transfer_value),
            btl: self.atl.votes_available(self.current_transfer_value),
            total: self.votes_available_total(),
        }
    }*/
    fn new(count:CountIndex,ballots_to_consider:&'_ [RetroscopeVoteIndex],retroscope:&'a Retroscope,election_data:&ElectionData,options:&ChooseVotesOptions) -> Self {
        let mut res = ChooseVotesUpTo{
            current_transfer_value: retroscope.transfer_value(count),
            atl: ChooseVotesIndistinguishableSet {
                current_votes: vec![],
                used_from_last_of_current_votes: 0,
                ballots_remaining: BallotPaperCount(0),
            },
            btl: ChooseVotesIndistinguishableSet {
                current_votes: vec![],
                used_from_last_of_current_votes: 0,
                ballots_remaining: BallotPaperCount(0),
            }
        };
        let num_atl = election_data.atl.len();
        for &v in ballots_to_consider {
            let is_atl = v.0 < num_atl;
            if is_atl {
                if options.allow_atl {
                    if options.allow_first_pref || retroscope.votes.btl[v.0-num_atl].upto>=election_data.metadata.party(election_data.atl[v.0].parties[0]).candidates.len() {
                        res.atl.current_votes.push(v);
                        res.atl.ballots_remaining+=BallotPaperCount(election_data.atl[v.0].n);
                    }
                }
            } else {
                if options.allow_first_pref || retroscope.votes.btl[v.0-num_atl].upto>0 {
                    res.btl.current_votes.push(v);
                    res.btl.ballots_remaining+=BallotPaperCount(election_data.btl[v.0-num_atl].n);
                }
            }
        }
        res
    }
    /// Get a specific number of ballots, taking BTL ones first.
    fn take_votes(&mut self,election_data:&'_ ElectionData,to_take:BallotPaperCount) -> Vec<TakeVotes> {
        let mut res = vec![];
        let take_btl = to_take.min(self.btl.ballots_remaining);
        let take_atl = to_take-take_btl;
        if !take_btl.is_zero() { self.btl.take_votes(election_data,take_btl,&mut res); }
        if !take_atl.is_zero() { self.atl.take_votes(election_data,take_atl,&mut res); }
        res
    }
    fn get_votes<R:PreferenceDistributionRules>(&mut self,election_data:&'_ ElectionData,wanted:R::Tally,allow_atl:bool) -> BallotsWithGivenTransferValue<R::Tally> {
        let max_available = if allow_atl { self.votes_available_total::<R>() } else { self.votes_available_btl::<R>() };
        let to_take = wanted.min(max_available);
        let ballots_needed = self.current_transfer_value.num_ballot_papers_to_get_this_tv(R::convert_tally_to_rational(to_take.clone()));
        BallotsWithGivenTransferValue{
            n: ballots_needed,
            tally: to_take,
            tv: self.current_transfer_value.clone(),
            ballots: self.take_votes(election_data,ballots_needed),
        }
    }
}
impl <'a> ChooseVotes<'a> {
    pub (crate) fn new(retroscope:&'a Retroscope,candidate:CandidateIndex,election_data:&'a ElectionData,options : ChooseVotesOptions) -> Self {
        let by_count = &retroscope.piles_by_candidate[candidate.0].by_count;
        let mut remaining_sources = by_count.iter().map(|(&count,votes)|(count,votes)).collect::<Vec<_>>();
        remaining_sources.sort_by_key(|(count,_)|retroscope.transfer_value(*count));
        let sources = remaining_sources.iter().map(|(count,ballots_to_consider)|ChooseVotesUpTo::new(*count,ballots_to_consider,retroscope,election_data,&options)).collect();
        ChooseVotes{
            //options,
            //retroscope,
            election_data,
            sources,
            // remaining_sources,
            // upto: None,
        }
    }
    /// Total available votes, both below and above the line, taking rounding into account.
    pub fn votes_available_total<R:PreferenceDistributionRules>(&self) -> R::Tally { self.sources.iter().map(|s|s.votes_available_total::<R>()).sum() }
    /// available votes below the line, taking rounding into account.
    pub fn votes_available_btl<R:PreferenceDistributionRules>(&self) -> R::Tally { self.sources.iter().map(|s|s.votes_available_btl::<R>()).sum() }

    /// If possible, get a set of ballots that will provide the wanted number of votes.
    /// This will preferentially use votes with a large transfer value.
    /// BTL votes will be used preferentially, but atl votes will be used if allow_atl is true.
    pub fn get_votes<R:PreferenceDistributionRules>(&mut self,wanted:R::Tally,allow_atl:bool) -> Option<Vec<BallotsWithGivenTransferValue<R::Tally>>> {
        let mut res = vec![];
        let mut sofar = R::Tally::zero();
        for i in (0..self.sources.len()).rev() {
            let parcel = self.sources[i].get_votes::<R>(self.election_data,wanted.clone()-sofar.clone(),allow_atl);
            if parcel.n.is_zero() {
                if i+1==self.sources.len() && (allow_atl||self.sources[i].atl.ballots_remaining.is_zero()) {
                    self.sources.pop(); // remove empty source from future considerations.
                }
            } else {
                sofar+=parcel.tally.clone();
                res.push(parcel);
                if sofar>=wanted { return Some(res) }
            }
        }
        None
    }

}

/*
#[derive(Clone,Debug)]
pub struct FoundVotes<Tally> {
    pub which_votes : Vec<TakeVotes>,
    pub papers : BallotPaperCount,
    pub tally : Tally,
}

impl <Tally:AddAssign> FoundVotes<Tally> {
    pub(crate) fn add(&mut self,other:FoundVotes<Tally>) {
        self.which_votes.extend(other.which_votes);
        self.papers+=other.papers;
        self.tally+=other.tally;
    }
}

 */


/// A concrete set of ballot level changes that are all similar - same TV, same source candidate, same destination candidate.
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct BallotsWithGivenTransferValue<Tally> {
    pub n : BallotPaperCount,
    pub tally : Tally,
    pub tv : TransferValue,
    pub ballots : Vec<TakeVotes>
}

impl <Tally:AddAssign> BallotsWithGivenTransferValue<Tally> {
    pub(crate) fn add(&mut self,other:BallotsWithGivenTransferValue<Tally>) {
        self.ballots.extend(other.ballots);
        self.n+=other.n;
        self.tally+=other.tally;
        assert_eq!(self.tv,other.tv);
    }
}

