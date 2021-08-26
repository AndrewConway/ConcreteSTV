


#[cfg(test)]
mod tests {
    use crate::parse::{get_federal_data_loader_2016, get_federal_data_loader_2019, get_federal_data_loader_2013};
    use stv::preference_distribution::distribute_preferences;
    use crate::{FederalRulesUsed2019, FederalRulesUsed2016, FederalRulesUsed2013};
    use std::collections::HashSet;
    use stv::tie_resolution::TieResolutionsMadeByEC;
    use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
    use std::fs::File;
    use std::iter::FromIterator;

    fn test2019(state:&str) -> anyhow::Result<()> {
        let loader = get_federal_data_loader_2019();
        let data = loader.load_cached_data(state)?;
        data.print_summary();
        let transcript = distribute_preferences::<FederalRulesUsed2019>(&data, loader.candidates_to_be_elected(state), &HashSet::default(), &TieResolutionsMadeByEC::default());
        let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
        std::fs::create_dir_all("test_transcripts")?;
        let file = File::create(format!("test_transcripts/transcript{}2019.json",state))?;
        serde_json::to_writer_pretty(file,&transcript)?;
        let official_transcript = loader.read_official_dop_transcript(&transcript.metadata)?;
        official_transcript.compare_with_transcript(&transcript.transcript,|tally|tally as f64);
        Ok(())
    }

    fn test2016(state:&str) -> anyhow::Result<()> {
        let loader = get_federal_data_loader_2016();
        let data = loader.load_cached_data(state)?;
        data.print_summary();
        let transcript = distribute_preferences::<FederalRulesUsed2016>(&data, loader.candidates_to_be_elected(state), &HashSet::from_iter(loader.excluded_candidates(state)), &loader.ec_decisions(state));
        let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
        std::fs::create_dir_all("test_transcripts")?;
        let file = File::create(format!("test_transcripts/transcript{}2016.json",state))?;
        serde_json::to_writer_pretty(file,&transcript)?;
        let official_transcript = loader.read_official_dop_transcript(&transcript.metadata)?;
        official_transcript.compare_with_transcript(&transcript.transcript,|tally|tally as f64);
        Ok(())
    }

    fn test2013(state:&str) -> anyhow::Result<()> {
        let loader = get_federal_data_loader_2013();
        let data = loader.load_cached_data(state)?;
        data.print_summary();
        let transcript = distribute_preferences::<FederalRulesUsed2013>(&data, loader.candidates_to_be_elected(state), &HashSet::from_iter(loader.excluded_candidates(state)), &loader.ec_decisions(state));
        let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
        std::fs::create_dir_all("test_transcripts")?;
        let file = File::create(format!("test_transcripts/transcript{}2013.json",state))?;
        serde_json::to_writer_pretty(file,&transcript)?;
        let official_transcript = loader.read_official_dop_transcript(&transcript.metadata)?;
        official_transcript.compare_with_transcript(&transcript.transcript,|tally|tally as f64);
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


}
