// Copyright 2021-2022 Andrew Conway.
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
use margin::record_changes::ElectionChanges;

#[derive(Parser)]
#[clap(version = "0.2", author = "Andrew Conway", name="ConcreteSTV")]
/// Count STV elections using a variety of rules including good approximations to
/// those used by various electoral commissions on various elections.
struct Opts {
    /// The counting rules to use.
    /// Currently supported AEC2013, AEC2016, AEC2019, Federal, ACTPre2020, ACT2020, ACT2021, NSWLocalGov2021, NSWECLocalGov2021
    rules : Rules,

    /// The name of the .stv (or .vchange) file to get votes from
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
    /// with 5 and 6 other candidates listed before them are ineligible. If specified, this overrides
    /// any candidates specified as excluded in the .stv file.
    #[clap(short, long,use_delimiter=true,require_delimiter=true)]
    exclude : Option<Vec<usize>>,

    /// Whether the status of the count should be printed out to stdout.
    #[clap(long)]
    verbose: bool,

    /// If a .vchange file is used instead of a .stv file, one of the vote manipulations in it can be applied first, specified here. 1 means the first one in the file, 2 the second, etc.
    /// This can be used to prove an upper bound on the margin.
    #[clap(short, long)]
    modification : Option<usize>,

    /// Specified resolution of ties that need to be resolved by the electoral commission, often by lot.
    ///
    /// ConcreteSTV, by default, chooses in favour of the candidate in a worse donkey-vote position (higher indices favoured).
    /// This is overriden by explicit tie resolutions specified when creating the .stv file.
    /// This flag overrides both of these.
    ///
    /// You can override this by specifying a list of candidate indices (starting counting at 0) to favour in said priority order.
    /// For example in a tie resolved between candidates 27 and 43, ConcreteSTV would favour 43 by default. Enter `--tie 43,27` to
    /// indicate that 27 should be favoured over 43 in a decision between them.
    /// This flag may be used multiple times for multiple tie resolutions.
    #[clap(long,parse(try_from_str=main_app::try_parse_candidate_list))]
    tie : Vec<Vec<CandidateIndex>>,

}

fn main() -> anyhow::Result<()> {
    let opt : Opts = Opts::parse();

    let votes : ElectionData = {
        let file = File::open(&opt.votes)?;
        if opt.votes.as_os_str().to_string_lossy().ends_with(".vchange") {
            let vchange : ElectionChanges<f64> = serde_json::from_reader(file)?; // Everything so far will parse as f64, and the values are not used in way here so accuracy is irrelevant.
            if let Some(modification_number_1_based) = opt.modification {
                if modification_number_1_based>vchange.changes.len() || modification_number_1_based==0 {
                    return Err(anyhow!("Modification number {} should be between 1 and {} (the number of modifications in that file)",modification_number_1_based,vchange.changes.len()));
                }
                vchange.changes[modification_number_1_based-1].ballots.apply_to_votes(&vchange.original,opt.verbose)
            } else { vchange.original }
        } else {
            serde_json::from_reader(file)?
        }
    };

    let vacancies=opt.vacancies.map(|n|NumberOfCandidates(n)).or(votes.metadata.vacancies).ok_or_else(||anyhow!("Need to specify number of vacancies"))?;

    let excluded = match &opt.exclude {
        None => Default::default(),
        Some(v) => HashSet::from_iter(v.iter().map(|c|CandidateIndex(*c))),
    };

    let ec_resolutions = if opt.tie.is_empty() { votes.metadata.tie_resolutions.clone() } else { TieResolutionsMadeByEC{ tie_resolutions: opt.tie }};

    let transcript = opt.rules.count(&votes,vacancies,&excluded,&ec_resolutions,opt.verbose);

    let transcript_file = match &opt.transcript {
        None => {
            let votename = opt.votes.file_name().map(|o|o.to_string_lossy()).unwrap_or_default();
            let votename = votename.trim_end_matches(".stv").trim_end_matches(".vchange");
            let modname = if let Some(modification) = opt.modification { modification.to_string()+"_"} else {"".to_string()};
            let rulename = opt.rules.to_string();
            let combined = votename.to_string()+"_"+&modname+&rulename+".transcript";
            opt.votes.with_file_name(combined)
        }
        Some(tf) => tf.clone(),
    };

    if let Some(parent) = transcript_file.parent() { std::fs::create_dir_all(parent)? }
    serde_json::to_writer(File::create(&transcript_file)?,&transcript)?;

    Ok(())
}
