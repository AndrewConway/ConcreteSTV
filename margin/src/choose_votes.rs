// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Choose specific votes that were attached to a candidate at a particular count.

use num_traits::Zero;
use stv::ballot_metadata::CandidateIndex;
use stv::ballot_pile::BallotPaperCount;
use stv::distribution_of_preferences_transcript::CountIndex;
use stv::election_data::ElectionData;
use stv::preference_distribution::PreferenceDistributionRules;
use stv::transfer_value::TransferValue;
use crate::retroscope::{Retroscope, RetroscopeVoteIndex};

#[derive(Clone,Copy,Debug)]
pub struct TakeVotes {
    /// Which vote to take
    pub from : RetroscopeVoteIndex,
    /// The number of them to take. Guaranteed to be <= the n in the corresponding ATL or BTL structure.
    pub n : usize,
}
/// Allow extraction of votes for a particular candidate.
pub struct ChooseVotes<'a> {
    options : ChooseVotesOptions,
    retroscope : &'a Retroscope,
    election_data : &'a ElectionData,
    /// sources that have not been considered yet, sorted by transfer value. Take from top first.
    remaining_sources : Vec<(CountIndex,&'a Vec<RetroscopeVoteIndex>)>,
    upto : Option<ChooseVotesUpTo<'a>>
}
struct ChooseVotesUpTo<'a> {
    current_transfer_value : &'a TransferValue,
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
impl <'a> ChooseVotesUpTo<'a> {
    /// Get a specific number of ballots.
    fn take_votes(&mut self,election_data:&'_ ElectionData,to_take:BallotPaperCount) -> Vec<TakeVotes> {
        let mut togo = to_take.0;
        let mut res = vec![];
        let num_atl = election_data.atl.len();
        while togo>0 {
            let source = self.current_votes.pop().unwrap();
            let is_atl = source.0 < num_atl;
            let n = (if is_atl { election_data.atl[source.0].n } else { election_data.btl[source.0-num_atl].n})-self.used_from_last_of_current_votes;
            let n_to_use = n.min(togo);
            res.push(TakeVotes{ from: source, n:n_to_use });
            togo-=n_to_use;
            self.ballots_remaining-=BallotPaperCount(n_to_use);
            if n>n_to_use {
                self.current_votes.push(source);
                self.used_from_last_of_current_votes +=n_to_use;
            } else {
                self.used_from_last_of_current_votes =0;
            }
        }
        res
    }
    fn get_votes<R:PreferenceDistributionRules>(&mut self,election_data:&'_ ElectionData,wanted:R::Tally) -> FoundVotes<R::Tally> {
        let max_available = R::use_transfer_value(self.current_transfer_value,self.ballots_remaining);
        let to_take = wanted.min(max_available);
        let ballots_needed = self.current_transfer_value.num_ballot_papers_to_get_this_tv(R::convert_tally_to_rational(to_take.clone()));
        FoundVotes{
            which_votes: self.take_votes(election_data,ballots_needed),
            papers: ballots_needed,
            tally: to_take,
        }
    }
}
impl <'a> ChooseVotes<'a> {
    pub (crate) fn new(retroscope:&'a Retroscope,candidate:CandidateIndex,election_data:&'a ElectionData,options : ChooseVotesOptions) -> Self {
        let by_count = &retroscope.piles_by_candidate[candidate.0].by_count;
        let mut remaining_sources = by_count.iter().map(|(&count,votes)|(count,votes)).collect::<Vec<_>>();
        remaining_sources.sort_by_key(|(count,_)|retroscope.transfer_value(*count));
        ChooseVotes{
            options,
            retroscope,
            election_data,
            remaining_sources,
            upto: None,
        }
    }
    /// make sure self.upto contains something, if possible, by taking it, if needed, from remaining_sources unless that is empty.
    fn make_sure_upto_has_something_if_possible(&mut self) {
        if self.upto.is_none() {
            if let Some((count,votes_at_this_count)) = self.remaining_sources.pop() {
                let mut current_votes = vec![];
                let mut ballots_remaining = BallotPaperCount(0);
                let num_atl = self.election_data.atl.len();
                for &v in votes_at_this_count {
                    let is_atl = v.0 < num_atl;
                    let ok = (self.options.allow_atl || !is_atl) &&
                        (self.options.allow_first_pref || (if is_atl { &self.retroscope.votes.atl[v.0] } else { &self.retroscope.votes.btl[v.0-num_atl] }).upto>0);
                    if ok {
                        current_votes.push(v);
                        ballots_remaining+=BallotPaperCount(if is_atl { self.election_data.atl[v.0].n} else { self.election_data.btl[v.0-num_atl].n });
                    }
                }
                self.upto = Some(ChooseVotesUpTo{
                    current_transfer_value: self.retroscope.transfer_value(count),
                    current_votes,
                    used_from_last_of_current_votes: 0,
                    ballots_remaining,
                });
            }
        }
    }
    /// Get up to wanted votes from a single tally. This is done so that rounding can be done sensible. None if nothing left.
    fn get_parcel_of_votes_from_a_single_count<R:PreferenceDistributionRules>(&mut self,wanted:R::Tally) -> Option<FoundVotes<R::Tally>> {
        loop {
            self.make_sure_upto_has_something_if_possible();
            if let Some(upto) = self.upto.as_mut() {
                let from_here = upto.get_votes::<R>(self.election_data,wanted.clone());
                if from_here.papers.is_zero() { self.upto=None }
                else { return Some(from_here) }
            } else { return None }
        }
    }
    /// If possible, get a set of ballots that will provide the wanted number of votes.
    /// This will preferentially use a large number of votes.
    pub fn get_votes<R:PreferenceDistributionRules>(&mut self,wanted:R::Tally) -> Option<FoundVotes<R::Tally>> {
        let mut which_votes = vec![];
        let mut papers = BallotPaperCount::zero();
        let mut togo = wanted.clone();
        while !togo.is_zero() {
            if let Some(parcel) = self.get_parcel_of_votes_from_a_single_count::<R>(togo.clone()) {
                togo-=parcel.tally;
                papers+=parcel.papers;
                which_votes.extend(parcel.which_votes.into_iter());
            } else { return None }
        }
        Some(FoundVotes{
            which_votes,
            papers,
            tally: wanted,
        })
    }

}

#[derive(Clone,Debug)]
pub struct FoundVotes<Tally> {
    pub which_votes : Vec<TakeVotes>,
    pub papers : BallotPaperCount,
    pub tally : Tally,
}