// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use nsw::nsw_random_rules::NSWECRandomLC2019;
use nsw::parse_lc::get_nsw_lc_data_loader_2023;
use nsw::run_election_multiple_times::PossibleResults;
use stv::parse_util::{FileFinder, RawDataSource};

/// Run an election 10 000 times and see if the same people win.
fn main() -> anyhow::Result<()> {
    let finder = FileFinder::find_ec_data_repository();
    let loader = get_nsw_lc_data_loader_2023(&finder)?;
    let data = loader.read_raw_data("")?;
    data.print_summary();
    let results = PossibleResults::new_from_runs_multithreaded::<NSWECRandomLC2019>(&data,10000,32);
    results.print_table_results(&data.metadata);
    let official = loader.read_official_dop_transcript(&data.metadata).unwrap();
    for elected in official.all_elected() {
        assert!(results.is_close_to_expected_prob_winning(elected,1.0));
    }
    Ok(())
}