// Copyright 2022-2025 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This tests how the official transcripts compare to the rules, with no knowledge of the actual votes.

use std::collections::HashSet;
use std::fs::File;
use federal::parse_house_reps::{parse_HouseDopByDivisionDownload, FederalHouseRepresentativesIRV, FederalHouseRepresentativesIRVAlwaysSimpleIRVToTwoCandidates};
use stv::ballot_metadata::CandidateIndex;
use stv::distribution_of_preferences_transcript::{CountIndex, TranscriptWithMetadata};
use stv::official_dop_transcript::{DifferenceBetweenOfficialDoPAndComputed, DifferenceBetweenOfficialDoPAndComputedOnParticularCount, ECTally};
use stv::parse_util::FileFinder;
use stv::preference_distribution::PreferenceDistributionRules;
use stv::verify_official_transcript::distribute_preferences_using_official_results;

/// Test the AEC counts for the given year.
fn test<Rules:PreferenceDistributionRules>(year:&str,ec_code:&str,just_division:Option<&str>,ignore:HashSet<&str>) -> Result<(),DifferenceBetweenOfficialDoPAndComputed<Rules::Tally>> where <Rules as PreferenceDistributionRules>::Tally: Send+Sync+'static {
    let finder = FileFinder::find_ec_data_repository();
    let filename = format!("HouseDopByDivisionDownload-{}.csv",ec_code);
    let archive_location = format!("Federal/{}/House of Representatives",year);
    let source_url = format!("https://results.aec.gov.au/31496/Website/HouseDownloadsMenu-31496-Csv.htm");
    let path = finder.find_raw_data_file(&filename,&archive_location,&source_url).unwrap();
    let divisions = parse_HouseDopByDivisionDownload(&path,year,&source_url).unwrap();
    for division in divisions {
        if just_division.as_ref().is_some_and(|s|*s!=division.metadata.name.electorate) {continue;}
        if ignore.contains(division.metadata.name.electorate.as_str()) {continue;}
        println!("Found division {}",division.metadata.name.human_readable_name());
        let official_transcript = division.dop;
        //println!("{:#?}",official_transcript.counts);
        let metadata = division.metadata;
        //println!("{:#?}",metadata);
        let transcript = distribute_preferences_using_official_results::<Rules>(&official_transcript, &metadata).unwrap();
        let result = official_transcript.compare_with_transcript_checking_for_ec_decisions(&transcript,false);
        if SAVE_TRANSCRIPTS {
            let transcript = TranscriptWithMetadata{ metadata, transcript };
            std::fs::create_dir_all(format!("test_house_rep_transcripts/{year}")).unwrap();
            let file = File::create(format!("test_house_rep_transcripts/{year}/transcript_{}.json",transcript.metadata.name.electorate)).unwrap();
            serde_json::to_writer_pretty(file,&transcript).unwrap();
        }
        match &result {
            Ok(_) => {}
            Err(e) => println!("{}",e),
        }
        result?;
    }
    Ok(())
}


const SAVE_TRANSCRIPTS : bool = true;

#[test]
fn test_2013() {
    assert!(test::<FederalHouseRepresentativesIRVAlwaysSimpleIRVToTwoCandidates>("2013","17496",None,HashSet::new()).is_ok());
}

#[test]
fn test_2016() {
    assert!(test::<FederalHouseRepresentativesIRVAlwaysSimpleIRVToTwoCandidates>("2016","20499",None,HashSet::new()).is_ok());
}


#[test]
fn test_2019() {
    assert!(test::<FederalHouseRepresentativesIRVAlwaysSimpleIRVToTwoCandidates>("2019","24310",None,HashSet::new()).is_ok());
}

#[test]
fn test_2022() {
    // There is an error in the New England (NSW) count. In the file HouseDopByDivisionDownload-27966.csv
    // * On line 5803 Sparks, Carol, has 8676 votes, being her tally at the end of count "4" (the fifth count - the first is "0").
    // * On line 5835 Sparks, Carol, has 0 votes, being her tally at the end of count "5" (the sixth count - the first is "0").
    // * On line 5837 this is explained as Sparks, Carol "gained" "-8677" votes, being the difference between 8676 and 0. Well, almost.
    // That same count Laura Hughes gains "5861" votes to take her tally from 19629 to 25489 with some correspondingly idiosyncratic arithmetic. 
    assert_eq!(test::<FederalHouseRepresentativesIRVAlwaysSimpleIRVToTwoCandidates>("2022","27966",Some("New England (NSW)"),HashSet::new()),Err(DifferenceBetweenOfficialDoPAndComputed::DifferentOnCount(CountIndex(5),None,DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyDeltaCandidate(ECTally(-8677.),8676,0,CandidateIndex(6)))));
    assert!(test::<FederalHouseRepresentativesIRVAlwaysSimpleIRVToTwoCandidates>("2022","27966",None,["New England (NSW)"].into_iter().collect()).is_ok());
}


#[test]
fn test_2025() {
    // In Eden-Monaro there is a multiple exclusion on round 2 that results in a winner on count 2.
    assert_eq!(test::<FederalHouseRepresentativesIRV>("2025","31496",Some("Eden-Monaro (NSW)"),HashSet::new()),Err(DifferenceBetweenOfficialDoPAndComputed::DifferentOnCount(CountIndex(1),None,DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ElectedCandidatesUnordered(vec![],vec![CandidateIndex(5)]))));
    // In Chifley there is a candidate with an absolute majority of first preferences.
    assert_eq!(test::<FederalHouseRepresentativesIRV>("2025","31496",Some("Chifley (NSW)"),HashSet::new()),Err(DifferenceBetweenOfficialDoPAndComputed::DifferentOnCount(CountIndex(0),None,DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ElectedCandidatesUnordered(vec![],vec![CandidateIndex(3)]))));
    // check all 2025 candidates.
    assert!(test::<FederalHouseRepresentativesIRVAlwaysSimpleIRVToTwoCandidates>("2025","31496",None,HashSet::new()).is_ok());
}

