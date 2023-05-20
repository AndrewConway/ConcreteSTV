// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Compare the effects of different rules on federal elections.

use stv::compare_rules::CompareRules;
use federal::{FederalRulesUsed2013, FederalRulesUsed2016, FederalRulesUsed2019, FederalRulesPre2021};
use federal::parse::FederalDataLoader;
use stv::election_data::ElectionData;
use stv::parse_util::{FileFinder, RawDataSource};


fn main()  -> anyhow::Result<()> {

    fn all_states_data(loader:FederalDataLoader) -> Vec<anyhow::Result<ElectionData>> {
        loader.all_electorates().iter().map(move |state|loader.load_cached_data(state)).collect::<Vec<_>>()
    }
    let loader13 = federal::parse::get_federal_data_loader_2013(&FileFinder::find_ec_data_repository());
    let loader14 = federal::parse::get_federal_data_loader_2014(&FileFinder::find_ec_data_repository());
    let loader16 = federal::parse::get_federal_data_loader_2016(&FileFinder::find_ec_data_repository());
    let loader19 = federal::parse::get_federal_data_loader_2019(&FileFinder::find_ec_data_repository());
    let mut data = vec![];
    data.append(&mut all_states_data(loader13));
    data.append(&mut all_states_data(loader14));
    data.append(&mut all_states_data(loader16));
    data.append(&mut all_states_data(loader19));
    let comparer = CompareRules{ dir: "Comparison/Federal".to_string() };
    // comparer.compute_dataset::<usize,FederalRulesUsed2013,FederalRulesUsed2016,FederalRulesUsed2019,FederalRules>(&data)?;

    comparer.compare_datasets::<usize,FederalRulesUsed2013,FederalRulesUsed2016,FederalRulesUsed2019, FederalRulesPre2021,_>(data.into_iter())?;


    Ok(())
}