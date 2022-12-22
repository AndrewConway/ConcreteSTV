// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::fs::File;
use stv::ballot_metadata::CandidateIndex;
use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
use stv::parse_util::{FileFinder, RawDataSource};
use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules};
use stv::tie_resolution::{TieResolutionAtom, TieResolutionsMadeByEC};
use vic::parse_vic::{get_vic_data_loader_2014, get_vic_data_loader_2022, VicDataLoader};
use vic::Vic2018LegislativeCouncil;

fn test<Rules:PreferenceDistributionRules>(electorate:&str, loader:&VicDataLoader) {
    let mut data = loader.read_raw_data(electorate).unwrap();
    data.print_summary();
    if data.metadata.name.year=="2022" && data.metadata.name.electorate=="North-Eastern Metropolitan Region" {
        // There is a bug, possibly in the VEC software, possibly in data transmission that produces an extra
        // vote in count 169 going to candidate DOLAN, Hugh. The most likely vote misinterpreted IMHO is the vote
        // [#31, #32, #33, #34, #35, #19, #20, #25, #26, #50, #51] which is missing a preference 12. Pretending it exists
        // solves the discrepancy.
        // just allowing gaps generally causes a whole lot of other issues, (and is pretty clearly against the legislation).
        let changed_vote = vec![CandidateIndex(31),CandidateIndex(32),CandidateIndex(33),CandidateIndex(34),CandidateIndex(35),CandidateIndex(19),CandidateIndex(20),CandidateIndex(25),CandidateIndex(26),CandidateIndex(50),CandidateIndex(51)];
        for v in &mut data.btl {
            if v.candidates==changed_vote {
                for c in 1..data.metadata.candidates.len() {
                    if !changed_vote.contains(&CandidateIndex(c)) { v.candidates.push(CandidateIndex(c)) }
                }
            }
        }
    }
    let mut tie_resolutions = TieResolutionsMadeByEC::default();
    let official_transcript = loader.read_official_dop_transcript(&data.metadata).unwrap();
    loop {
        let transcript = distribute_preferences::<Rules>(&data, loader.candidates_to_be_elected(electorate), &data.metadata.excluded.iter().cloned().collect(), &tie_resolutions,None,false);
        let transcript = TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript };
        std::fs::create_dir_all("test_transcripts").unwrap();
        {
            let file = File::create(format!("test_transcripts/Vic {} {}.transcript",transcript.metadata.name.year,electorate)).unwrap();
            serde_json::to_writer_pretty(file,&transcript).unwrap();
        }
        if let Some(decision) = official_transcript.compare_with_transcript_checking_for_ec_decisions(&transcript.transcript,true).unwrap() {
            println!("Observed tie resolution favouring {:?} over {:?}", decision.favoured, decision.disfavoured);
            assert!(decision.favoured.iter().map(|c|c.0).min().unwrap() < decision.disfavoured[0].0, "favoured candidate should be lower as higher candidates are assumed favoured.");
            tie_resolutions.tie_resolutions.push(TieResolutionAtom::ExplicitDecision(decision));
        } else {
            return;
        }
    }
}

#[test]
fn test_all_upper_house_races_2014() {
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_vic_data_loader_2014(&finder).unwrap();
    println!("Made loader");
    for electorate in &loader.all_electorates() {
        if electorate=="Eastern Metropolitan Region" { continue; } // The Eastern Metropolitan Region results were changed "Minor adjustments included May 2015 following routine integrity checks." according to the VEC website and do not agree with the vote data we were given.
        println!("Testing Electorate {}",electorate);
        test::<Vic2018LegislativeCouncil>(electorate, &loader);
    }
}

#[test]
fn test_all_upper_house_races_2022() {
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_vic_data_loader_2022(&finder).unwrap();
    println!("Made loader");
    for electorate in &loader.all_electorates() {
        println!("Testing Electorate {}",electorate);
        test::<Vic2018LegislativeCouncil>(electorate, &loader);
    }
}
