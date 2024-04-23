// Copyright 2021-2024 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::HashSet;
use stv::preference_distribution::RoundUpToUsize;
use num_traits::Zero;
use stv::ballot_metadata::{CandidateIndex};
use stv::ballot_pile::BallotPaperCount;
use stv::distribution_of_preferences_transcript::{CountIndex, ReasonForCount, SingleCount};
use stv::election_data::ElectionData;
use stv::preference_distribution::PreferenceDistributionRules;
use stv::random_util::Randomness;
use crate::choose_votes::{ChooseVotes, ChooseVotesOptions};
use crate::evaluate_and_optimize_vote_changes::optimise;
use crate::record_changes::ElectionChanges;
use crate::retroscope::Retroscope;
use crate::vote_changes::{VoteChange, VoteChanges};

pub fn find_outcome_changes <Rules>(original_data:&ElectionData, vote_choice_options:&ChooseVotesOptions,verbose:bool) -> ElectionChanges<Rules::Tally>
where Rules : PreferenceDistributionRules {

    let transcript = original_data.distribute_preferences::<Rules>(&mut Randomness::ReverseDonkeyVote);
    let mut not_continuing = HashSet::new();

    let mut retroscope = Retroscope::new(&original_data, &original_data.metadata.excluded);
    let mut change_recorder = ElectionChanges::new(original_data,&vote_choice_options.ballot_types_considered_unverifiable);
    for countnumber in 0 .. transcript.counts.len() {
        let count = &transcript.counts[countnumber];
        retroscope.apply(CountIndex(countnumber),count );
        // In NSW there are multiple counts for one 'action', so most counts do not have a decision
        // We only need to try changing the decision on those counts for which some decision (e.g.
        // eliminate or seat someone) can be made. This is superfluous for non-NSW elections.
        if !count.reason_completed {
            continue;
        }
        let mut sorted_continuing_candidates:Vec<CandidateIndex> = retroscope.continuing.iter().cloned().collect();
        sorted_continuing_candidates.sort_by_key(| c | count.status.tallies.candidate[c.0].clone());
        match if countnumber < transcript.counts.len()-1 { Some(&transcript.counts[countnumber+1].reason) } else { None } {
            // If this is a count that redistributes the votes of a candidate eliminated in the previous
            // count, see if we can get someone else eliminated instead, first by iterating through
            // all other continuing candidates to see if we can shift votes from them to the otherwise-eliminated
            // candidate.
            // Then try an addition-only option, where we simply try adding enough votes to the would-be
            // eliminated candidate in order to raise it above the next-lowest.
            Some(ReasonForCount::Elimination(eliminated_candidates)) if eliminated_candidates.len()==1 => {
                let eliminated_candidate = eliminated_candidates[0];
                // TODO: Add break when we've found at least one value for each kind of change (manipulation and addition)
                for (index_of_target_in_sorted_list,&candidate) in sorted_continuing_candidates.iter().enumerate() {
                    if candidate!=eliminated_candidate {
                        let vote_change = compute_vote_change::<Rules>(eliminated_candidate, candidate, count,verbose);
                        let vote_changes = VoteChanges{ changes: vec![vote_change] };
                        if let Some(possible_manipulation) = optimise::<Rules>(&vote_changes, &original_data, &retroscope, vote_choice_options,verbose) {
                            change_recorder.add(possible_manipulation,verbose);
                        }
                        // try leveling
                        if let Some(leveling) = compute_vote_change_leveling::<Rules>(index_of_target_in_sorted_list,true,count,&sorted_continuing_candidates,&original_data, &retroscope, vote_choice_options,true,verbose) {
                            if verbose { println!("Found a levelling to try {}",leveling); }
                            if let Some(possible_manipulation) = optimise::<Rules>(&leveling, &original_data, &retroscope, vote_choice_options,verbose) {
                                change_recorder.add(possible_manipulation,verbose);
                                // that worked! Try related things.
                                if let Some(leveling) = compute_vote_change_leveling::<Rules>(index_of_target_in_sorted_list,true,count,&sorted_continuing_candidates,&original_data, &retroscope, vote_choice_options,false,verbose) {
                                    if verbose { println!("Found a related levelling to try {}",leveling); }
                                    if let Some(possible_manipulation) = optimise::<Rules>(&leveling, &original_data, &retroscope, vote_choice_options,verbose) {
                                        change_recorder.add(possible_manipulation,verbose);
                                    }
                                }
                                if let Some(leveling) = compute_vote_change_leveling::<Rules>(index_of_target_in_sorted_list,false,count,&sorted_continuing_candidates,&original_data, &retroscope, vote_choice_options,false,verbose) {
                                    if verbose { println!("Found a related levelling to try {}",leveling); }
                                    if let Some(possible_manipulation) = optimise::<Rules>(&leveling, &original_data, &retroscope, vote_choice_options,verbose) {
                                        change_recorder.add(possible_manipulation,verbose);
                                    }
                                }
                                if let Some(leveling) = compute_vote_change_leveling::<Rules>(index_of_target_in_sorted_list,false,count,&sorted_continuing_candidates,&original_data, &retroscope, vote_choice_options,true,verbose) {
                                    if verbose { println!("Found a related levelling to try {}",leveling); }
                                    if let Some(possible_manipulation) = optimise::<Rules>(&leveling, &original_data, &retroscope, vote_choice_options,verbose) {
                                        change_recorder.add(possible_manipulation,verbose);
                                    }
                                }
                            }
                        }
                    }
                }
                // Addition-only option.
                let vote_addition = compute_vote_addition::<Rules>(eliminated_candidate, sorted_continuing_candidates[1], count,verbose);
                let vote_additions = VoteChanges{ changes: vec![vote_addition] };
                if let Some(possible_addition) = optimise::<Rules>(&vote_additions, &original_data, &retroscope, vote_choice_options,verbose) {
                    change_recorder.add(possible_addition,verbose);
                }
            }

            // If this is not an elimination count, probably someone got elected in this count,
            // either because they got a quota or because the
            // number of continuing candidates was just enough to fill the seats. See if we can get
            // someone else elected instead.
            // At the moment, this simply tries swapping with the highest continuing candidate who
            // is not an official winner.
            // It then tries the addition-only option, in which we try adding votes to the highest
            // non-winner until it exceeds the official winner.

            _ => {
                let sorted_continuing_non_winners = sorted_continuing_candidates.iter().filter(| c | !transcript.elected.contains(c)).cloned().collect::<Vec<_>>();
                let just_elected_candidates = &count.elected;
                if just_elected_candidates.len() > 0  && sorted_continuing_non_winners.len() > 0 {
                    let highest_non_winner= sorted_continuing_non_winners[sorted_continuing_non_winners.len()-1];
                    let elected_candidate_tallies = just_elected_candidates.iter().map(|c| count.status.tallies.candidate[c.who.0].clone()).collect::<Vec<_>>();
                    let lowest_winner_tally = elected_candidate_tallies.iter().cloned().min().unwrap();
                    let lowest_winner_index = elected_candidate_tallies.iter().position(|t| lowest_winner_tally == *t).unwrap();
                    let lowest_winner = just_elected_candidates[lowest_winner_index].who;
                    let vote_change = compute_vote_change::<Rules>(highest_non_winner, lowest_winner, &count,verbose);

                    // Try shifting votes from the lowest winner to the highest non-winner
                    let vote_changes = VoteChanges{ changes: vec![vote_change] };
                    if let Some(possible_manipulation) = optimise::<Rules>(&vote_changes, &original_data, &retroscope, vote_choice_options,verbose) {
                        change_recorder.add(possible_manipulation,verbose);
                    }

                    // Addition-only
                    let vote_addition = compute_vote_addition::<Rules>(highest_non_winner, lowest_winner, &count,verbose);
                    let vote_additions = VoteChanges{ changes: vec![vote_addition] };
                    if let Some(possible_addition) = optimise::<Rules>(&vote_additions, &original_data, &retroscope, vote_choice_options,verbose) {
                        change_recorder.add(possible_addition,verbose);
                    }
                }
            }
        }
        for c in &count.not_continuing { not_continuing.insert(*c); }

    }
    change_recorder.sort();
    if verbose { println!("Electorate: {}. {} total votes. Min manipulations: size {:?}", original_data.metadata.name.electorate, original_data.num_votes(),  change_recorder.changes.iter().map(| c | c.ballots.n).collect::<Vec<_>>()); }
    change_recorder
}

/// Find an addition that makes to_candidate have a higher number of votes that next_largest.
/// Used to make the current candidate be eliminated.
fn compute_vote_addition<Rules:PreferenceDistributionRules>(to_candidate: CandidateIndex, next_largest: CandidateIndex, count: &SingleCount<Rules::Tally>,verbose:bool) -> VoteChange<Rules::Tally> {
    let vote_difference = count.status.tallies.candidate[next_largest.0].clone() - count.status.tallies.candidate[to_candidate.0].clone();
    if verbose { println!("Vote difference: {}", vote_difference); }
    return VoteChange {
        vote_value: vote_difference+Rules::Tally::from(BallotPaperCount(1)), // could probably be improved to minimum increment above self.
        from: None,
        to: Some(to_candidate)
    }
}

/// Find a change that takes directly from one candidate and gives directly to another candidate, typically to make to_candidate be eliminated.
fn compute_vote_change<Rules:PreferenceDistributionRules>(to_candidate: CandidateIndex, from_candidate: CandidateIndex, count: &SingleCount<Rules::Tally>,verbose:bool) -> VoteChange<Rules::Tally> {
    let tally_to_candidate = count.status.tallies.candidate[to_candidate.0].clone();
    let tally_from_candidate = count.status.tallies.candidate[from_candidate.0].clone();

    if verbose { println!("Tally for from candidate {} is {}, for to candidate {} is {}",from_candidate,tally_from_candidate,to_candidate,tally_to_candidate); }
    let vote_difference = tally_from_candidate  - tally_to_candidate;
    let votes_to_change = Rules::Tally::from(BallotPaperCount(vote_difference.ceil() / 2 + 1)); // Want diff of 11 to produce 6, a diff of 12 to produce 7.
    return VoteChange {
        vote_value: votes_to_change,
        from: Some(from_candidate),
        to: Some(to_candidate)
    }
}

/// Find a change that tries to get one target candidate excluded by making them lower than all other candidates.
/// Takes votes from the target (if may_take_votes_from_candidate) and possibly higher candidates.
/// Gives votes to the candidates with a tally equal or less than the target until the target is one less than all lower.
///
/// Method:
///
/// Will aim for a level base_level for the target. Everyone else should be > base_level.
///
fn compute_vote_change_leveling<Rules:PreferenceDistributionRules>(index_of_target_in_sorted_list:usize,may_take_votes_from_target:bool, count: &SingleCount<Rules::Tally>,sorted_continuing_candidates:&[CandidateIndex],election_data:&ElectionData,retroscope:&Retroscope,options:&ChooseVotesOptions,reverse_secondary_targets:bool,verbose:bool) -> Option<VoteChanges<Rules::Tally>> {
    let can_use_atl = |c:CandidateIndex|retroscope.is_highest_continuing_member_party_ticket(c,&election_data.metadata);
    let target: CandidateIndex=sorted_continuing_candidates[index_of_target_in_sorted_list];
    let current_target_tally = count.status.tallies.candidate[target.0].clone();
    let target_chooser = if may_take_votes_from_target { retroscope.get_chooser(target,election_data,options) } else { ChooseVotes::zero(election_data) };
    let max_can_take_from_target : Rules::Tally = target_chooser.votes_available_total::<Rules>();
    let base_level = if may_take_votes_from_target { // level that the target is aimed at.
        let mut sub_targets_to_raise = 0;
        let mut sum_of_target_plus_subtargets_plus_one = current_target_tally.clone()+Rules::Tally::from(BallotPaperCount(1));
        let mut currently_being_considered_level = current_target_tally.clone();
        while sub_targets_to_raise < index_of_target_in_sorted_list {
            let subtarget_tally = count.status.tallies.candidate[sorted_continuing_candidates[sub_targets_to_raise].0].clone();
            if currently_being_considered_level<subtarget_tally { break; } // the currently considered level is sufficient.
            sum_of_target_plus_subtargets_plus_one+=subtarget_tally;
            sub_targets_to_raise+=1;
            let consider_level : <Rules as PreferenceDistributionRules>::Tally= BallotPaperCount(sum_of_target_plus_subtargets_plus_one.ceil()/(sub_targets_to_raise+1)).into(); // still need to subtract 1, but may be < 1 which would cause underflow.
            let consider_level = if consider_level<=Rules::Tally::from(BallotPaperCount(1)) { Rules::Tally::zero() } else { consider_level-Rules::Tally::from(BallotPaperCount(1))};
            currently_being_considered_level = consider_level.max(current_target_tally.clone()-max_can_take_from_target.clone())
        }
        currently_being_considered_level
    } else { current_target_tally.clone() };
    if verbose { println!("{}Target candidate {} with tally {} is to be reduced to {}. Can take {} from target.",if election_data.metadata.results.as_ref().unwrap().contains(&target) {"ELECTED "} else {""},target,current_target_tally,base_level,max_can_take_from_target); }
    // need to raise everything up to target.
    let mut res =VoteChanges { changes: vec![] };
    let mut source = PolyFromSource{
        from_sources: vec![target],
        available_to_take_from_sources: vec![target_chooser],
        index_currently_taking_from_btl_only: 0,
        index_currently_taking_from_atl_ok: 0
    };
    // find other sources.
    let secondary_targets : Vec<CandidateIndex> = if reverse_secondary_targets { sorted_continuing_candidates[index_of_target_in_sorted_list+1..].iter().rev().take_while(|&&c|c!=target).cloned().collect() } else {sorted_continuing_candidates[index_of_target_in_sorted_list+1..].iter().take_while(|&&c|c!=target).cloned().collect()};
    for secondary_target in secondary_targets {
        source.from_sources.push(secondary_target);
        source.available_to_take_from_sources.push(retroscope.get_chooser(secondary_target,election_data,options));
    }
    let fudge_factor = 1;
    let margin_above_base = Rules::Tally::from(BallotPaperCount(1+fudge_factor)); // want to go to above base tally. Go 2 higher instead of 1 just to allow for wierd things with rounding. 1 fudge factor.
    for atl_ok in [false,true] {
        for &c in sorted_continuing_candidates {
            if c!=target && atl_ok == can_use_atl(c) { // do those not in a party first as they can't take ATL votes
                let tally = count.status.tallies.candidate[c.0].clone();
                if tally<= base_level {
                    let increment = base_level.clone()-tally.clone()+margin_above_base.clone();
                    if !source.give_to_candidate::<Rules>(increment,c,&mut res,atl_ok) { return None; }
                }
            }
        }
    }
    // it is possible that not enough votes have been taken from target, if the votes available were largely ATL and the people being given to were largely BTL.
    let after_mods_tally_for_target = current_target_tally+source.available_to_take_from_sources[0].votes_available_total::<Rules>()-max_can_take_from_target;
    if after_mods_tally_for_target>base_level {
        if verbose { println!("Could not take enough from target. Taking more."); }
        if let Some(&recipient) = sorted_continuing_candidates[index_of_target_in_sorted_list+1..].iter().rev().find(|&&c|can_use_atl(c)) {
            res.transfer(after_mods_tally_for_target-base_level,target,recipient);
            res.changes.reverse(); // put at start, so binary search reduces it first.
        }
        // could do an "else return None" but there is a chance the current thing will still do something useful.
    }
    Some(res)
}

struct PolyFromSource<'a> {
    from_sources : Vec<CandidateIndex>,
    available_to_take_from_sources : Vec<ChooseVotes<'a>>,
    index_currently_taking_from_btl_only : usize,
    index_currently_taking_from_atl_ok : usize,
}

impl <'a> PolyFromSource<'a> {
    fn available_to_take_atl_ok<Rules:PreferenceDistributionRules>(&mut self) -> Option<Rules::Tally> {
        while self.index_currently_taking_from_atl_ok<self.from_sources.len() {
            let available : Rules::Tally = self.available_to_take_from_sources[self.index_currently_taking_from_atl_ok].votes_available_total::<Rules>();
            if available == Rules::Tally::zero() { self.index_currently_taking_from_atl_ok+=1; }
            else { return Some(available); }
        }
        None
    }
    fn available_to_take_btl_only<Rules:PreferenceDistributionRules>(&mut self) -> Option<Rules::Tally> {
        while self.index_currently_taking_from_btl_only<self.from_sources.len() {
            let available : Rules::Tally = self.available_to_take_from_sources[self.index_currently_taking_from_btl_only].votes_available_btl::<Rules>();
            if available == Rules::Tally::zero() { self.index_currently_taking_from_btl_only+=1; }
            else { return Some(available); }
        }
        None
    }
    /// Try to give a certain amount of votes to a given recipient, return true iff success.
    fn give_to_candidate<Rules:PreferenceDistributionRules>(&mut self, amount:Rules::Tally, recipient:CandidateIndex, changes:&mut VoteChanges<Rules::Tally>,may_be_atl:bool) -> bool {
        let mut togo = amount;
        while togo>Rules::Tally::zero() {
            if let Some(available) = if may_be_atl { self.available_to_take_atl_ok::<Rules>() } else { self.available_to_take_btl_only::<Rules>() } {
                let parcel = togo.clone().min(available);
                let index_currently_taking_from = if may_be_atl {self.index_currently_taking_from_atl_ok} else { self.index_currently_taking_from_btl_only};
                changes.transfer(parcel.clone(),self.from_sources[index_currently_taking_from],recipient);
                if self.available_to_take_from_sources[index_currently_taking_from].get_votes::<Rules>(parcel.clone(),may_be_atl).is_none() { return false; } // unlikely
                togo-=parcel.clone();
            } else { return false; }
        }
        true
    }
}