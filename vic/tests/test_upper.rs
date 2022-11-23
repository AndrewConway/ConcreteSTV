// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::fs::File;
use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
use stv::parse_util::{FileFinder, RawDataSource};
use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules};
use stv::tie_resolution::{TieResolutionAtom, TieResolutionsMadeByEC};
use vic::parse_vic::{get_vic_data_loader_2014, VicDataLoader};
use vic::Vic2018LegislativeCouncil;

fn test<Rules:PreferenceDistributionRules>(electorate:&str, loader:&VicDataLoader) {
    let data = loader.read_raw_data(electorate).unwrap();
    data.print_summary();
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
        if let Some(decision) = official_transcript.compare_with_transcript_checking_for_ec_decisions(&transcript.transcript,true) {
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
