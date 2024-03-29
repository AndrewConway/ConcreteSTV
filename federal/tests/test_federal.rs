// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This runs the federal elections and compares the results to the AEC provided transcripts.


#[cfg(test)]
mod tests {
    use federal::parse::{get_federal_data_loader_2016, get_federal_data_loader_2019, get_federal_data_loader_2013, get_federal_data_loader_2022, get_federal_data_loader_2014};
    use stv::preference_distribution::distribute_preferences;
    use federal::{FederalRulesUsed2019, FederalRulesUsed2016, FederalRulesUsed2013};
    use std::collections::HashSet;
    use stv::tie_resolution::TieResolutionsMadeByEC;
    use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
    use std::fs::File;
    use std::iter::FromIterator;
    use stv::compare_transcripts::{DeltasInCandidateLists, DifferentCandidateLists};
    use stv::parse_util::{RawDataSource, FileFinder};
    use stv::random_util::Randomness;


    fn test2022(state:&str) -> anyhow::Result<()> {
        let loader = get_federal_data_loader_2022(&FileFinder::find_ec_data_repository());
        let data = loader.read_raw_data(state)?;
        data.print_summary();
        //use stv::ballot_metadata::CandidateIndex;
        //let mut excluded = HashSet::default();
        //excluded.insert(CandidateIndex(8));
        let transcript = distribute_preferences::<FederalRulesUsed2019>(&data, loader.candidates_to_be_elected(state), &HashSet::default(), &TieResolutionsMadeByEC::default(),None,true,&mut Randomness::ReverseDonkeyVote);
        let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
        std::fs::create_dir_all("test_transcripts")?;
        let file = File::create(format!("test_transcripts/transcript{}2022.json",state))?;
        serde_json::to_writer_pretty(file,&transcript)?;
        let official_transcript = loader.read_official_dop_transcript(&transcript.metadata)?;

        if official_transcript.all_elected()!=transcript.transcript.elected {
            let lists : DeltasInCandidateLists = DifferentCandidateLists{list1:official_transcript.all_elected(),list2:transcript.transcript.elected.clone() }.into();
            println!("Official elected differs from computed : {}",lists.pretty_print(&transcript.metadata));
        }
        official_transcript.compare_with_transcript(&transcript.transcript);
        for (number,count) in transcript.transcript.counts.iter().enumerate() {
            for decision in &count.decisions {
                println!("Decision made in count {} : {}",number+1,decision)
            }
        }
        Ok(())
    }

    fn test2019(state:&str) -> anyhow::Result<()> {
        let loader = get_federal_data_loader_2019(&FileFinder::find_ec_data_repository());
        let data = loader.read_raw_data(state)?;
        data.print_summary();
        let transcript = distribute_preferences::<FederalRulesUsed2019>(&data, loader.candidates_to_be_elected(state), &HashSet::default(), &TieResolutionsMadeByEC::default(),None,true,&mut Randomness::ReverseDonkeyVote);
        let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
        std::fs::create_dir_all("test_transcripts")?;
        let file = File::create(format!("test_transcripts/transcript{}2019.json",state))?;
        serde_json::to_writer_pretty(file,&transcript)?;
        let official_transcript = loader.read_official_dop_transcript(&transcript.metadata)?;
        official_transcript.compare_with_transcript(&transcript.transcript);
        for (number,count) in transcript.transcript.counts.iter().enumerate() {
            for decision in &count.decisions {
                println!("Decision made in count {} : {}",number+1,decision)
            }
        }
        Ok(())
    }

    fn test2016(state:&str) -> anyhow::Result<()> {
        let loader = get_federal_data_loader_2016(&FileFinder::find_ec_data_repository());
        let data = loader.read_raw_data(state)?;
        data.print_summary();
        let transcript = distribute_preferences::<FederalRulesUsed2016>(&data, loader.candidates_to_be_elected(state), &HashSet::from_iter(loader.excluded_candidates(state)), &loader.ec_decisions(state),None,true,&mut Randomness::ReverseDonkeyVote);
        let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
        std::fs::create_dir_all("test_transcripts")?;
        let file = File::create(format!("test_transcripts/transcript{}2016.json",state))?;
        serde_json::to_writer_pretty(file,&transcript)?;
        let official_transcript = loader.read_official_dop_transcript(&transcript.metadata)?;
        official_transcript.compare_with_transcript(&transcript.transcript);
        Ok(())
    }

    fn test2014(state:&str) -> anyhow::Result<()> {
        let loader = get_federal_data_loader_2014(&FileFinder::find_ec_data_repository());
        let data = loader.read_raw_data(state)?;
        data.print_summary();
        let transcript = distribute_preferences::<FederalRulesUsed2013>(&data, loader.candidates_to_be_elected(state), &HashSet::from_iter(loader.excluded_candidates(state)), &loader.ec_decisions(state),None,true,&mut Randomness::ReverseDonkeyVote);
        let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
        std::fs::create_dir_all("test_transcripts")?;
        let file = File::create(format!("test_transcripts/transcript{}2014.json",state))?;
        serde_json::to_writer_pretty(file,&transcript)?;
        let official_transcript = loader.read_official_dop_transcript(&transcript.metadata)?;
        official_transcript.compare_with_transcript(&transcript.transcript);
        Ok(())
    }

    fn test2013(state:&str) -> anyhow::Result<()> {
        let loader = get_federal_data_loader_2013(&FileFinder::find_ec_data_repository());
        let data = loader.read_raw_data(state)?;
        data.print_summary();
        let transcript = distribute_preferences::<FederalRulesUsed2013>(&data, loader.candidates_to_be_elected(state), &HashSet::from_iter(loader.excluded_candidates(state)), &loader.ec_decisions(state),None,true,&mut Randomness::ReverseDonkeyVote);
        let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
        std::fs::create_dir_all("test_transcripts")?;
        let file = File::create(format!("test_transcripts/transcript{}2013.json",state))?;
        serde_json::to_writer_pretty(file,&transcript)?;
        let official_transcript = loader.read_official_dop_transcript(&transcript.metadata)?;
        official_transcript.compare_with_transcript(&transcript.transcript);
        Ok(())
    }


    #[test]
    #[allow(non_snake_case)]
    fn test_ACT2013() { test2013("ACT").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_WA2013() { test2013("WA").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_SA2013() { test2013("SA").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_NT2013() { test2013("NT").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_QLD2013() { test2013("QLD").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_NSW2013() { test2013("NSW").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_VIC2013() { test2013("VIC").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_TAS2013() { test2013("TAS").unwrap() }

    #[test]
    #[allow(non_snake_case)]
    fn test_WA2014() { test2014("WA").unwrap() }

    #[test]
    #[allow(non_snake_case)]
    fn test_ACT2016() { test2016("ACT").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_WA2016() { test2016("WA").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_SA2016() { test2016("SA").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_NT2016() { test2016("NT").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_QLD2016() { test2016("QLD").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_NSW2016() { test2016("NSW").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_VIC2016() { test2016("VIC").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_TAS2016() { test2016("TAS").unwrap() }


    #[test]
    #[allow(non_snake_case)]
    fn test_ACT2019() { test2019("ACT").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_WA2019() { test2019("WA").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_SA2019() { test2019("SA").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_NT2019() { test2019("NT").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_QLD2019() { test2019("QLD").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_NSW2019() { test2019("NSW").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_VIC2019() { test2019("VIC").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_TAS2019() { test2019("TAS").unwrap() }

    #[test]
    #[allow(non_snake_case)]
    fn test_ACT2022() { test2022("ACT").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_WA2022() { test2022("WA").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_SA2022() { test2022("SA").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_NT2022() { test2022("NT").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_QLD2022() { test2022("QLD").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_NSW2022() { test2022("NSW").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_VIC2022() { test2022("VIC").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_TAS2022() { test2022("TAS").unwrap() }


}
