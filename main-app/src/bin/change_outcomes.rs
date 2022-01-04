// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::fs::{create_dir_all, File};
use std::io::Write;
use margin::choose_votes::ChooseVotesOptions;
use margin::find_outcome_changes::find_outcome_changes;
use nsw::NSWECLocalGov2021;
use nsw::parse_lge::get_nsw_lge_data_loader_2021;
use stv::parse_util::{FileFinder, RawDataSource};

fn main() -> anyhow::Result <()> {

    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_nsw_lge_data_loader_2021(&finder)?;
    println!("Made loader");
    let electorates = loader.all_electorates();

    create_dir_all("changes")?;
    let mut summary = File::create("changes/summary.csv")?;
    for electorate in &electorates {
        println!("Electorate: {}", electorate);
        let data = loader.load_cached_data(electorate)?;
        let results = find_outcome_changes::<NSWECLocalGov2021>(&data, ChooseVotesOptions{ allow_atl: true, allow_first_pref: true });

        let out = File::create(format!("changes/{}.vchange", electorate))?;
        serde_json::to_writer(out,&results)?;

        let min_add = results.changes.iter().filter( |vc | !vc.requires.changed_ballots).map(|vc| vc.ballots.n).min();
        let min_manipulation = results.changes.iter().filter( |vc | vc.requires.changed_ballots).map(|vc| vc.ballots.n).min();
        write!(summary, "{},{},{},{}", electorate, data.num_votes(), min_add.map(|vc| vc.to_string()).unwrap_or("".to_string()), min_manipulation.map(|vc| vc.to_string()).unwrap_or("".to_string()))?;
        summary.flush()?;
    }

    Ok(())
}

