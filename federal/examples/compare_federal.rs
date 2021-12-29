// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Compare the effects of different rules on federal elections.

use stv::compare_rules::CompareRules;
use federal::{FederalRulesUsed2013, FederalRulesUsed2016, FederalRulesUsed2019, FederalRules};
use stv::parse_util::FileFinder;


fn main()  -> anyhow::Result<()> {

    let loader13 = federal::parse::get_federal_data_loader_2013(&FileFinder::find_ec_data_repository());
    let loader16 = federal::parse::get_federal_data_loader_2016(&FileFinder::find_ec_data_repository());
    let loader19 = federal::parse::get_federal_data_loader_2019(&FileFinder::find_ec_data_repository());
    let iterator = loader13.all_states_data().chain(loader16.all_states_data()).chain(loader19.all_states_data());
    let comparer = CompareRules{ dir: "Comparison/Federal".to_string() };
    // comparer.compute_dataset::<usize,FederalRulesUsed2013,FederalRulesUsed2016,FederalRulesUsed2019,FederalRules>(&data)?;

    comparer.compare_datasets::<usize,FederalRulesUsed2013,FederalRulesUsed2016,FederalRulesUsed2019,FederalRules,_>(iterator)?;


    Ok(())
}