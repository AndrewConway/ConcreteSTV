// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use nsw::nsw_random_rules::{NSWECrandomLGE2012, NSWECrandomLGE2017};
use nsw::parse_lge::{get_nsw_lge_data_loader_2012};
use nsw::run_election_multiple_times::PossibleResults;
use stv::ballot_metadata::CandidateIndex;
use stv::parse_util::{FileFinder, RawDataSource};



#[test]
fn test_2012_plausible() {
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_nsw_lge_data_loader_2012(&finder).unwrap();
    println!("Made loader");
    assert_eq!(&loader.all_electorates()[0],"Albury City Council");
    for electorate in &loader.all_electorates() {
        println!("Testing Electorate {}",electorate);
        let data = loader.read_raw_data(electorate).unwrap();
        data.print_summary();
    }
}

#[test]
/// From a prior project we have estimates of probability of different candidates winning for Boorowa Council:
/// ```text
///Candidate	Proportion Elected	Mean position	Official Count
///TUCKERMAN Wendy	1.000000	1.000000	1
///SYKES Peter	1.000000	2.000000	2
///CORCORAN Christopher	1.000000	3.000000	3
///RYAN Jack	1.000000	4.000000	4
///GLEDHILL Robert	1.000000	5.240372	6
///McGRATH Tim	1.000000	5.759628	5
///EVANS David	0.999962	8.113406	7
///SOUTHWELL Andrew	0.995019	7.458948	8
///MAGEE Paul	0.663534	8.201037
///CLEMENTS Angus	0.341447	8.853784	9
///COTTER Grant	0.000038	9.000000
/// ```
///
/// Note that there is a chance that this will fail if we are absurdly unlucky.
fn test_boorowa_run_10000_times_and_check_probabilistic_winners_reasonably_close_to_expected() {
    let finder = FileFinder::find_ec_data_repository();
    let loader = get_nsw_lge_data_loader_2012(&finder).unwrap();
    let data = loader.read_raw_data("Boorowa Council").unwrap();
    let results = PossibleResults::new_from_runs::<NSWECrandomLGE2017>(&data,10000);
    results.print_table_results(&data.metadata);
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(12),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(11),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(4),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(10),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(2),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(9),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(7),0.999962));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(1),0.995019));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(5),0.663534));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(6),0.341447));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(8),0.000038));
    assert_eq!("1",results.candidates[12].mean_position_elected().to_string());
    assert_eq!("2",results.candidates[11].mean_position_elected().to_string());
    assert_eq!("3",results.candidates[4].mean_position_elected().to_string());
    assert_eq!("4",results.candidates[10].mean_position_elected().to_string());
    assert!(results.candidates[2].mean_position_elected()>5.05);
    assert!(results.candidates[2].mean_position_elected()<5.5);
}


#[test]
/// From a prior project we have estimates of probability of different candidates winning for Griffith City Council:
/// ```text
/// Candidate	Proportion Elected	Mean position	Official Count
/// NEVILLE Mike	1.000000	1.000000	1
/// LANCASTER Bill	1.000000	2.000000	2
/// ZAPPACOSTA Dino	1.000000	3.000000	3
/// STEAD Christine	1.000000	4.000000	4
/// COX Pat	1.000000	5.000000	5
/// NAPOLI Anne	1.000000	6.000000	6
/// THORPE Leon	1.000000	7.000000	7
/// CROCE Simon	1.000000	8.000000	8
/// ROSSETTO Paul	1.000000	9.000015	9
/// CURRAN Doug	1.000000	9.999985	10
/// MERCURI Rina	0.911583	11.000000
/// BALIND Alison	0.088417	11.000000	11
/// ```
///
/// Note that there is a chance that this will fail if we are absurdly unlucky.
fn test_griffith_run_10000_times_and_check_probabilistic_winners_reasonably_close_to_expected_using_less_buggy_count() {
    let finder = FileFinder::find_ec_data_repository();
    let loader = get_nsw_lge_data_loader_2012(&finder).unwrap();
    let data = loader.read_raw_data("Griffith City Council").unwrap();
    let results = PossibleResults::new_from_runs::<NSWECrandomLGE2017>(&data,10000);
    results.print_table_results(&data.metadata);
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(8),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(6),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(2),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(4),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(15),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(9),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(1),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(5),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(10),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(14),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(3),0.911583));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(7),0.088417));
    assert_eq!("1",results.candidates[8].mean_position_elected().to_string());
    assert_eq!("2",results.candidates[6].mean_position_elected().to_string());
    assert_eq!("3",results.candidates[2].mean_position_elected().to_string());
    assert_eq!("4",results.candidates[4].mean_position_elected().to_string());
}

#[test]
/// From a personal communication from the NSWEC we have estimates of probability of different candidates winning for Griffith City Council,
/// using their software, which I think NSWECrandomLGE2012 emulates.
/// In this case MERCURI Rina wins 10% of the time and BALIND Alison 90% of the time.
/// See our report NSWLGE2012CountErrorTechReport for details.
///
/// Note that there is a chance that this will fail if we are absurdly unlucky.
fn test_griffith_run_100_times_and_check_probabilistic_winners_reasonably_close_to_expected_using_more_buggy_count() {
    let finder = FileFinder::find_ec_data_repository();
    let loader = get_nsw_lge_data_loader_2012(&finder).unwrap();
    let data = loader.read_raw_data("Griffith City Council").unwrap();
    let results = PossibleResults::new_from_runs::<NSWECrandomLGE2012>(&data,100);
    results.print_table_results(&data.metadata);
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(8),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(6),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(2),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(4),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(15),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(9),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(1),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(5),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(10),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(14),1.0));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(3),0.1));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(7),0.9));
    assert_eq!("1",results.candidates[8].mean_position_elected().to_string());
    assert_eq!("2",results.candidates[6].mean_position_elected().to_string());
    assert_eq!("3",results.candidates[2].mean_position_elected().to_string());
    assert_eq!("4",results.candidates[4].mean_position_elected().to_string());
}
