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
use margin::evaluate_and_optimize_vote_changes::{ChangeResult, optimise, simple_test};
use stv::ballot_metadata::{Candidate, CandidateIndex, ElectionMetadata, ElectionName, NumberOfCandidates, Party, PartyIndex};
use stv::ballot_paper::{ATL, BTL};
use stv::distribution_of_preferences_transcript::CountIndex;
use stv::election_data::ElectionData;
use stv::preference_distribution::distribute_preferences;
use stv::tie_resolution::TieResolutionsMadeByEC;
use margin::retroscope::{PileStatus, Retroscope, RetroscopeVoteIndex, RetroscopeVoteStatus};
use margin::vote_changes::{VoteChange, VoteChanges};
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
            results: Some(vec![CandidateIndex(0),CandidateIndex(2),CandidateIndex(3)]),
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
        atl_types: vec![],
        btl: vec![
            BTL{ candidates: vec![CandidateIndex(0)], n: 80 },
            BTL{ candidates: vec![CandidateIndex(1)], n: 10 },
            BTL{ candidates: vec![CandidateIndex(2),CandidateIndex(4)], n: 60 },
            BTL{ candidates: vec![CandidateIndex(3)], n: 50 },
            BTL{ candidates: vec![CandidateIndex(4),CandidateIndex(2),CandidateIndex(1)], n: 1 },
        ],
        btl_types: vec![],
        informal: 0
    };
    let transcript = distribute_preferences::<FederalRules>(&vote_data,NumberOfCandidates(3),&HashSet::new(),&TieResolutionsMadeByEC::default(),false);
    println!("{}",serde_json::to_string_pretty(&transcript).unwrap());
    let mut retroscope = Retroscope::new(&vote_data,&[]);
    assert_eq!(false,retroscope.is_highest_continuing_member_party_ticket(CandidateIndex(1),&vote_data.metadata)); // 0 is above
    assert_eq!(false,retroscope.is_highest_continuing_member_party_ticket(CandidateIndex(3),&vote_data.metadata)); // 2 is above
    assert_eq!(false,retroscope.is_highest_continuing_member_party_ticket(CandidateIndex(4),&vote_data.metadata)); // not on a ticket

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
    assert_eq!(retroscope.piles_by_candidate[3].by_count.get(&CountIndex(1)),None);
    assert_eq!(retroscope.piles_by_candidate[3].by_count.get(&CountIndex(2)).unwrap(),&vec![RetroscopeVoteIndex(1)]);
    assert_eq!(retroscope.piles_by_candidate[4].by_count.get(&CountIndex(0)).unwrap(),&vec![RetroscopeVoteIndex(6)]);
    assert_eq!(retroscope.piles_by_candidate[4].by_count.get(&CountIndex(2)).unwrap(),&vec![RetroscopeVoteIndex(4)]);
    assert_eq!(retroscope.transfer_value(CountIndex(2)),&TransferValue::from_str("59/160").unwrap());

    assert_eq!(true,retroscope.is_highest_continuing_member_party_ticket(CandidateIndex(1),&vote_data.metadata)); // top of ticket
    assert_eq!(true,retroscope.is_highest_continuing_member_party_ticket(CandidateIndex(3),&vote_data.metadata)); // top of ticket
    assert_eq!(false,retroscope.is_highest_continuing_member_party_ticket(CandidateIndex(4),&vote_data.metadata)); // not on a ticket
    // Test ChooseVotes
    let mut chooser1 = retroscope.get_chooser(CandidateIndex(1),&vote_data,&ChooseVotesOptions{ allow_atl: true, allow_first_pref: true, allow_verifiable: true, ballot_types_considered_unverifiable: Default::default() });
    assert!(chooser1.get_votes::<FederalRules>(1000,true).is_none());
    let mut chooser1 = retroscope.get_chooser(CandidateIndex(1),&vote_data,&ChooseVotesOptions{ allow_atl: false, allow_first_pref: false, allow_verifiable: true, ballot_types_considered_unverifiable: Default::default() });
    assert!(chooser1.get_votes::<FederalRules>(1,true).is_none());
    let mut chooser1 = retroscope.get_chooser(CandidateIndex(1),&vote_data,&ChooseVotesOptions{ allow_atl: true, allow_first_pref: true, allow_verifiable: true, ballot_types_considered_unverifiable: Default::default() });
    assert_eq!(10,chooser1.votes_available_btl::<FederalRules>());
    assert_eq!(53,chooser1.votes_available_total::<FederalRules>());
    let found1 = chooser1.get_votes::<FederalRules>(4,true).unwrap(); // there are 10 BTL TV 1, and 100 ATL TV 79/180
    assert_eq!(found1.len(),1);
    assert_eq!(found1[0].n,BallotPaperCount(4));
    assert_eq!(found1[0].tally,4);
    assert_eq!(found1[0].tv,TransferValue::one());
    assert_eq!(found1[0].ballots.len(),1);
    assert_eq!(found1[0].ballots[0].n,4);
    assert_eq!(found1[0].ballots[0].from,RetroscopeVoteIndex(3));
    let found1 = chooser1.get_votes::<FederalRules>(1,true).unwrap(); // there are 6 BTL TV 1, and 100 ATL TV 79/180 left
    assert_eq!(found1.len(),1);
    assert_eq!(found1[0].n,BallotPaperCount(1));
    assert_eq!(found1[0].tally,1);
    assert_eq!(found1[0].tv,TransferValue::one());
    assert_eq!(found1[0].ballots.len(),1);
    assert_eq!(found1[0].ballots[0].n,1);
    assert_eq!(found1[0].ballots[0].from,RetroscopeVoteIndex(3));
    let found1 = chooser1.get_votes::<FederalRules>(25,true).unwrap(); // there are 5 BTL TV 1, and 100 ATL TV 79/180 left
    assert_eq!(found1.len(),2);
    assert_eq!(found1[0].n,BallotPaperCount(5));
    assert_eq!(found1[1].n,BallotPaperCount(46));
    assert_eq!(found1[0].tally,5);
    assert_eq!(found1[1].tally,20);
    assert_eq!(found1[0].tv,TransferValue::one());
    assert_eq!(found1[1].tv,TransferValue::from_str("79/180").unwrap());
    assert_eq!(found1[0].ballots.len(),1);
    assert_eq!(found1[0].ballots[0].n,5);
    assert_eq!(found1[0].ballots[0].from,RetroscopeVoteIndex(3));
    assert_eq!(found1[1].ballots.len(),1);
    assert_eq!(found1[1].ballots[0].n,46);
    assert_eq!(found1[1].ballots[0].from,RetroscopeVoteIndex(0));
    assert!(chooser1.get_votes::<FederalRules>(30,true).is_none());

    let attempted_changes = VoteChanges{ changes: vec![VoteChange{ vote_value: 30, from: Some(CandidateIndex(1)), to: Some(CandidateIndex(4)) }] };
    let concrete = attempted_changes.make_concrete::<FederalRules>(&retroscope,&vote_data,&ChooseVotesOptions{ allow_atl: true, allow_first_pref: true, allow_verifiable: true, ballot_types_considered_unverifiable: Default::default() });
    assert!(concrete.is_none());
    let attempted_changes = VoteChanges{ changes: vec![VoteChange{ vote_value: 30, from: Some(CandidateIndex(1)), to: Some(CandidateIndex(3)) }] };
    let concrete = attempted_changes.make_concrete::<FederalRules>(&retroscope,&vote_data,&ChooseVotesOptions{ allow_atl: true, allow_first_pref: true, allow_verifiable: true, ballot_types_considered_unverifiable: Default::default() }).unwrap();
    assert_eq!(2,concrete.changes.len());
    assert_eq!(BallotPaperCount(10),concrete.changes[0].n);
    assert_eq!(10,concrete.changes[0].tally);
    assert_eq!(Some(CandidateIndex(3)),concrete.changes[0].candidate_to);
    assert_eq!(CandidateIndex(1),concrete.changes[0].from.as_ref().unwrap().candidate);
    assert_eq!(1,concrete.changes[0].from.as_ref().unwrap().ballots.len());
    assert_eq!(10,concrete.changes[0].from.as_ref().unwrap().ballots[0].n);
    assert_eq!(RetroscopeVoteIndex(3),concrete.changes[0].from.as_ref().unwrap().ballots[0].from);
    assert_eq!(TransferValue::one(),concrete.changes[0].from.as_ref().unwrap().tv);
    assert_eq!(BallotPaperCount(46),concrete.changes[1].n);
    assert_eq!(20,concrete.changes[1].tally);
    assert_eq!(Some(CandidateIndex(3)),concrete.changes[1].candidate_to);
    assert_eq!(CandidateIndex(1),concrete.changes[1].from.as_ref().unwrap().candidate);
    assert_eq!(1,concrete.changes[1].from.as_ref().unwrap().ballots.len());
    assert_eq!(46,concrete.changes[1].from.as_ref().unwrap().ballots[0].n);
    assert_eq!(RetroscopeVoteIndex(0),concrete.changes[1].from.as_ref().unwrap().ballots[0].from);
    assert_eq!(TransferValue::from_str("79/180").unwrap(),concrete.changes[1].from.as_ref().unwrap().tv);

    // consider changing the outcome of the election at this point. Candidate 1 has 53 votes, 3 has 86, 4 has 23. Normally 4 would be excluded, giving 1 vote to candidate 1, and then candidate 3 gets elected 86 to 54. This could be changed by moving 17 votes from candidate 3 to candidate 1.
    let vote_changes = VoteChanges{ changes: vec![VoteChange{ vote_value: 20, from: Some(CandidateIndex(3)), to: Some(CandidateIndex(1)) }] };
    match simple_test::<FederalRules>(&vote_changes,&vote_data,&retroscope,&ChooseVotesOptions{ allow_atl: true, allow_first_pref: true, allow_verifiable: true, ballot_types_considered_unverifiable: Default::default() }) {
        ChangeResult::NoChange => panic!("No change!"),
        ChangeResult::NotEnoughVotesAvailable => panic!("Not enough votes available!"),
        ChangeResult::Change(deltas,ballot_changes) => {
            assert_eq!(deltas.list2only,vec![CandidateIndex(3)]);
            assert_eq!(deltas.list1only,vec![CandidateIndex(1)]);
            assert_eq!(ballot_changes.n,BallotPaperCount(20)); // first prefs.
        }
    }

    let optimize_result = optimise::<FederalRules>(&vote_changes,&vote_data,&retroscope,&ChooseVotesOptions{ allow_atl: true, allow_first_pref: true, allow_verifiable: true, ballot_types_considered_unverifiable: Default::default() }).unwrap();
    assert_eq!(optimize_result.deltas.list2only,vec![CandidateIndex(3)]);
    assert_eq!(optimize_result.deltas.list1only,vec![CandidateIndex(1)]);
    assert_eq!(optimize_result.changes.n,BallotPaperCount(17)); // optimized it down to 17.

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