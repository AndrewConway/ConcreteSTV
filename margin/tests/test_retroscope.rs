// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Look back over a transcript, and answer the question of which candidate's pile a vote is sitting on at a given count, and with what transfer value.

use std::collections::HashSet;
use std::str::FromStr;
use federal::FederalRules;
use margin::choose_votes::ChooseVotesOptions;
use stv::ballot_metadata::{Candidate, CandidateIndex, ElectionMetadata, ElectionName, NumberOfCandidates, Party, PartyIndex};
use stv::ballot_paper::{ATL, BTL};
use stv::distribution_of_preferences_transcript::CountIndex;
use stv::election_data::ElectionData;
use stv::preference_distribution::distribute_preferences;
use stv::tie_resolution::TieResolutionsMadeByEC;
use margin::retroscope::{PileStatus, Retroscope, RetroscopeVoteIndex, RetroscopeVoteStatus};
use stv::ballot_pile::BallotPaperCount;
use stv::transfer_value::TransferValue;

#[test]
fn test_retroscope() {
    let vote_data = ElectionData {
        metadata: ElectionMetadata {
            name: ElectionName {
                year: "".to_string(),
                authority: "".to_string(),
                name: "".to_string(),
                electorate: "".to_string(),
                modifications: vec![],
                comment: None
            },
            candidates: vec![
                Candidate{ name: "A1".to_string(),  party: Some(PartyIndex(0)), position: Some(1), ec_id: None },
                Candidate{ name: "A2".to_string(),  party: Some(PartyIndex(0)), position: Some(2), ec_id: None },
                Candidate{ name: "B1".to_string(),  party: Some(PartyIndex(1)), position: Some(1), ec_id: None },
                Candidate{ name: "B2".to_string(),  party: Some(PartyIndex(1)), position: Some(2), ec_id: None },
                Candidate{ name: "C1".to_string(),  party: None, position: None, ec_id: None },
            ],
            parties: vec![
                Party{ column_id: "A".to_string(), name: "The group of people who like A".to_string(),  abbreviation: None, atl_allowed: true, candidates: vec![CandidateIndex(0),CandidateIndex(1)], tickets: vec![] },
                Party{ column_id: "B".to_string(), name: "The group of people who like B".to_string(),  abbreviation: None, atl_allowed: true, candidates: vec![CandidateIndex(2),CandidateIndex(3)], tickets: vec![] },
            ],
            source: vec![],
            results: None,
            vacancies: Some(NumberOfCandidates(3)),
            enrolment: None,
            secondary_vacancies: None,
            excluded: vec![],
            tie_resolutions: Default::default()
        },
        atl: vec![
            ATL{ parties : vec![PartyIndex(0)], n:100},
            ATL{ parties : vec![PartyIndex(1)], n:100},
        ],
        btl: vec![
            BTL{ candidates: vec![CandidateIndex(0)], n: 80 },
            BTL{ candidates: vec![CandidateIndex(1)], n: 10 },
            BTL{ candidates: vec![CandidateIndex(2),CandidateIndex(4)], n: 60 },
            BTL{ candidates: vec![CandidateIndex(3)], n: 50 },
            BTL{ candidates: vec![CandidateIndex(4),CandidateIndex(2),CandidateIndex(1)], n: 1 },
        ],
        informal: 0
    };
    let transcript = distribute_preferences::<FederalRules>(&vote_data,NumberOfCandidates(3),&HashSet::new(),&TieResolutionsMadeByEC::default(),false);
    println!("{}",serde_json::to_string_pretty(&transcript).unwrap());
    let mut retroscope = Retroscope::new(&vote_data,&[]);

    retroscope.apply(CountIndex(0),transcript.count(CountIndex(0)));
    // First preferences - should get candidates 0 and 2 elected.
    assert_eq!(retroscope.count,CountIndex(0));
    assert_eq!(retroscope.continuing,[CandidateIndex(1),CandidateIndex(3),CandidateIndex(4)].into_iter().collect::<HashSet<CandidateIndex>>());
    assert_eq!(retroscope.elected,vec![CandidateIndex(0),CandidateIndex(2)]);
    fn assert_pile(s:&RetroscopeVoteStatus,c:CandidateIndex,count:CountIndex) {
        assert_eq!(s.pile_status,PileStatus::InPile,"Pile status");
        assert_eq!(s.count_arrived,count,"Count arrived");
        assert_eq!(s.candidate(),Some(c),"Candidate whose pile it is in");
    }
    assert_pile(&retroscope.votes.atl[0],CandidateIndex(0),CountIndex(0));
    assert_pile(&retroscope.votes.atl[1],CandidateIndex(2),CountIndex(0));
    assert_pile(&retroscope.votes.btl[0],CandidateIndex(0),CountIndex(0));
    assert_pile(&retroscope.votes.btl[1],CandidateIndex(1),CountIndex(0));
    assert_pile(&retroscope.votes.btl[2],CandidateIndex(2),CountIndex(0));
    assert_pile(&retroscope.votes.btl[3],CandidateIndex(3),CountIndex(0));
    assert_pile(&retroscope.votes.btl[4],CandidateIndex(4),CountIndex(0));
    assert_eq!(retroscope.piles_by_candidate[0].by_count.get(&CountIndex(0)).unwrap(),&vec![RetroscopeVoteIndex(0),RetroscopeVoteIndex(2)]);
    assert_eq!(retroscope.piles_by_candidate[1].by_count.get(&CountIndex(0)).unwrap(),&vec![RetroscopeVoteIndex(3)]);
    assert_eq!(retroscope.transfer_value(CountIndex(0)),&TransferValue::one());

    retroscope.apply(CountIndex(1),transcript.count(CountIndex(1)));
    // Second count - distribute candidate 0.
    assert_eq!(retroscope.continuing,[CandidateIndex(1),CandidateIndex(3),CandidateIndex(4)].into_iter().collect::<HashSet<CandidateIndex>>());
    assert_pile(&retroscope.votes.atl[0],CandidateIndex(1),CountIndex(1));
    assert_pile(&retroscope.votes.atl[1],CandidateIndex(2),CountIndex(0));
    assert_eq!(retroscope.votes.btl[0].pile_status,PileStatus::Exhausted);
    assert_pile(&retroscope.votes.btl[1],CandidateIndex(1),CountIndex(0));
    assert_pile(&retroscope.votes.btl[2],CandidateIndex(2),CountIndex(0));
    assert_pile(&retroscope.votes.btl[3],CandidateIndex(3),CountIndex(0));
    assert_pile(&retroscope.votes.btl[4],CandidateIndex(4),CountIndex(0));
    assert_eq!(retroscope.piles_by_candidate[1].by_count.get(&CountIndex(0)).unwrap(),&vec![RetroscopeVoteIndex(3)]);
    assert_eq!(retroscope.piles_by_candidate[1].by_count.get(&CountIndex(1)).unwrap(),&vec![RetroscopeVoteIndex(0)]);
    assert_eq!(retroscope.transfer_value(CountIndex(1)),&TransferValue::from_str("79/180").unwrap());

    retroscope.apply(CountIndex(2),transcript.count(CountIndex(2)));
    // Third count - distribute candidate 2. atl[1] goes to 3, btl[2] goes to 4.
    assert_eq!(retroscope.continuing,[CandidateIndex(1),CandidateIndex(3),CandidateIndex(4)].into_iter().collect::<HashSet<CandidateIndex>>());
    assert_pile(&retroscope.votes.atl[0],CandidateIndex(1),CountIndex(1));
    assert_pile(&retroscope.votes.atl[1],CandidateIndex(3),CountIndex(2));
    assert_eq!(retroscope.votes.btl[0].pile_status,PileStatus::Exhausted);
    assert_pile(&retroscope.votes.btl[1],CandidateIndex(1),CountIndex(0));
    assert_pile(&retroscope.votes.btl[2],CandidateIndex(4),CountIndex(2));
    assert_pile(&retroscope.votes.btl[3],CandidateIndex(3),CountIndex(0));
    assert_pile(&retroscope.votes.btl[4],CandidateIndex(4),CountIndex(0));
    assert_eq!(retroscope.piles_by_candidate[3].by_count.get(&CountIndex(0)).unwrap(),&vec![RetroscopeVoteIndex(5)]);
    assert_eq!(retroscope.piles_by_candidate[3].by_count.get(&CountIndex(2)).unwrap(),&vec![RetroscopeVoteIndex(1)]);
    assert_eq!(retroscope.piles_by_candidate[4].by_count.get(&CountIndex(0)).unwrap(),&vec![RetroscopeVoteIndex(6)]);
    assert_eq!(retroscope.piles_by_candidate[4].by_count.get(&CountIndex(2)).unwrap(),&vec![RetroscopeVoteIndex(4)]);
    assert_eq!(retroscope.transfer_value(CountIndex(2)),&TransferValue::from_str("59/160").unwrap());

    // Test ChooseVotes
    let mut chooser1 = retroscope.get_chooser(CandidateIndex(1),&vote_data,ChooseVotesOptions{ allow_atl: true, allow_first_pref: true });
    assert!(chooser1.get_votes::<FederalRules>(1000).is_none());
    let mut chooser1 = retroscope.get_chooser(CandidateIndex(1),&vote_data,ChooseVotesOptions{ allow_atl: false, allow_first_pref: false });
    assert!(chooser1.get_votes::<FederalRules>(1).is_none());
    let mut chooser1 = retroscope.get_chooser(CandidateIndex(1),&vote_data,ChooseVotesOptions{ allow_atl: true, allow_first_pref: true });
    let found1 = chooser1.get_votes::<FederalRules>(4).unwrap(); // there are 10 BTL TV 1, and 100 ATL TV 79/180
    assert_eq!(found1.papers,BallotPaperCount(4));
    assert_eq!(found1.which_votes.len(),1);
    assert_eq!(found1.which_votes[0].n,4);
    assert_eq!(found1.which_votes[0].from,RetroscopeVoteIndex(3));
    let found1 = chooser1.get_votes::<FederalRules>(1).unwrap(); // there are 6 BTL TV 1, and 100 ATL TV 79/180 left
    assert_eq!(found1.papers,BallotPaperCount(1));
    assert_eq!(found1.which_votes.len(),1);
    assert_eq!(found1.which_votes[0].n,1);
    assert_eq!(found1.which_votes[0].from,RetroscopeVoteIndex(3));
    let found1 = chooser1.get_votes::<FederalRules>(25).unwrap(); // there are 5 BTL TV 1, and 100 ATL TV 79/180 left
    assert_eq!(found1.papers,BallotPaperCount(51));
    assert_eq!(found1.which_votes.len(),2);
    assert_eq!(found1.which_votes[0].n,5);
    assert_eq!(found1.which_votes[0].from,RetroscopeVoteIndex(3));
    assert_eq!(found1.which_votes[1].n,46); // 46*79/180 = 20.xxx
    assert_eq!(found1.which_votes[1].from,RetroscopeVoteIndex(0));
    assert!(chooser1.get_votes::<FederalRules>(30).is_none());


    retroscope.apply(CountIndex(3),transcript.count(CountIndex(3)));
    // Fourth count - eliminate candidate 4, TV 1 btl[4] goes to 1.
    assert_eq!(retroscope.continuing,[CandidateIndex(1),CandidateIndex(3)].into_iter().collect::<HashSet<CandidateIndex>>());
    assert_eq!(retroscope.elected,vec![CandidateIndex(0),CandidateIndex(2)]);
    assert_pile(&retroscope.votes.atl[0],CandidateIndex(1),CountIndex(1));
    assert_pile(&retroscope.votes.atl[1],CandidateIndex(3),CountIndex(2));
    assert_eq!(retroscope.votes.btl[0].pile_status,PileStatus::Exhausted);
    assert_pile(&retroscope.votes.btl[1],CandidateIndex(1),CountIndex(0));
    assert_pile(&retroscope.votes.btl[2],CandidateIndex(4),CountIndex(2));
    assert_pile(&retroscope.votes.btl[3],CandidateIndex(3),CountIndex(0));
    assert_pile(&retroscope.votes.btl[4],CandidateIndex(1),CountIndex(3));
    assert_eq!(retroscope.piles_by_candidate[1].by_count.get(&CountIndex(0)).unwrap(),&vec![RetroscopeVoteIndex(3)]);
    assert_eq!(retroscope.piles_by_candidate[1].by_count.get(&CountIndex(1)).unwrap(),&vec![RetroscopeVoteIndex(0)]);
    assert_eq!(retroscope.piles_by_candidate[1].by_count.get(&CountIndex(3)).unwrap(),&vec![RetroscopeVoteIndex(6)]);
    assert_eq!(retroscope.transfer_value(CountIndex(3)),&TransferValue::from_str("1/1").unwrap());

    retroscope.apply(CountIndex(4),transcript.count(CountIndex(4)));
    // Fourth count - eliminate candidate 4, TV 59/160 btl[2] goes to exhausted. Candidate 3 gets elected
    assert_eq!(retroscope.continuing,[CandidateIndex(1)].into_iter().collect::<HashSet<CandidateIndex>>());
    assert_eq!(retroscope.elected,vec![CandidateIndex(0),CandidateIndex(2),CandidateIndex(3)]);
    assert_pile(&retroscope.votes.atl[0],CandidateIndex(1),CountIndex(1));
    assert_pile(&retroscope.votes.atl[1],CandidateIndex(3),CountIndex(2));
    assert_eq!(retroscope.votes.btl[0].pile_status,PileStatus::Exhausted);
    assert_pile(&retroscope.votes.btl[1],CandidateIndex(1),CountIndex(0));
    assert_eq!(retroscope.votes.btl[2].pile_status,PileStatus::Exhausted);
    assert_pile(&retroscope.votes.btl[3],CandidateIndex(3),CountIndex(0));
    assert_pile(&retroscope.votes.btl[4],CandidateIndex(1),CountIndex(3));
    assert_eq!(retroscope.transfer_value(CountIndex(4)),&TransferValue::from_str("59/160").unwrap());
}