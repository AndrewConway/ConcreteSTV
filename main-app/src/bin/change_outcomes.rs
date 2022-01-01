// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::HashSet;
use margin::find_outcome_changes::find_outcome_changes;
use nsw::NSWECLocalGov2021;
use nsw::parse_lge::get_nsw_lge_data_loader_2021;
use stv::ballot_metadata::CandidateIndex;
use stv::ballot_paper::{ATL, BTL};
use stv::compare_transcripts::compare_transcripts;
use stv::distribution_of_preferences_transcript::ReasonForCount;
use stv::parse_util::{FileFinder, RawDataSource};
use stv::preference_distribution::distribute_preferences;

fn main() -> anyhow::Result <()> {

    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_nsw_lge_data_loader_2021(&finder)?;
    println!("Made loader");
    let electorates = loader.all_electorates();
    for electorate in &electorates {
        println!("Electorate: {}", electorate);
        let data = loader.load_cached_data(electorate)?;
        find_outcome_changes::<NSWECLocalGov2021>(&data);
    }

    Ok(())
}

