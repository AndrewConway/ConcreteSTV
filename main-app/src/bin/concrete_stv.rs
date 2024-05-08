// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use clap::{Parser};
use std::path::PathBuf;
use std::fs::File;
use main_app::ModifyStvFileOptions;
use main_app::rules::Rules;
use stv::extract_votes_in_pile::ExtractionRequest;
use stv::random_util::Randomness;

#[derive(Parser)]
#[clap(version = "0.3", author = "Andrew Conway", name="ConcreteSTV")]
/// Count STV elections using a variety of rules including good approximations to
/// those used by various electoral commissions on various elections.
struct Opts {
    /// The counting rules to use.
    /// Currently supported AEC2013, AEC2016, AEC2019, FederalPre2021, FederalPost2021, FederalPost2021Manual, ACTPre2020, ACT2020, ACT2021, NSWLocalGov2021, NSWECLocalGov2021, NSWECRandomLGE2012, NSWECRandomLGE2016, NSWECRandomLGE2017, NSWECRandomLC2015, NSWECRandomLC2019, Vic2018, WA2008
    rules : Rules,

    /// The name of the .stv (or .vchange) file to get votes from
    #[clap(value_parser)]
    votes : PathBuf,

    /// An optional .transcript file to store the output in.
    /// If not specified, defaults to votes_rules.transcript where votes and rules are from above.
    #[clap(short, long,value_parser)]
    transcript : Option<PathBuf>,

    #[clap(flatten)]
    input_options : ModifyStvFileOptions,

    /// Whether the status of the count should be printed out to stdout.
    #[clap(long)]
    verbose: bool,

    /// How random ties are done. If specified, the seed for a pseudo random number generator.
    /// If not specified, then reverse donkey vote is used.
    #[clap(short, long,value_parser)]
    seed : Option<u64>,

    /// It is possible to extract the particular votes at some point in the transcript. The
    /// general format for this is --extract what_to_extract;what_to_do_with_it, where
    ///
    /// what_to_extract can currently only be UsedToElectACT:candidate_number where candidate_number
    /// is an integer 0 to the number of candidates-1 and will extract the votes used to elect the
    /// candidate according to the ACT casual vacancies legislation.
    ///
    /// what_to_do_with_it can currently only be file:file_name where file_name is the name of a .stv
    /// file that you want to store the extracted votes in.
    #[clap(long)]
    extract : Vec<ExtractionRequest>,

    /// You can list all the votes that are in a candidate's pile at every count for every candidate
    /// in the output transcript. This will usually make a very big file and be very slow! Default is to
    /// not do this, flag makes it be done.
    #[clap(long)]
    include_list_of_votes_in_transcript:bool,
}

fn main() -> anyhow::Result<()> {
    let opt : Opts = Opts::parse();

    let votes = opt.input_options.get_data(&opt.votes,opt.verbose)?;
    let transcript_file = opt.input_options.result_file_name(&opt.votes,opt.transcript.as_ref(),".transcript",&opt.rules);
    let mut randomness : Randomness = opt.seed.into();
    let transcript = opt.rules.count_simple(&votes,opt.verbose,&mut randomness,&opt.extract,opt.include_list_of_votes_in_transcript)?;

    if let Some(parent) = transcript_file.parent() { std::fs::create_dir_all(parent)? }
    serde_json::to_writer(File::create(&transcript_file)?,&transcript)?;

    Ok(())
}
