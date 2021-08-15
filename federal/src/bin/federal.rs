use federal::parse::{get_federal_data_loader_2016, get_federal_data_loader_2019};
use stv::preference_distribution::{distribute_preferences};
use std::collections::HashSet;
use federal::FederalRules;
use std::fs::File;
use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;

fn main()  -> anyhow::Result<()> {
    let loader = get_federal_data_loader_2016();
    //let metadata = loader.read_raw_metadata("ACT")?;
    //println!("{:#?}",metadata);
    let data = loader.load_cached_data("ACT")?;
    data.save_to_cache()?;
    data.print_summary();
    let transcript = distribute_preferences::<FederalRules>(&data,2,&HashSet::default());
    let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
    let file = File::create("transcript.json")?;
    serde_json::to_writer_pretty(file,&transcript)?;
    Ok(())
}