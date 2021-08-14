use federal::parse::{get_federal_data_loader_2016, get_federal_data_loader_2019};

fn main()  -> anyhow::Result<()> {
    let loader = get_federal_data_loader_2016();
    let metadata = loader.read_raw_metadata("ACT")?;
    println!("{:#?}",metadata);
    let data = loader.read_raw_data("ACT")?;
    data.print_summary();
    Ok(())
}