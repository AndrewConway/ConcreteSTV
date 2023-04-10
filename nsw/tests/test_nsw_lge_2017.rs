// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use std::fs::File;
use nsw::nsw_random_rules::NSWrandomLGE;
use nsw::parse_lge::{get_nsw_lge_data_loader_2017, NSWLGEDataLoader};
use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
use stv::official_dop_transcript::DifferenceBetweenOfficialDoPAndComputed;
use stv::parse_util::{FileFinder, RawDataSource};
use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules};
use stv::tie_resolution::{TieResolutionAtom, TieResolutionsMadeByEC};


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
        match official_transcript.compare_with_transcript_checking_for_ec_decisions(&transcript.transcript,true) {
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
fn test_2017_plausible() {
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_nsw_lge_data_loader_2017(&finder).unwrap();
    println!("Made loader");
    let electorate =&loader.all_electorates()[0];
    assert_eq!(electorate,"Armidale Regional");
    for electorate in &loader.all_electorates() {
        test::<NSWrandomLGE>(electorate,&loader);
        println!("Testing Electorate {}",electorate);
//        test::<NSWECLocalGov2021>(electorate,&loader);
        let metadata = loader.read_raw_metadata(electorate).unwrap();
        println!("{:?}",metadata);
        let data = loader.read_raw_data(electorate).unwrap();
        data.print_summary();
        let _dop = loader.read_official_dop_transcript(&metadata).unwrap();
    }
}

#[test]
fn test_wollstonecraft() {
    let finder = FileFinder::find_ec_data_repository();
    let loader = get_nsw_lge_data_loader_2017(&finder).unwrap();
    test::<NSWrandomLGE>("North Sydney - Wollstonecraft Ward",&loader);
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
fn test_wollstonecraft_run_1000_times_and_check_probabilistic_winners_reasonably_close_to_expected() {
    let finder = FileFinder::find_ec_data_repository();
    let loader = get_nsw_lge_data_loader_2017(&finder).unwrap();
    let data = loader.read_raw_data("North Sydney - Wollstonecraft Ward").unwrap();
    let mut num_times_elected = vec![0;data.metadata.candidates.len()];
    for _ in 0..1000 {
        let result = data.distribute_preferences::<NSWrandomLGE>();
        for e in result.elected { num_times_elected[e.0]+=1; }
    }
    assert_eq!(1000,num_times_elected[3]);
    assert_eq!(1000,num_times_elected[9]);
    assert_eq!(1000,num_times_elected[0]+num_times_elected[6]);
    assert!(100<num_times_elected[6]);
    assert!(350>num_times_elected[6]);
    for candidate_index in 0..num_times_elected.len() {
        if num_times_elected[candidate_index]>0 {
            println!("Candidate {} : {} elected {} times ",candidate_index,data.metadata.candidates[candidate_index].name,num_times_elected[candidate_index]);
        }
    }
}


