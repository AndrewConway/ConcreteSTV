// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::fs::File;
use std::path::PathBuf;
use anyhow::anyhow;
use clap::{Parser};
use main_app::{ChangeOptions, ModifyStvFileOptions};
use main_app::rules::Rules;

#[derive(Parser)]
#[clap(version = "0.2", author = "Andrew Conway and Vanessa Teague", name="ConcreteSTV")]
/// Find small changes to the votes that change the outcome of the election.
/// This uses heuristics to try some likely possibilities, and verifies that they work.
/// It will not necessarily find the smallest possible change, but can be
/// used as an upper bound on the margin for the election.
///
/// This program basically takes a .stv file as input, and produces a .vchange file
/// as output. It is possible to use a .vchange file as input instead of a .stv file;
/// this allows searching for manipulations on top of manipulations.
///
/// This will not reliably work with ticket elections (e.g. Federal 2013 and earlier) if ATL modifications are allowed.
struct Opts {
    /// The counting rules to use.
    /// Currently supported AEC2013, AEC2016, AEC2019, Federal, ACTPre2020, ACT2020, ACT2021, NSWLocalGov2021, NSWECLocalGov2021
    rules : Rules,

    /// The name of the .stv (or .vchange) file to get votes from
    #[clap(parse(from_os_str))]
    votes : PathBuf,

    /// An optional .vchange file to store the output in.
    /// If not specified, defaults to votes_rules.vchange where votes and rules are from above.
    #[clap(short, long,parse(from_os_str))]
    out : Option<PathBuf>,

    #[clap(flatten)]
    change_options : ChangeOptions,

    #[clap(flatten)]
    input_options : ModifyStvFileOptions,

    /// Whether the status of the analysis should be printed out to stdout.
    #[clap(long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let opt : Opts = Opts::parse();

    let mut votes = opt.input_options.get_data(&opt.votes,opt.verbose)?;

    let result_file = opt.input_options.result_file_name(&opt.votes,opt.out.as_ref(),".vchange",&opt.rules);

    // make sure the default elected people are correct.
    let normal_elected_transcript = opt.rules.count(&votes,votes.metadata.vacancies.ok_or_else(||anyhow!("Need to specify number of vacancies"))?,&votes.metadata.excluded.iter().cloned().collect(),&votes.metadata.tie_resolutions,None,false);
    votes.metadata.results=Some(normal_elected_transcript.elected().clone());


    let changes = opt.rules.find_changes(&votes,&opt.change_options,opt.verbose)?;

    if let Some(parent) = result_file.parent() { std::fs::create_dir_all(parent)? }
    serde_json::to_writer(File::create(&result_file)?,&changes)?;

    Ok(())
}

