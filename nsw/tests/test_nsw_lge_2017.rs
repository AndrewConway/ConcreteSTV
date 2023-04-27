// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use std::fs::File;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use nsw::nsw_random_rules::{NSWECrandomLGE2017};
use nsw::parse_lge::{get_nsw_lge_data_loader_2017, NSWLGEDataLoader, NSWLGEDataSource};
use nsw::run_election_multiple_times::PossibleResults;
use stv::ballot_metadata::CandidateIndex;
use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
use stv::official_dop_transcript::{DifferenceBetweenOfficialDoPAndComputed, test_official_dop_without_actual_votes};
use stv::parse_util::{FileFinder, RawDataSource};
use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules};
use stv::random_util::Randomness;
use stv::tie_resolution::{TieResolutionAtom, TieResolutionExplicitDecisionInCount};


fn test<Rules:PreferenceDistributionRules>(electorate:&str,loader:&NSWLGEDataLoader) {
    let data = loader.read_raw_data(electorate).unwrap();
    data.print_summary();
    let mut tie_resolutions = data.metadata.tie_resolutions.clone();
    let official_transcript = loader.read_official_dop_transcript(&data.metadata).unwrap();
    let mut randomness = Randomness::PRNG(ChaCha20Rng::seed_from_u64(1));
    loop {
        let transcript = distribute_preferences::<Rules>(&data, loader.candidates_to_be_elected(electorate), &data.metadata.excluded.iter().cloned().collect(), &tie_resolutions,None,false,&mut randomness);
        let transcript = TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript };
        std::fs::create_dir_all("test_transcripts").unwrap();
        {
            let file = File::create(format!("test_transcripts/NSW LG{} {}.transcript",transcript.metadata.name.year,electorate)).unwrap();
            serde_json::to_writer_pretty(file,&transcript).unwrap();
        }
        match official_transcript.compare_with_transcript_checking_for_ec_decisions(&transcript.transcript,true) {
            Ok(None) => { return; }
            Ok(Some(decision)) => {
                println!("Observed tie resolution {}", decision.decision);
                tie_resolutions.tie_resolutions.push(TieResolutionAtom::ExplicitDecision(decision));
            }
            Err(DifferenceBetweenOfficialDoPAndComputed::DifferentNumbersOfCounts(official,our)) => {
                println!("Official DoP had {} counts; ConcreteSTV had {}. Not surprising as the algorithm contains random elements.",official,our);
                return;
            }
            Err(DifferenceBetweenOfficialDoPAndComputed::DifferentOnCount(count_index,_,diff)) => {
                println!("Tie resolutions : {:?}",tie_resolutions);
                println!("There was a difference between the official DoP and ConcreteSTV's on count {} : {}",1+count_index.0,diff);
                if count_index.0<2 {
                    panic!("A count error on count {} is not explainable by the random part of the algorithm : {}",1+count_index.0,diff);
                } else {
                    println!("This is probably due to the random elements of the algorithm.");
                    return;
                }
            }
            Err(e) => {
                println!("Tie resolutions : {:?}",tie_resolutions);
                panic!("There was a difference between the official DoP and ConcreteSTV's : {}",e);
            }
        }
    }
}



#[test]
fn test_2017_plausible() {
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_nsw_lge_data_loader_2017(&finder).unwrap();
    println!("Made loader");
    assert_eq!(&loader.all_electorates()[0],"Armidale Regional");
    for electorate in &loader.all_electorates() {
        test::<NSWECrandomLGE2017>(electorate,&loader);
        println!("Testing Electorate {}",electorate);
    }
}

#[test]
fn test_wollstonecraft() {
    let finder = FileFinder::find_ec_data_repository();
    let loader = get_nsw_lge_data_loader_2017(&finder).unwrap();
    test::<NSWECrandomLGE2017>("North Sydney - Wollstonecraft Ward",&loader);
}


#[test]
/// From a prior project we have estimates of probability of different candidates winning for North Sydney Wollstonecraft Ward:
/// ```text
/// Candidate	Proportion Elected	Mean position	Official Count
/// BAKER Zoe	1.000000	1.000000	1
/// MUTTON Ian	1.000000	2.000000	2
/// GUNNING Samuel	0.789956	3.000000	3
/// KELLY Tim	0.210044	3.000000
/// ```
///
/// Note that there is a chance that this will fail if we are absurdly unlucky.
fn test_wollstonecraft_run_10000_times_and_check_probabilistic_winners_reasonably_close_to_expected() {
    let finder = FileFinder::find_ec_data_repository();
    let loader = get_nsw_lge_data_loader_2017(&finder).unwrap();
    let data = loader.read_raw_data("North Sydney - Wollstonecraft Ward").unwrap();
    let mut randomness = Randomness::PRNG(ChaCha20Rng::seed_from_u64(1));
    let results = PossibleResults::new_from_runs::<NSWECrandomLGE2017>(&data,10000,&mut randomness);
    results.print_table_results(&data.metadata);
    assert_eq!(10000,results.candidates[9].num_times_elected);
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(9),1.0));
    assert_eq!(10000,results.candidates[3].num_times_elected);
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(3),1.0));
    assert_eq!(10000,results.candidates[0].num_times_elected+results.candidates[6].num_times_elected);
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(0),0.789956));
    assert!(results.is_close_to_expected_prob_winning(CandidateIndex(6),0.210044));
    assert!(1000<results.candidates[6].num_times_elected);
    assert!(3500>results.candidates[6].num_times_elected);
    assert_eq!("1",results.candidates[9].mean_position_elected().to_string());
    assert_eq!("2",results.candidates[3].mean_position_elected().to_string());
    assert_eq!("3",results.candidates[0].mean_position_elected().to_string());
    assert_eq!("3",results.candidates[6].mean_position_elected().to_string());
}


#[test]
fn test_2017_internally_consistent() {
    let finder = FileFinder::find_ec_data_repository();
    let loader = get_nsw_lge_data_loader_2017(&finder).unwrap();
    for electorate in &loader.all_electorates() {
        // there is something bizarre in the Federation DoP. On the NSWEC website, Federation, count 35, the second candidate WALES Norm ended the count with 623 votes. But on count 36, Wales Norm started the count with 630 votes. Other people also magically change tally. There seems no plausible way to emulate this.
        // there is something bizarre in the Inner West - Marrickville Ward DoP. On the NSWEC website, count 12, the webpage is not a count webpage but rather a duplicate of the DoP summary page.
        if electorate!="Federation" && electorate!="Inner West - Marrickville Ward" {
            println!("Testing electorate {}",electorate);
            assert_eq!(test_internally_consistent::<NSWECrandomLGE2017>("2017",electorate).unwrap(),Ok(None));
        }
    }
}

/// Test a particular year & electorate against a particular set of rules.
/// Outermost error is IO type errors.
/// Innermost error is discrepancies with the official DoP.
fn test_internally_consistent<Rules:PreferenceDistributionRules>(year:&str,state:&str) -> anyhow::Result<Result<Option<TieResolutionExplicitDecisionInCount>, DifferenceBetweenOfficialDoPAndComputed<Rules::Tally>>> where <Rules as PreferenceDistributionRules>::Tally: Send+Sync+'static {
    test_official_dop_without_actual_votes::<Rules,_>(&NSWLGEDataSource{},year,state,false)
}

