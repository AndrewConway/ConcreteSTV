use federal::parse::{get_federal_data_loader_2016, get_federal_data_loader_2019, get_federal_data_loader_2013};
use stv::preference_distribution::{distribute_preferences};
use std::collections::HashSet;
use federal::FederalRules;
use std::fs::File;
use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
use stv::tie_resolution::TieResolutionsMadeByEC;
use std::io::stdout;

fn main()  -> anyhow::Result<()> {
    let loader = get_federal_data_loader_2013();
    //let metadata = loader.read_raw_metadata("ACT")?;
    //serde_json::to_writer_pretty(stdout(),&metadata)?;
    //println!("{:#?}",metadata);

    let data = loader.load_cached_data("ACT")?;
    data.print_summary();
    let transcript = distribute_preferences::<FederalRules>(&data, 2, &HashSet::default(), &TieResolutionsMadeByEC::default());
    let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
    let file = File::create("transcript.json")?;
    serde_json::to_writer_pretty(file,&transcript)?;
    //let official_transcript = loader.read_official_dop_transcript(&transcript.metadata)?;
    //official_transcript.compare_with_transcript(&transcript.transcript,|tally|tally as f64);
    Ok(())
}