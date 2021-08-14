use federal::parse::{get_federal_data_loader_2016, get_federal_data_loader_2019};
use stv::preference_distribution::{distribute_preferences};
use std::collections::HashSet;
use stv::ballot_pile::DoNotSplitByCountNumber;
use federal::FederalRules;

fn main()  -> anyhow::Result<()> {
    let loader = get_federal_data_loader_2016();
    //let metadata = loader.read_raw_metadata("ACT")?;
    //println!("{:#?}",metadata);
    let data = loader.read_raw_data("ACT")?;
    data.print_summary();
    distribute_preferences::<DoNotSplitByCountNumber,usize,FederalRules>(&data,2,&HashSet::default());
    Ok(())
}