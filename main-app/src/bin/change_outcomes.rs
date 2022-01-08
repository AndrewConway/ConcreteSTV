// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::HashSet;
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
    let ballot_types_considered_unverifiable = ["iVote"];
    let ballot_types_considered_unverifiable : HashSet<String> = ballot_types_considered_unverifiable.iter().map(|s|s.to_string()).collect();
    let options1 = ChooseVotesOptions{ allow_atl: true, allow_first_pref: true, allow_verifiable: false, ballot_types_considered_unverifiable:ballot_types_considered_unverifiable.clone() };
    let options2 = ChooseVotesOptions{ allow_atl: true, allow_first_pref: true, allow_verifiable: true, ballot_types_considered_unverifiable:ballot_types_considered_unverifiable.clone() };
    writeln!(summary,"Electorate,Votes,Min Addition,Min Manipulation")?;
    for electorate in &electorates {
        println!("Electorate: {}", electorate);
        // let data = loader.load_cached_data(electorate)?;
        let data = loader.read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<NSWECLocalGov2021>(electorate)?;
        data.print_summary();
        let mut results = find_outcome_changes::<NSWECLocalGov2021>(&data,&options1);
        //let results2 = find_outcome_changes::<NSWECLocalGov2021>(&data,&options2);
        //results.merge(results2);
        results.sort();

        let out = File::create(format!("changes/{}.vchange", electorate))?;
        serde_json::to_writer(out,&results)?;

        let min_add = results.changes.iter().filter( |vc | !vc.requires.changed_ballots).map(|vc| vc.ballots.n).min();
        let min_manipulation = results.changes.iter().filter( |vc | vc.requires.changed_ballots).map(|vc| vc.ballots.n).min();
        writeln!(summary, "{},{},{},{}", electorate, data.num_votes(), min_add.map(|vc| vc.to_string()).unwrap_or("".to_string()), min_manipulation.map(|vc| vc.to_string()).unwrap_or("".to_string()))?;
        summary.flush()?;
    }

    Ok(())
}

