use stv::preference_distribution::{distribute_preferences};
use std::collections::HashSet;
use federal::FederalRules;
use std::fs::File;
use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
use stv::tie_resolution::TieResolutionsMadeByEC;
use stv::ballot_metadata::NumberOfCandidates;
use stv::parse_util::{FileFinder, RawDataSource};

fn main()  -> anyhow::Result<()> {
    let loader = federal::parse::get_federal_data_loader_2013(&FileFinder::find_ec_data_repository());
    //let metadata = loader.read_raw_metadata("ACT")?;
    //serde_json::to_writer_pretty(std::io::stdout(),&metadata)?;
    //println!("{:#?}",metadata);

    let data = loader.load_cached_data("ACT")?;
    data.print_summary();
    let transcript = distribute_preferences::<FederalRules>(&data, NumberOfCandidates(2), &HashSet::default(), &TieResolutionsMadeByEC::default());
    let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
    let file = File::create("transcript.json")?;
    serde_json::to_writer_pretty(file,&transcript)?;
    //let official_transcript = loader.read_official_dop_transcript(&transcript.metadata)?;
    //official_transcript.compare_with_transcript(&transcript.transcript,|tally|tally as f64);
    Ok(())
}