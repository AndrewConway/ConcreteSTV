// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use std::fs::File;
use nsw::nsw_random_rules::{NSWECrandomLGE2016, NSWrandomLGE};
use nsw::parse_lge::{get_nsw_lge_data_loader_2016, NSWLGEDataLoader, NSWLGEDataSource};
use stv::ballot_metadata::CandidateIndex;
use stv::distribution_of_preferences_transcript::{CountIndex, TranscriptWithMetadata};
use stv::official_dop_transcript::{DifferenceBetweenOfficialDoPAndComputed, DifferenceBetweenOfficialDoPAndComputedOnParticularCount, ECTally, test_official_dop_without_actual_votes};
use stv::parse_util::{FileFinder, RawDataSource};
use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules};
use stv::tie_resolution::{TieResolutionAtom, TieResolutionExplicitDecision, TieResolutionsMadeByEC};


fn test<Rules:PreferenceDistributionRules>(electorate:&str,loader:&NSWLGEDataLoader) {
    let data = loader.read_raw_data(electorate).unwrap();
    data.print_summary();
    let mut tie_resolutions = TieResolutionsMadeByEC::default();
    let official_transcript = loader.read_official_dop_transcript(&data.metadata).unwrap();
    loop {
        let transcript = distribute_preferences::<Rules>(&data, loader.candidates_to_be_elected(electorate), &data.metadata.excluded.iter().cloned().collect(), &tie_resolutions,None,false);
        let transcript = TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript };
        std::fs::create_dir_all("test_transcripts").unwrap();
        {
            let file = File::create(format!("test_transcripts/NSW LG{} {}.transcript",transcript.metadata.name.year,electorate)).unwrap();
            serde_json::to_writer_pretty(file,&transcript).unwrap();
        }
        match official_transcript.compare_with_transcript_checking_for_ec_decisions(&transcript.transcript,false) {
            Ok(None) => { return; }
            Ok(Some(decision)) => {
                println!("Observed tie resolution favouring {:?} over {:?}", decision.favoured, decision.disfavoured);
                assert!(decision.favoured.iter().map(|c|c.0).min().unwrap() < decision.disfavoured[0].0, "favoured candidate should be lower as higher candidates are assumed favoured.");
                tie_resolutions.tie_resolutions.push(TieResolutionAtom::ExplicitDecision(decision));
            }
            Err(DifferenceBetweenOfficialDoPAndComputed::DifferentNumbersOfCounts(official,our)) => {
                println!("Official DoP had {} counts; ConcreteSTV had {}. Not surprising as the algorithm contains random elements.",official,our);
                return;
            }
            Err(DifferenceBetweenOfficialDoPAndComputed::DifferentOnCount(count_index,_,diff)) => {
                println!("There was a difference between the official DoP and ConcreteSTV's on count {} : {}",1+count_index.0,diff);
                if count_index.0<2 {
                    panic!("A count error on count {} is not explainable by the random part of the algorithm : {}",1+count_index.0,diff);
                } else {
                    println!("This is probably due to the random elements of the algorithm.");
                    return;
                }
            }
            Err(e) => {
                panic!("There was a difference between the official DoP and ConcreteSTV's : {}",e);
            }
        }
    }
}



#[test]
fn test_2016_plausible() {
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_nsw_lge_data_loader_2016(&finder).unwrap();
    println!("Made loader");
    let electorate =&loader.all_electorates()[0];
    assert_eq!(electorate,"Albury City Council");
    for electorate in &loader.all_electorates() {
        test::<NSWECrandomLGE2016>(electorate,&loader);
    }
}

#[test]
fn test_bland_shire_nswec_bug() {
    let finder = FileFinder::find_ec_data_repository();
    let loader = get_nsw_lge_data_loader_2016(&finder).unwrap();
    let data = loader.read_raw_data("Bland Shire Council").unwrap();
    let official_dop = loader.read_official_dop_transcript(&data.metadata).unwrap();
    let used_rules = data.distribute_preferences::<NSWECrandomLGE2016>();
    let without_rounding_errors = data.distribute_preferences::<NSWrandomLGE>();
    assert_eq!(Err(DifferenceBetweenOfficialDoPAndComputed::DifferentOnCount(CountIndex(1),None,DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalCandidate(ECTally(260.),259,CandidateIndex(0)))),
               official_dop.compare_with_transcript_checking_for_ec_decisions(&without_rounding_errors,false));
    assert_ne!(Err(DifferenceBetweenOfficialDoPAndComputed::DifferentOnCount(CountIndex(1),None,DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalCandidate(ECTally(260.),259,CandidateIndex(0)))),
               official_dop.compare_with_transcript_checking_for_ec_decisions(&used_rules,false));
}

#[test]
fn test_2016_internally_consistent() {
    let finder = FileFinder::find_ec_data_repository();
    let loader = get_nsw_lge_data_loader_2016(&finder).unwrap();
    for electorate in &loader.all_electorates() {
        println!("Testing electorate {}",electorate);
        assert_eq!(test_internally_consistent::<NSWECrandomLGE2016>("2016",electorate).unwrap(),Ok(None));
    }
}

/// Test a particular year & electorate against a particular set of rules.
/// Outermost error is IO type errors.
/// Innermost error is discrepancies with the official DoP.
fn test_internally_consistent<Rules:PreferenceDistributionRules>(year:&str,state:&str) -> anyhow::Result<Result<Option<TieResolutionExplicitDecision>, DifferenceBetweenOfficialDoPAndComputed<Rules::Tally>>> where <Rules as PreferenceDistributionRules>::Tally: Send+Sync+'static {
    test_official_dop_without_actual_votes::<Rules,_>(&NSWLGEDataSource{},year,state,false)
}

