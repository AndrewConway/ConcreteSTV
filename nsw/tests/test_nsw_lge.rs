// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This runs the NSW Local Gov against a variety of artificial test cases.
//! TODO This is just an early start, many more tests are needed.


#[cfg(test)]
mod tests {
    use stv::election_data::ElectionData;
    use stv::ballot_metadata::{ElectionMetadata, ElectionName, Candidate, NumberOfCandidates, CandidateIndex};
    use stv::ballot_paper::BTL;
    use nsw::NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation;
    use stv::ballot_pile::BallotPaperCount;
    use stv::random_util::Randomness;

    fn candidate(name: &str) -> Candidate {
        Candidate {
            name: name.to_string(),
            party: None,
            position: None,
            ec_id: None
        }
    }

    fn candidates(names: &[&str]) -> Vec<Candidate> {
        names.iter().map(|&n|candidate(n)).collect()
    }

    fn election_name(name: &str) -> ElectionName {
        ElectionName {
            year: "2021".to_string(),
            authority: "testing".to_string(),
            name: "NSW LGE".to_string(),
            electorate: name.to_string(),
            modifications: vec![],
            comment: None
        }
    }

    #[test]
    /// A very simple test. In the first count, no one gets a quota.
    /// The second count is exclusion of the last candidate.
    /// Then the first 3 candidates are elected under rule 11(3).
    fn test_terminate_count2() -> anyhow::Result<()> {
        let data = ElectionData {
            metadata: ElectionMetadata {
                name: election_name("terminate count 2"),
                candidates: candidates(&["A", "B", "C", "D", "E", "F"]),
                parties: vec![],
                source: vec![],
                results: None,
                vacancies: Some(NumberOfCandidates(3)),
                enrolment: None,
                secondary_vacancies: None,
                excluded: vec![],
                tie_resolutions: Default::default()
            },
            atl: vec![],
            atl_types: vec![],
            btl: vec![
                BTL { candidates: vec![CandidateIndex(0)], n: 10000 },
                BTL { candidates: vec![CandidateIndex(1)], n: 10000 },
                BTL { candidates: vec![CandidateIndex(2)], n: 10000 },
                BTL { candidates: vec![CandidateIndex(3)], n: 9000 },
                BTL { candidates: vec![CandidateIndex(4)], n: 900 },
                BTL { candidates: vec![CandidateIndex(5)], n: 100 },
            ],
            btl_types: vec![],
            informal: 0
        };
        let transcript = data.distribute_preferences::<NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation>(&mut Randomness::ReverseDonkeyVote);
        assert_eq!(transcript.quota.as_ref().unwrap().papers, BallotPaperCount(40000));
        assert_eq!(transcript.quota.as_ref().unwrap().quota, 10001);
        assert_eq!(transcript.elected, vec![CandidateIndex(2), CandidateIndex(1), CandidateIndex(0)]);
        assert_eq!(transcript.counts.len(), 2);
        Ok(())
    }
}