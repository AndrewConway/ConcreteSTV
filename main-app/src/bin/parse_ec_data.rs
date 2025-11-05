// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Parse Election Commmission data and produce .stv files.

use clap::Parser;
use main_app::ec_data_source::ECDataSource;
use std::path::PathBuf;
use std::fs::File;
use std::io::stdout;
use stv::ballot_metadata::CandidateIndex;
use stv::parse_util::FileFinder;
use stv::tie_resolution::{TieResolutionAtom, TieResolutionsMadeByEC};

#[derive(Parser)]
#[clap(version = "0.2", author = "Andrew Conway", name="ConcreteSTV")]
/// Produce a .stv file with actual election data from a download from an Electoral Commission.
struct Opts {
    /// The election to load data from.
    /// Currently accepted AEC2013, AEC2016, AEC2019, AEC2022, ACT2008, ACT2012, ACT2016, ACT2020, NSWLG2021, NSWLG2024, VIC2014, VIC2018, VIC2022,
    election : ECDataSource,

    /// The electorate to load data for the given election.
    /// E.g. VIC for AEC2013. If you enter an invalid electorate, a list of valid electorates will be provided.
    electorate : String,

    /// An optional output file. If not specified, stdout is used.
    /// It is strongly recommended that this be used as stdout is also used for other information.
    #[clap(short, long,value_parser)]
    out : Option<PathBuf>,

    /// An optional list of candidate numbers (starting counting at 0) to mark as to be excluded (ineligible).
    /// Separate with commas.
    #[clap(short, long,value_delimiter=',')]
    exclude : Option<Vec<CandidateIndex>>,

    /// An optional directory to use for finding raw data files.
    /// If not specified, the current directory will be used.
    /// Files will be searched in this directory, and in an EC/year specific subdirectory (e.g Federal/2013/)
    src : Option<FileFinder>,

    /// Specified resolution of ties that need to be resolved by the electoral commission, often by lot.
    ///
    /// This flag overrides ConcreteSTV's default of choosing in favour of the candidate in a worse donkey-vote position (higher indices favoured).
    /// You can override this by specifying a list of candidate indices (starting counting at 0) to favour in said priority order.
    /// For example in a tie resolved between candidates 27 and 43, ConcreteSTV would favour 43 by default. Enter `--tie 43,27` to
    /// indicate that 27 should be favoured over 43 in a decision between them.
    /// This flag may be used multiple times for multiple tie resolutions.
    #[clap(long,value_parser=main_app::try_parse_candidate_list)]
    tie : Vec<TieResolutionAtom>,
}


fn main() -> anyhow::Result<()> {
    let opt: Opts = Opts::parse();
    let finder : FileFinder = opt.src.clone().unwrap_or_else(||FileFinder::default());
    let mut res = opt.election.load(&opt.electorate,&finder)?;
    if !opt.tie.is_empty() {
        res.metadata.tie_resolutions=TieResolutionsMadeByEC{ tie_resolutions: opt.tie };
    }
    if let Some(exclude) = opt.exclude {
        res.metadata.excluded=exclude; //.iter().map(|&e|CandidateIndex(e)).collect();
    }


    let out : Box<dyn std::io::Write> = match &opt.out {
        None => Box::new(stdout()),
        Some(path) => Box::new(File::create(path)?),
    };
    serde_json::to_writer(out,&res)?;
    Ok(())
}