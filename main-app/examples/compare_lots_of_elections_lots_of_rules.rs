// Copyright 2024 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Compare the effects of different rules on lots of elections.

use act::parse::ACTDataSource;
use federal::parse::FederalDataSource;
use main_app::compare_different_rules::RulesComparisonGroups;
use main_app::rules::Rules;
use nsw::parse_lc::NSWLCDataSource;
use nsw::parse_lge::NSWLGEDataSource;
use stv::datasource_description::ElectionDataSource;
use stv::parse_util::{FileFinder};

fn get_all_elections() -> Vec<Box<dyn ElectionDataSource+Sync+Send>>{
    vec![Box::new(ACTDataSource{}),Box::new(FederalDataSource{}),Box::new(NSWLCDataSource{}),Box::new(NSWLGEDataSource{})]
}

/// Compare various rules for elections in get_all_elections()
fn main()  -> anyhow::Result<()> {

    let rules = vec![
        Rules::AEC2013,Rules::AEC2016,Rules::AEC2019,Rules::FederalPre2021,Rules::FederalPost2021,Rules::FederalPost2021Manual,
        Rules::ACTPre2020,Rules::ACT2020,Rules::ACT2021,
        Rules::NSWLocalGov2021,Rules::NSWECLocalGov2021,
        Rules::Vic2018,
        Rules::WA2008];


    for source in get_all_elections() {
        for year in source.years() {
            println!("Trying {} {}",source.name(),&year);
            let loader = source.get_loader_for_year(&year,&FileFinder::find_ec_data_repository())?;
            for electorate in loader.all_electorates() {
                let votes = loader.load_cached_data(&electorate)?;
                let comparison = RulesComparisonGroups::create(&votes,&rules)?;
                if comparison.has_different_winners() { println!("Different winners for {} : {}",electorate,comparison)}
                else if comparison.has_different_orders() { println!("Different order for {} : {}",electorate,comparison)}
                else { println!("Same winners for {}",electorate)}
            }
        }
    }
    Ok(())
}