// Copyright 2021-2024 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use std::collections::HashSet;
use std::fs::File;
use nsw::{NSWECLocalGov2021, SimpleIRVAnyDifferenceBreaksTies};
use nsw::parse_lge::{get_nsw_lge_data_loader_2024, NSWLGEDataLoader};
use stv::ballot_metadata::CandidateIndex;
use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
use stv::parse_util::{FileFinder, RawDataSource};
use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules};
use stv::random_util::Randomness;
use stv::tie_resolution::TieResolutionAtom;


fn test<Rules:PreferenceDistributionRules>(electorate:&str,loader:&NSWLGEDataLoader) {
    let data = loader.read_raw_data(electorate).unwrap();
    data.print_summary();
    let mut tie_resolutions = data.metadata.tie_resolutions.clone();
    let official_transcript = loader.read_official_dop_transcript(&data.metadata).unwrap();
    loop {
        let transcript = distribute_preferences::<Rules>(&data, loader.candidates_to_be_elected(electorate), &data.metadata.excluded.iter().cloned().collect(), &tie_resolutions,None,false,&mut Randomness::ReverseDonkeyVote);
        let transcript = TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript };
        std::fs::create_dir_all("test_transcripts").unwrap();
        {
            let file = File::create(format!("test_transcripts/NSW LG{} {}.transcript",transcript.metadata.name.year,electorate)).unwrap();
            serde_json::to_writer_pretty(file,&transcript).unwrap();
        }
        if let Some(decision) = official_transcript.compare_with_transcript_checking_for_ec_decisions(&transcript.transcript,true).unwrap() {
            println!("Observed tie resolution {}", decision.decision);
            tie_resolutions.tie_resolutions.push(TieResolutionAtom::ExplicitDecision(decision));
        } else {
            return;
        }
    }
}

#[test]
fn test_ineligible() {
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_nsw_lge_data_loader_2024(&finder).unwrap();
    let data = loader.read_raw_data_checking_electorate_valid("Ballina - B Ward").unwrap();
    assert_eq!(data.metadata.excluded,vec![CandidateIndex(0)]);
}


#[test]
/// Test all 2024 Mayoral elections
fn test_2024_mayoral() {
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_nsw_lge_data_loader_2024(&finder).unwrap();
    println!("Made loader");
    let electorate =&loader.all_electorates()[0];
    assert_eq!(electorate,"City of Albury");
    for electorate in &loader.all_electorates() {
        if electorate.ends_with(" Mayoral") {
            println!("Testing Electorate {}",electorate);
            test::<SimpleIRVAnyDifferenceBreaksTies>(electorate, &loader);
        }
    }
}
#[test]
fn test_2024_council() {
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_nsw_lge_data_loader_2024(&finder).unwrap();
    println!("Made loader");
    let electorate =&loader.all_electorates()[0];
    assert_eq!(electorate,"City of Albury");
    for electorate in &loader.all_electorates() {
        if !electorate.ends_with(" Mayoral") {
            println!("Testing Electorate {}",electorate);
            if electorate!="Balranald" { // the Balranald official DoP seems corrupt.
                test::<NSWECLocalGov2021>(electorate,&loader);
            }
        }
    }
}

#[allow(non_snake_case)]
#[test]
fn test_2024_Upper_Lachlan_Shire() { // Upper Lachlan Shire had a tie for elected candidates on round 1. The EC decision is hard coded in the parse metadata function.
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_nsw_lge_data_loader_2024(&finder).unwrap();
    println!("Made loader");
    test::<NSWECLocalGov2021>("Upper Lachlan Shire",&loader);
}

#[allow(non_snake_case)]
#[test]
fn test_2024_Balrand() { // The DoP speadsheet on the NSWEC website for Balrand was corrupt. I looked at the HTML DoP, but it is missing the sub-counts, and seems to count all exhausted votes as "lost" rather than "exhausted" which is always blank. First encountered, 7.1
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}", finder.path);
    let loader = get_nsw_lge_data_loader_2024(&finder).unwrap();
    println!("Made loader");
    let data = loader.read_raw_data_checking_electorate_valid("Balranald").unwrap();
    data.print_summary();
    let transcript = distribute_preferences::<NSWECLocalGov2021>(&data, loader.candidates_to_be_elected("Balranald"), &data.metadata.excluded.iter().cloned().collect(), &data.metadata.tie_resolutions, None, false, &mut Randomness::ReverseDonkeyVote);
    let transcript = TranscriptWithMetadata { metadata: data.metadata.clone(), transcript };
    std::fs::create_dir_all("test_transcripts").unwrap();
    {
        let file = File::create(format!("test_transcripts/NSW LG{} {}.transcript", transcript.metadata.name.year, "Balranald")).unwrap();
        serde_json::to_writer_pretty(file, &transcript).unwrap();
    }
    assert_eq!(transcript.transcript.elected.iter().copied().collect::<HashSet<CandidateIndex>>(), transcript.metadata.results.unwrap().iter().copied().collect::<HashSet<CandidateIndex>>());
}
/*
#[test]
fn make_stv_file_of_everything() {
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_nsw_lge_data_loader_2021(&finder).unwrap();
    println!("Made loader");
    std::fs::create_dir_all("test_stv_files").unwrap();
    for electorate in loader.all_electorates() {
        println!("Trying to load {}",&electorate);
        let data = loader.read_raw_data(&electorate).unwrap();
        data.print_summary();
        let file = File::create(format!("test_stv_files/NSW LG{} {}.stv",data.metadata.name.year,electorate)).unwrap();
        serde_json::to_writer(file,&data).unwrap();
    }
}*/
