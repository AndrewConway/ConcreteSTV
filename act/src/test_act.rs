// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This runs the federal elections and compares the results to the AEC provided transcripts.


#[cfg(test)]
mod tests {
    use crate::parse::{get_act_data_loader_2020, get_act_data_loader_2016, ACTDataLoader, get_act_data_loader_2012, get_act_data_loader_2008};
    use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules};
    use std::collections::HashSet;
    use stv::tie_resolution::TieResolutionsMadeByEC;
    use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
    use std::fs::File;
    use stv::parse_util::{RawDataSource, FileFinder};
    use crate::{ACTPre2020};


    fn test2020(electorate: &str) -> anyhow::Result<()> {
        let loader = get_act_data_loader_2020(&FileFinder::find_ec_data_repository())?;
        let data = loader.load_cached_data(electorate)?;
        data.print_summary();
        let transcript = distribute_preferences::<ACTPre2020>(&data, loader.candidates_to_be_elected(electorate), &HashSet::default(), &TieResolutionsMadeByEC::default());
        let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
        std::fs::create_dir_all("test_transcripts")?;
        let file = File::create(format!("test_transcripts/transcript{}2020.json",electorate))?;
        serde_json::to_writer_pretty(file,&transcript)?;
        let official_transcript = loader.read_official_dop_transcript(&transcript.metadata,None)?;
        official_transcript.compare_with_transcript(&transcript.transcript,|tally|tally as f64);
        Ok(())
    }

    fn test<Rules:PreferenceDistributionRules,F:Fn(Rules::Tally)->f64>(electorate:&str,loader:ACTDataLoader,decode:F) -> anyhow::Result<()> {
        let data = loader.load_cached_data(electorate)?;
        data.print_summary();
        let transcript = distribute_preferences::<Rules>(&data, loader.candidates_to_be_elected(electorate), &HashSet::default(), &TieResolutionsMadeByEC::default());
        let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
        std::fs::create_dir_all("test_transcripts")?;
        let file = File::create(format!("test_transcripts/transcript{}{}.json",electorate,transcript.metadata.name.year))?;
        serde_json::to_writer_pretty(file,&transcript)?;
        let official_transcript = loader.read_official_dop_transcript(&transcript.metadata,None)?;
        official_transcript.compare_with_transcript(&transcript.transcript,decode);
        Ok(())
    }

    fn test2016(electorate: &str) -> anyhow::Result<()> {
        let loader = get_act_data_loader_2016(&FileFinder::find_ec_data_repository())?;
        test::<ACTPre2020,_>(electorate,loader,|tally|tally as f64)
    }

    fn test2012(electorate: &str) -> anyhow::Result<()> {
        let loader = get_act_data_loader_2012(&FileFinder::find_ec_data_repository())?;
        test::<ACTPre2020,_>(electorate,loader,|tally|tally as f64)
    }

    fn test2008(electorate: &str) -> anyhow::Result<()> {
        let loader = get_act_data_loader_2008(&FileFinder::find_ec_data_repository())?;
        test::<ACTPre2020,_>(electorate,loader,|tally|tally as f64)
    }


    #[test]
    #[allow(non_snake_case)]
    fn test_Brindabella2016() { test2016("Brindabella").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_Ginninderra2016() { test2016("Ginninderra").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_Kurrajong2016() { test2016("Kurrajong").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_Murrumbidgee2016() { test2016("Murrumbidgee").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_Yerrabi2016() { test2016("Yerrabi").unwrap() }

    #[test]
    #[allow(non_snake_case)]
    fn test_Brindabella2012() { test2012("Brindabella").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_Ginninderra2012() { test2012("Ginninderra").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_Molonglo2012() { test2012("Molonglo").unwrap() }

    #[test]
    #[allow(non_snake_case)]
    fn test_Brindabella2008() { test2008("Brindabella").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_Ginninderra2008() { test2008("Ginninderra").unwrap() }
    #[test]
    #[allow(non_snake_case)]
    fn test_Molonglo2008() { test2008("Molonglo").unwrap() }


}