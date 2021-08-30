// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Parse Election Commmission data and produce .stv files.

use clap::{AppSettings, Clap};
use main_app::ec_data_source::ECDataSource;
use std::path::PathBuf;
use std::fs::File;
use std::io::stdout;
use stv::parse_util::FileFinder;

#[derive(Clap)]
#[clap(version = "0.1", author = "Andrew Conway", name="ConcreteSTV")]
#[clap(setting = AppSettings::ColoredHelp)]
/// Produce a .stv file with actual election data from a download from an Electoral Commission.
struct Opts {
    /// The election to load data from.
    /// Currently accepted AEC2013, AEC2016, AEC2019
    election : ECDataSource,

    /// The electorate to load data for the given election.
    /// E.g. VIC for AEC2013. If you enter an invalid electorate, a list of valid electorates will be provided.
    electorate : String,

    /// An optional output file. If not specified, stdout is used.
    /// It is strongly recommended that this be used as stdout is also used for other information.
    #[clap(short, long,parse(from_os_str))]
    out : Option<PathBuf>,

    /// An optional list of candidate numbers (starting counting at 0) to mark as to be excluded.
    #[clap(short, long,use_delimiter=true,require_delimiter=true)]
    exclude : Option<Vec<usize>>,

    /// An optional directory to use for finding raw data files.
    /// If not specified, the current directory will be used.
    /// Files will be searched in this directory, and in an EC/year specific subdirectory (e.g Federal/2013/)
    src : Option<FileFinder>
}


fn main() -> anyhow::Result<()> {
    let opt: Opts = Opts::parse();
    let finder : FileFinder = opt.src.clone().unwrap_or_else(||FileFinder::default());
    let res = opt.election.load(&opt.electorate,&finder)?;


    let out : Box<dyn std::io::Write> = match &opt.out {
        None => Box::new(stdout()),
        Some(path) => Box::new(File::create(path)?),
    };
    serde_json::to_writer(out,&res)?;
    Ok(())
}