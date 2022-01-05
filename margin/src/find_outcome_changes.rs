// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::HashSet;
use stv::ballot_metadata::{CandidateIndex};
use stv::distribution_of_preferences_transcript::{CountIndex, ReasonForCount, SingleCount};
use stv::election_data::ElectionData;
use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules};
use crate::choose_votes::ChooseVotesOptions;
use crate::evaluate_and_optimize_vote_changes::optimise;
use crate::record_changes::ElectionChanges;
use crate::retroscope::Retroscope;
use crate::vote_changes::{VoteChange, VoteChanges};

pub fn find_outcome_changes <Rules>(original_data:&ElectionData, vote_choice_options:&ChooseVotesOptions) -> ElectionChanges<Rules::Tally>
where Rules : PreferenceDistributionRules<Tally=usize> {

    let transcript = distribute_preferences::<Rules>(&original_data, original_data.metadata.vacancies.unwrap(), &original_data.metadata.excluded.iter().cloned().collect(), &original_data.metadata.tie_resolutions, false);
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
        sorted_continuing_candidates.sort_by_key(| c | count.status.tallies.candidate[c.0]);
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
                for &candidate in &sorted_continuing_candidates {
                    if candidate!=eliminated_candidate {
                        let vote_change = compute_vote_change::<Rules>(eliminated_candidate, candidate, count);
                        let vote_changes = VoteChanges{ changes: vec![vote_change] };
                        if let Some(possible_manipulation) = optimise::<Rules>(&vote_changes, &original_data, &retroscope, vote_choice_options) {
                            change_recorder.add(possible_manipulation);
                        }
                    }
                }
                // Addition-only option.
                let vote_addition = compute_vote_addition::<Rules>(eliminated_candidate, sorted_continuing_candidates[1], count);
                let vote_additions = VoteChanges{ changes: vec![vote_addition] };
                if let Some(possible_addition) = optimise::<Rules>(&vote_additions, &original_data, &retroscope, vote_choice_options) {
                    change_recorder.add(possible_addition);
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
                    let elected_candidate_tallies = just_elected_candidates.iter().map(|c| count.status.tallies.candidate[c.who.0]).collect::<Vec<_>>();
                    let lowest_winner_tally = elected_candidate_tallies.iter().cloned().min().unwrap();
                    let lowest_winner_index = elected_candidate_tallies.iter().position(|&t| lowest_winner_tally == t).unwrap();
                    let lowest_winner = just_elected_candidates[lowest_winner_index].who;
                    let vote_change = compute_vote_change::<Rules>(highest_non_winner, lowest_winner, &count);

                    // Try shifting votes from the lowest winner to the highest non-winner
                    let vote_changes = VoteChanges{ changes: vec![vote_change] };
                    if let Some(possible_manipulation) = optimise::<Rules>(&vote_changes, &original_data, &retroscope, vote_choice_options) {
                        change_recorder.add(possible_manipulation);
                    }

                    // Addition-only
                    let vote_addition = compute_vote_addition::<Rules>(highest_non_winner, lowest_winner, &count);
                    let vote_additions = VoteChanges{ changes: vec![vote_addition] };
                    if let Some(possible_addition) = optimise::<Rules>(&vote_additions, &original_data, &retroscope, vote_choice_options) {
                        change_recorder.add(possible_addition);
                    }
                }
            }
        }
        for c in &count.not_continuing { not_continuing.insert(*c); }

    }
    change_recorder.sort();
    println!("Electorate: {}. {} total votes. Min manipulations: size {:?}", original_data.metadata.name.electorate, original_data.num_votes(),  change_recorder.changes.iter().map(| c | c.ballots.n).collect::<Vec<_>>());
    change_recorder
}

fn compute_vote_addition<Rules:PreferenceDistributionRules<Tally=usize>>(to_candidate: CandidateIndex, next_largest: CandidateIndex, count: &SingleCount<usize>) -> VoteChange<Rules::Tally> {
    let vote_difference = count.status.tallies.candidate[next_largest.0] - count.status.tallies.candidate[to_candidate.0];
    println!("Vote difference: {}", vote_difference);
    return VoteChange {
        vote_value: vote_difference+1,
        from: None,
        to: Some(to_candidate)
    }
}

fn compute_vote_change<Rules:PreferenceDistributionRules<Tally=usize>>(to_candidate: CandidateIndex, from_candidate: CandidateIndex, count: &SingleCount<usize>) -> VoteChange<Rules::Tally> {
    let tally_to_candidate = count.status.tallies.candidate[to_candidate.0];
    let tally_from_candidate = count.status.tallies.candidate[from_candidate.0];

    println!("Tally for from candidate {} is {}, for to candidate {} is {}",from_candidate,tally_from_candidate,to_candidate,tally_to_candidate);
    let vote_difference = tally_from_candidate  - tally_to_candidate;
    let votes_to_change = vote_difference / 2 + 1; // Want diff of 11 to produce 6, a diff of 12 to produce 7.
    return VoteChange {
        vote_value: votes_to_change,
        from: Some(from_candidate),
        to: Some(to_candidate)
    }
}