// Copyright 2024 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use clap::{Parser};
use std::path::PathBuf;
use main_app::compare_different_rules::RulesComparisonGroups;
use main_app::ModifyStvFileOptions;
use main_app::rules::Rules;

#[derive(Parser)]
#[clap(version = "0.3", author = "Andrew Conway", name="ConcreteSTV")]
/// Count STV elections using a variety of rules and compare the results
struct Opts {
    /*
    /// The counting rules to use.
    /// Currently supported AEC2013, AEC2016, AEC2019, FederalPre2021, FederalPost2021, FederalPost2021Manual, ACTPre2020, ACT2020, ACT2021, NSWLocalGov2021, NSWECLocalGov2021, NSWECRandomLGE2012, NSWECRandomLGE2016, NSWECRandomLGE2017, NSWECRandomLC2015, NSWECRandomLC2019, Vic2018, WA2008
    rules : Rules,
*/

    /// The name of the .stv (or .vchange) file to get votes from
    #[clap(value_parser)]
    votes : PathBuf,

    #[clap(flatten)]
    input_options : ModifyStvFileOptions,

    /// Whether the status of the output should be JSON rather than human readable text.
    #[clap(long)]
    json: bool,

    /// How detailed the human readable output is. 1 means just consider who is elected. 2 means also consider order. 3 means also consider transcript (not implemented yet).
    #[clap(long)]
    detail: Option<usize>,

}

fn main() -> anyhow::Result<()> {
    let opt : Opts = Opts::parse();

    let votes = opt.input_options.get_data(&opt.votes,false)?;
    let rules = vec![
        Rules::AEC2013,Rules::AEC2016,Rules::AEC2019,Rules::FederalPre2021,Rules::FederalPost2021,Rules::FederalPost2021Manual,
        Rules::ACTPre2020,Rules::ACT2020,Rules::ACT2021,
        Rules::NSWLocalGov2021,Rules::NSWECLocalGov2021,
        Rules::Vic2018,
        Rules::WA2008];
    let comparison = RulesComparisonGroups::create(&votes,&rules)?;
    if opt.json { println!("{}",serde_json::to_string(&comparison)?) }
    else { println!("{:.*}",opt.detail.unwrap_or(3),comparison)}
    Ok(())
}
