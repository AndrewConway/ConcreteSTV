// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! This takes a VoteChanges, sees if it changes anything, and tries to modify it slightly to improve.






use num_traits::Zero;
use stv::compare_transcripts::{DeltasInCandidateLists, DifferentCandidateLists};
use stv::election_data::ElectionData;
use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules, RoundUpToUsize};
use crate::choose_votes::ChooseVotesOptions;
use crate::retroscope::Retroscope;
use crate::vote_changes::{BallotChanges, VoteChanges};


pub enum ChangeResult<Tally> {
    NotEnoughVotesAvailable,
    NoChange,
    Change(DeltasInCandidateLists,BallotChanges<Tally>)
}

/// Test the effect of the provided changes on the election.
/// ElectionData must contain vacancy information and results (official winners).
pub fn simple_test<R:PreferenceDistributionRules>(vote_changes:&VoteChanges<R::Tally>,election_data:&ElectionData,retroscope:&Retroscope,options:&ChooseVotesOptions) -> ChangeResult<R::Tally> {
    if let Some(ballot_changes) = vote_changes.make_concrete::<R>(retroscope,election_data,options) {
        let changed_data = ballot_changes.apply_to_votes(election_data,false);
        let transcript = distribute_preferences::<R>(&changed_data,election_data.metadata.vacancies.unwrap(),&election_data.metadata.excluded.iter().cloned().collect(),&election_data.metadata.tie_resolutions,false);
        let diffs  : DeltasInCandidateLists = DifferentCandidateLists{ list1: transcript.elected.clone(), list2: election_data.metadata.results.as_ref().unwrap().clone() }.into();
        if diffs.is_empty() { ChangeResult::NoChange } else { ChangeResult::Change(diffs,ballot_changes)}
    } else {ChangeResult::NotEnoughVotesAvailable}
}

pub struct FoundChange<Tally> {
    pub vote_changes : VoteChanges<Tally>,
    pub deltas : DeltasInCandidateLists,
    pub changes : BallotChanges<Tally>
}
pub fn optimise<R:PreferenceDistributionRules>(vote_changes:&VoteChanges<R::Tally>,election_data:&ElectionData,retroscope:&Retroscope,options:&ChooseVotesOptions,verbose:bool) -> Option<FoundChange<R::Tally>> {
    optimise_work::<R>(vote_changes,election_data,retroscope,options,verbose,0)
}
pub fn optimise_work<R:PreferenceDistributionRules>(vote_changes:&VoteChanges<R::Tally>,election_data:&ElectionData,retroscope:&Retroscope,options:&ChooseVotesOptions,verbose:bool,tried_already:usize) -> Option<FoundChange<R::Tally>> {
    match simple_test::<R>(vote_changes,election_data,retroscope,options) {
        ChangeResult::NotEnoughVotesAvailable => { // could try reducing.
            if verbose { println!("Not enough votes available - looking for {} from {}",vote_changes.changes.iter().map(|c|c.vote_value.clone()).sum::<R::Tally>(),vote_changes.changes.first().and_then(|c|c.from).map(|c|election_data.metadata.candidate(c).name.as_str()).unwrap_or(""));}
            None // TODO try reducing
        }
        ChangeResult::NoChange => { // could try increasing
            if tried_already==0 {
                if verbose { println!("No change - trying doubling everything"); }
                let mut new_changes = vote_changes.clone();
                for c in &mut new_changes.changes { c.vote_value+=c.vote_value.clone(); }
                optimise_work::<R>(&new_changes,election_data,retroscope,options,verbose,tried_already+1)
            } else {
                if verbose { println!("No change - giving up"); }
                None
            }
        }
        ChangeResult::Change(deltas,changes) => {
            // guaranteed something, try to improve!
            let mut had_change = true;
            let mut best_so_far = FoundChange{ vote_changes:vote_changes.clone(), deltas,changes };
            let mut opt_vote_changes = vote_changes.clone();
            while had_change {
                had_change=false;
                for i in 0..vote_changes.changes.len() {
                    let current_tally = opt_vote_changes.changes[i].vote_value.ceil();
                    let try_value = |new_count:usize| {
                        if verbose { println!("Trying change to {}",new_count); }
                        simple_test::<R>(&opt_vote_changes.change_single_value(i,new_count),election_data,retroscope,options)
                    };
                    if let Some(search_res) = binary_search(try_value,0,current_tally) {
                        if search_res.n<current_tally { // had an improvement!
                            if verbose { println!("Improved change from {} to {}",current_tally,search_res.n); }
                            opt_vote_changes.changes[i].vote_value=search_res.n.into();
                            if vote_changes.changes.len()>1 { had_change=true; }
                            if best_so_far.changes.n>=search_res.changes.n { // almost always the case if votes are reduced
                                best_so_far = FoundChange{ vote_changes:opt_vote_changes.clone(), deltas:search_res.deltas,changes:search_res.changes };
                            }
                        }
                    }
                }
            }
            best_so_far.vote_changes.changes.retain(|c|!c.vote_value.is_zero()); // get rid of zero value changes.
            Some(best_so_far)
        }
    }
}

struct BinarySearchSuccess<Tally> {
    n : usize, // the best value that works.
    deltas : DeltasInCandidateLists,
    changes : BallotChanges<Tally>
}

/// Find the smallest value v such that low<=v<=high and f(v)=Change()
fn binary_search<F,Tally>(f:F,mut low:usize,mut high:usize) -> Option<BinarySearchSuccess<Tally>>
  where F: Fn(usize) -> ChangeResult<Tally>
{
    let mut last_good : Option<BinarySearchSuccess<Tally>> = None;
    while low<high || low==high && (last_good.is_none() || last_good.as_ref().unwrap().n!=low) {
        let mid = (low+high)/2;
        match f(mid) {
            ChangeResult::NotEnoughVotesAvailable => { if mid==0 { return None; } else { high=mid-1;} } // has to be smaller.
            ChangeResult::NoChange => { low=mid+1 } // has to be bigger
            ChangeResult::Change(deltas, changes) => { high=mid; last_good=Some(BinarySearchSuccess{n:mid,deltas,changes}) }
        }
    }
    last_good
}