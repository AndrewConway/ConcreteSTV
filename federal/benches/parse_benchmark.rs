use std::collections::HashSet;
use criterion::{criterion_group, criterion_main, Criterion};
use federal::FederalRulesUsed2019;
use federal::parse::get_federal_data_loader_2019;
use stv::distribution_of_preferences_transcript::{Transcript};
use stv::election_data::ElectionData;
use stv::parse_util::{FileFinder, RawDataSource};
use stv::preference_distribution::distribute_preferences;
use stv::tie_resolution::TieResolutionsMadeByEC;

fn load2019(state:&str) -> anyhow::Result<ElectionData> {
    let loader = get_federal_data_loader_2019(&FileFinder::find_ec_data_repository());
    loader.read_raw_data(state)
}

fn count2019(data:&ElectionData) -> Transcript<usize> {
    distribute_preferences::<FederalRulesUsed2019>(&data, data.metadata.vacancies.unwrap(), &HashSet::default(), &TieResolutionsMadeByEC::default(),None,false)
}

fn load_tas2019(c: &mut Criterion) {
    c.bench_function("Parse raw data TAS 2019", |b| b.iter(|| load2019("TAS")));
}

fn count_tas2019(c: &mut Criterion) {
    let data = load2019("TAS").unwrap();
    c.bench_function("Count TAS 2019", |b| b.iter(|| count2019(&data)));
}

criterion_group!(benches, load_tas2019,count_tas2019);
criterion_main!(benches);