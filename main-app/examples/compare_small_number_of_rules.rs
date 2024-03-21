// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Compare the effects of different rules on NSW elections 2021.

use act::ACTPre2020;
use stv::compare_rules::CompareRules;
use federal::FederalRulesPre2021;
use nsw::{NSWECLocalGov2021, NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation};
use nsw::parse_lge::get_nsw_lge_data_loader_2021;
use stv::parse_util::{FileFinder, RawDataSource};

/// Compare various rules for the NSW2021 elections.
fn main()  -> anyhow::Result<()> {
    let loader = get_nsw_lge_data_loader_2021(&FileFinder::find_ec_data_repository())?;

    let electorates = loader.all_electorates();
    let iterator = electorates.iter().filter(|e|!e.ends_with(" Mayoral")).map(|e|loader.load_cached_data(e));

    let comparer = CompareRules{ dir: "Comparison/NSW2021".to_string() };
    comparer.compare_datasets::<usize,NSWECLocalGov2021,NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation,ACTPre2020, FederalRulesPre2021,_>(iterator)?;

    Ok(())
}