// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::HashSet;
use stv::ballot_metadata::CandidateIndex;
use stv::ballot_paper::{ATL, BTL};
use stv::compare_transcripts::compare_transcripts;
use stv::distribution_of_preferences_transcript::ReasonForCount;
use stv::election_data::ElectionData;
use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules};

pub fn find_outcome_changes <Rules>(original_data:&ElectionData)
where Rules : PreferenceDistributionRules<Tally=usize> {

    let transcript = distribute_preferences::<Rules>(&original_data, original_data.metadata.vacancies.unwrap(), &original_data.metadata.excluded.iter().cloned().collect(), &original_data.metadata.tie_resolutions, false);
    let mut not_continuing = HashSet::new();

    // minimum Manipulation. Second element is whose votes should be added (would-be eliminated); third element is whose should be removed (would-be winner).
    let mut min_manipulation = Manipulation { removed_winner: CandidateIndex(0), otherwise_eliminated: CandidateIndex(0), size:original_data.num_votes()}; // Initialise with total votes, guaranteed to be greater than any difference.
    for count in &transcript.counts {
        match &count.reason {
            ReasonForCount::FirstPreferenceCount => {}
            ReasonForCount::ExcessDistribution(_) => {}
            ReasonForCount::Elimination(eliminated_candidates) if eliminated_candidates.len()==1 => {
                let eliminated_candidate = eliminated_candidates[0];
                let tally_excluded_candidate = count.status.tallies.candidate[eliminated_candidate.0];
                let continuing_elected_candidates = transcript.elected.iter().filter(|c|!not_continuing.contains(*c)).cloned().collect::<Vec<_>>();
                if continuing_elected_candidates.len() > 0 {
                    let elected_candidate_tallies  = continuing_elected_candidates.iter().map(|c| count.status.tallies.candidate[c.0]).collect::<Vec<_>>();
                    let lowest_winner_tally = elected_candidate_tallies.iter().cloned().min().unwrap();
                    let lowest_winner_index = elected_candidate_tallies.iter().position(|&t| lowest_winner_tally == t ).unwrap();
                    let lowest_winner = continuing_elected_candidates[lowest_winner_index];
                    let vote_difference = lowest_winner_tally - tally_excluded_candidate;
                    let votes_to_change = vote_difference / 2 + vote_difference % 2; // Round up to nearest int if odd
                    if votes_to_change < min_manipulation.size {
                        min_manipulation = Manipulation {size: votes_to_change, otherwise_eliminated: eliminated_candidate, removed_winner: continuing_elected_candidates[lowest_winner_index]};
                    }
                }
            }
            ReasonForCount::Elimination(_) => {}
        }
        for c in &count.not_continuing { not_continuing.insert(*c); }

        let mut data = original_data.clone();
        // println!("Electorate {} lowest winner {:?}. Votes to change {}", data.metadata.name.electorate, min_manipulation.removed_winner, min_manipulation.size );
        // Add enough votes to bump up the would-be eliminated candidate
        data.btl.push(BTL{
            candidates: vec![min_manipulation.otherwise_eliminated],
            n: min_manipulation.size
        });

        // Remove votes for the lowest winner
        
        let mut new_btls = vec![] ;
        let mut num_to_go = min_manipulation.size;
        for v in data.btl {
            if v.candidates[0] == min_manipulation.removed_winner {
                if v.n <= num_to_go {
                    num_to_go -= v.n;
                } else {
                    new_btls.push(BTL{
                        candidates: v.candidates,
                        n: v.n - num_to_go
                    });
                    num_to_go = 0;
                }
            } else {
                new_btls.push(v);
            }
        }
        data.btl = new_btls;

        // If the lowest (would-be) winner is first on their party's ticket, we can remove ATL
        // votes for that party/group
        let mut new_atls = vec![] ;
        if data.metadata.candidate(min_manipulation.removed_winner).position == Some(1) {
            if let Some(party) = data.metadata.candidate(min_manipulation.removed_winner).party {
                for v in data.atl {
                    if v.parties[0] == party {
                        if v.n <= num_to_go {
                            num_to_go -= v.n;
                        } else {
                            new_atls.push(ATL {
                                parties: v.parties,
                                n: v.n - num_to_go
                            })
                        }
                    } else {
                        new_atls.push(v);
                    }
                }
            }
            data.atl = new_atls;
        }

        let altered_transcript = distribute_preferences::<Rules>(&data,data.metadata.vacancies.unwrap(),&data.metadata.excluded.iter().cloned().collect(),&data.metadata.tie_resolutions,false);

        if num_to_go == 0 {
            println!("Electorate {}: min manipulation {}. Result: {:?}", data.metadata.name.electorate, min_manipulation.size, compare_transcripts(&transcript,&altered_transcript));
        }
    }
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
