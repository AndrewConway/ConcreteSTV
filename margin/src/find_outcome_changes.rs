// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::HashSet;
use stv::ballot_metadata::{CandidateIndex, ElectionMetadata};
use stv::ballot_paper::{ATL, BTL};
use stv::compare_transcripts::{compare_transcripts, DifferenceBetweenTranscripts};
use stv::distribution_of_preferences_transcript::{CountIndex, ReasonForCount, Transcript};
use stv::election_data::ElectionData;
use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules};
use crate::retroscope::Retroscope;

pub fn find_outcome_changes <Rules>(original_data:&ElectionData)
where Rules : PreferenceDistributionRules<Tally=usize> {

    let transcript = distribute_preferences::<Rules>(&original_data, original_data.metadata.vacancies.unwrap(), &original_data.metadata.excluded.iter().cloned().collect(), &original_data.metadata.tie_resolutions, false);
    let mut not_continuing = HashSet::new();

    let mut min_manipulation = Manipulation { removed_winner: CandidateIndex(0), otherwise_eliminated: CandidateIndex(0), size:original_data.num_votes()}; // Initialise with total votes, guaranteed to be greater than any difference.
    let mut retroscope = Retroscope::new(&original_data, &original_data.metadata.excluded);
    let mut sorted_continuing_candidates:Vec<CandidateIndex> = retroscope.continuing.iter().cloned().collect();
    for countnumber in 0 .. transcript.counts.len() -1 {
        let count = &transcript.counts[countnumber];
        retroscope.apply(CountIndex(countnumber),count );
        // In NSW there are multiple counts for one 'action', so most counts do not have a decision
        // We only need to try changing the decision on those counts for which some decision (e.g.
        // eliminate or seat someone) can be made.
        if !count.reason_completed {
            continue;
        }
        sorted_continuing_candidates = retroscope.continuing.iter().cloned().collect();
        sorted_continuing_candidates.sort_by_key(| c | count.status.tallies.candidate[c.0]);
        let next_count =  &transcript.counts[countnumber+1];
        match if countnumber < transcript.counts.len()-1 { Some(&transcript.counts[countnumber+1].reason) } else { None } {
                // If this is a count that redistributes the votes of a candidate eliminated in the previous
                // count, see if we can get someone else eliminated instead.
                Some(ReasonForCount::Elimination(eliminated_candidates)) if eliminated_candidates.len()==1 => {
                let eliminated_candidate = eliminated_candidates[0];
                let tally_excluded_candidate = count.status.tallies.candidate[eliminated_candidate.0];
                // TODO: Add break when we've found at least one value for each kind of change (manipulation and addition)
                for &candidate in &sorted_continuing_candidates {
                    let vote_difference = count.status.tallies.candidate[candidate.0] - tally_excluded_candidate;
                    let votes_to_change = vote_difference / 2 + vote_difference % 2; // Round up to nearest int if odd
                    //let possible_manipulation = try_swapping_two_candidates::<Rules>(eliminated_candidate, continuing_elected_candidates[lowest_winner_index], votes_to_change, &original_data, &transcript);
                    let possible_manipulation = try_swapping_two_candidates::<Rules>(eliminated_candidate, candidate, votes_to_change, &original_data, &transcript);
                    // TODO: think carefully about whether the size/value distinction is properly captured.
                    match possible_manipulation {
                        None => {}
                        Some(m) => {
                            if m.size < min_manipulation.size {
                                min_manipulation = m;
                            }
                        }
                    }
                }
            }
                // If someone got elected in this count, either because they got a quota or because the
                // number of continuing candidates was just enough to fill the seats, see if we can get
                // someone else elected instead.
                // At the moment, this simply tries swapping with the highest continuing candidate who
                // is not an official winner.
            _ => {
                let sorted_continuing_non_winners = sorted_continuing_candidates.iter().filter(| c | !transcript.elected.contains(c)).cloned().collect::<Vec<_>>();
                let just_elected_candidates = &count.elected;
                if just_elected_candidates.len() > 0  && sorted_continuing_non_winners.len() > 0 {
                    let highest_non_winner= sorted_continuing_non_winners[sorted_continuing_non_winners.len()-1];
                    let elected_candidate_tallies = just_elected_candidates.iter().map(|c| count.status.tallies.candidate[c.who.0]).collect::<Vec<_>>();
                    let lowest_winner_tally = elected_candidate_tallies.iter().cloned().min().unwrap();
                    let lowest_winner_index = elected_candidate_tallies.iter().position(|&t| lowest_winner_tally == t).unwrap();
                    let lowest_winner = just_elected_candidates[lowest_winner_index].who;

                    let vote_difference = count.status.tallies.candidate[lowest_winner.0] - count.status.tallies.candidate[highest_non_winner.0];

                    let votes_to_change = vote_difference / 2 + vote_difference % 2; // Round up to nearest int if odd
                    let possible_manipulation = try_swapping_two_candidates::<Rules>(highest_non_winner, lowest_winner, votes_to_change, &original_data, &transcript);
                    // TODO: think carefully about whether the size/value distinction is properly captured.
                    match possible_manipulation {
                        None => {}
                        Some(m) => {
                            if m.size < min_manipulation.size {
                                min_manipulation = m;
                            }
                        }
                    }
                }
            }
        }
        for c in &count.not_continuing { not_continuing.insert(*c); }

    }
    println!("Electorate: {}. {} total votes. Min manipulation: size {}", original_data.metadata.name.electorate, original_data.num_votes(),  min_manipulation.size)
}

// TODO: when this is updated to do proper calculations about current vote weight, it will need to
// return the number of ballots actually changed to produce a certain value. Probably best to
// simply add that to the Manipulation data structure.
fn try_swapping_two_candidates<Rules>(eliminated_candidate:CandidateIndex, to_be_reduced: CandidateIndex, votes_to_change: usize, original_data: &ElectionData, original_transcript: &Transcript<usize>) -> Option<Manipulation>
where Rules : PreferenceDistributionRules<Tally=usize> {
    let mut data = original_data.clone();
    // println!("Electorate {} lowest winner {:?}. Votes to change {}", data.metadata.name.electorate, min_manipulation.removed_winner, min_manipulation.size );
    // Add enough votes to bump up the would-be eliminated candidate
    data.btl.push(BTL {
        candidates: vec![eliminated_candidate],
        n: votes_to_change
    });

    // Remove votes for the lowest winner
    let (new_btls, num_to_go) = remove_btls(to_be_reduced, votes_to_change, data.btl);
    data.btl = new_btls;

    // If the lowest (would-be) winner is first on their party's ticket, we can remove ATL
    // votes for that party/group
    let (new_atls, num_to_go) = if_top_remove_atls(to_be_reduced, num_to_go, data.atl, &data.metadata);
    data.atl = new_atls;

    let altered_transcript = distribute_preferences::<Rules>(&data, data.metadata.vacancies.unwrap(), &data.metadata.excluded.iter().cloned().collect(), &data.metadata.tie_resolutions, false);

    let transcript_comparison = compare_transcripts(original_transcript, &altered_transcript);

    match &transcript_comparison {
        DifferenceBetweenTranscripts::DifferentCandidatesElected(_) => {
            println!("Electorate {}: min manipulation {}. Result: {:?}", data.metadata.name.electorate, votes_to_change, transcript_comparison);
            return Some(Manipulation{ size: votes_to_change, otherwise_eliminated: eliminated_candidate, removed_winner: to_be_reduced }); }
        _ => {}
    }
    None
}

// If the candidate to be reduced is at the top of a party ticket, we can reduce their tally by removing
// ATL votes for that party/group.
fn if_top_remove_atls(candidate_to_be_reduced: CandidateIndex, num_to_go: usize, old_atls: Vec<ATL>, metadata: &ElectionMetadata) -> (Vec<ATL>, usize) {
    let mut editable_num_to_go = num_to_go;
    let mut new_atls = vec![];

    if metadata.candidate(candidate_to_be_reduced).position == Some(1) {
        if let Some(party) = metadata.candidate(candidate_to_be_reduced).party {
            for v in old_atls {
                if v.parties[0] == party {
                    if v.n <= editable_num_to_go {
                        editable_num_to_go -= v.n;
                    } else {
                        new_atls.push(ATL {
                            parties: v.parties,
                            n: v.n - editable_num_to_go
                        })
                    }
                } else {
                    new_atls.push(v);
                }
            }
            (new_atls, editable_num_to_go)
        } else {
            (old_atls.clone(), num_to_go)
        }
    } else {
        (old_atls.clone(), num_to_go)
    }
}

fn remove_btls(candidate_to_reduce: CandidateIndex, votes_to_change: usize, old_btls: Vec<BTL>) -> (Vec<BTL>, usize) {
    let mut new_btls = vec![];
    let mut num_to_go = votes_to_change;
    for v in old_btls {
        if v.candidates[0] == candidate_to_reduce {
            if v.n <= num_to_go {
                num_to_go -= v.n;
            } else {
                new_btls.push(BTL {
                    candidates: v.candidates.clone(),
                    n: v.n - num_to_go
                });
                num_to_go = 0;
            }
        } else {
            new_btls.push(v);
        }
    }
    (new_btls, num_to_go)
}

/// A minimum Manipuation, including both the removed would-be winner and the otherwise-eliminated candidate who has to be raised to remove them
#[derive(Clone,Debug)]
pub struct Manipulation {
// Candidate who would have won, without this Manipulation
    pub removed_winner: CandidateIndex,
    // Candidate whose tally needed to increase
    pub otherwise_eliminated: CandidateIndex,
    // number of votes to shift
    pub size: usize
}
