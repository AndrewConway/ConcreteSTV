// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use clap::{Parser};
use std::path::PathBuf;
use std::fs::File;
use main_app::ModifyStvFileOptions;
use main_app::rules::Rules;

#[derive(Parser)]
#[clap(version = "0.2", author = "Andrew Conway", name="ConcreteSTV")]
/// Count STV elections using a variety of rules including good approximations to
/// those used by various electoral commissions on various elections.
struct Opts {
    /// The counting rules to use.
    /// Currently supported AEC2013, AEC2016, AEC2019, FederalPre2021, FederalPost2021, FederalPost2021Manual, ACTPre2020, ACT2020, ACT2021, NSWLocalGov2021, NSWECLocalGov2021
    rules : Rules,

    /// The name of the .stv (or .vchange) file to get votes from
    #[clap(parse(from_os_str))]
    votes : PathBuf,

    /// An optional .transcript file to store the output in.
    /// If not specified, defaults to votes_rules.transcript where votes and rules are from above.
    #[clap(short, long,parse(from_os_str))]
    transcript : Option<PathBuf>,

    #[clap(flatten)]
    input_options : ModifyStvFileOptions,

    /// Whether the status of the count should be printed out to stdout.
    #[clap(long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let opt : Opts = Opts::parse();

    let votes = opt.input_options.get_data(&opt.votes,opt.verbose)?;
    let transcript_file = opt.input_options.result_file_name(&opt.votes,opt.transcript.as_ref(),".transcript",&opt.rules);

    let transcript = opt.rules.count_simple(&votes,opt.verbose)?;

    if let Some(parent) = transcript_file.parent() { std::fs::create_dir_all(parent)? }
    serde_json::to_writer(File::create(&transcript_file)?,&transcript)?;

    Ok(())
}
