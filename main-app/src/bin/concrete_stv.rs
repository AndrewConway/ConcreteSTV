// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use clap::{Parser};
use std::path::PathBuf;
use std::fs::File;
use stv::election_data::ElectionData;
use stv::tie_resolution::TieResolutionsMadeByEC;
use std::collections::HashSet;
use stv::ballot_metadata::{NumberOfCandidates, CandidateIndex};
use anyhow::anyhow;
use std::iter::FromIterator;
use main_app::rules::Rules;

#[derive(Parser)]
#[clap(version = "0.1", author = "Andrew Conway", name="ConcreteSTV")]
/// Count STV elections using a variety of rules including good approximations to
/// those used by various electoral commissions on various elections.
struct Opts {
    /// The counting rules to use.
    /// Currently supported AEC2013, AEC2016, AEC2019, Federal
    rules : Rules,

    /// The name of the .stv file to get votes from
    #[clap(parse(from_os_str))]
    votes : PathBuf,

    /// The number of people to elect. If used, overrides the value in the .stv file.
    #[clap(short, long)]
    vacancies : Option<usize>,

    /// An optional .transcript file to store the output in.
    /// If not specified, defaults to votes_rules.transcript where votes and rules are from above.
    #[clap(short, long,parse(from_os_str))]
    transcript : Option<PathBuf>,

    /// An optional list of candidates to exclude. This is a comma separated list of numbers,
    /// starting counting at zero. E.g. --exclude=5,6 would do the count assuming the candidates
    /// with 5 and 6 other candidates listed before them are ineligible.
    #[clap(short, long,use_delimiter=true,require_delimiter=true)]
    exclude : Option<Vec<usize>>,

    /// Whether the status of the count should be printed out to stdout.
    #[clap(long)]
    verbose: bool,

}


fn main() -> anyhow::Result<()> {
    let opt : Opts = Opts::parse();

    let votes : ElectionData = {
        let file = File::open(&opt.votes)?;
        serde_json::from_reader(file)?
    };

    let vacancies=opt.vacancies.map(|n|NumberOfCandidates(n)).or(votes.metadata.vacancies).ok_or_else(||anyhow!("Need to specify number of vacancies"))?;

    let excluded = match &opt.exclude {
        None => Default::default(),
        Some(v) => HashSet::from_iter(v.iter().map(|c|CandidateIndex(*c))),
    };
    let transcript = opt.rules.count(&votes,vacancies,&excluded,&TieResolutionsMadeByEC::default(),opt.verbose);

    let transcript_file = match &opt.transcript {
        None => {
            let votename = opt.votes.file_name().map(|o|o.to_string_lossy()).unwrap_or_default();
            let votename = votename.trim_end_matches(".stv");
            let rulename = opt.rules.to_string();
            let combined = votename.to_string()+"_"+&rulename+".transcript";
            opt.votes.with_file_name(combined)
        }
        Some(tf) => tf.clone(),
    };

    if let Some(parent) = transcript_file.parent() { std::fs::create_dir_all(parent)? }
    serde_json::to_writer(File::create(&transcript_file)?,&transcript)?;

    Ok(())
}
