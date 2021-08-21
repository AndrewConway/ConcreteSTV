mod rules;

use clap::{AppSettings, Clap};
use std::path::PathBuf;
use std::fs::File;
use stv::election_data::ElectionData;
use crate::rules::Rules;
use stv::tie_resolution::TieResolutionsMadeByEC;
use std::collections::HashSet;


#[derive(Clap)]
#[clap(version = "0.1", author = "Andrew Conway")]
#[clap(setting = AppSettings::ColoredHelp)]
/// Count STV elections using a variety of rules including good approximations to
/// those used by various electroral commissions on various elections.
struct Opts {
    /// The counting rules to use.
    /// Currently supported AEC2013, AEC2016, AEC2019, Federal
    #[clap(short, long)]
    rules : Rules,

    /// The name of the .stv file to get votes from
    #[clap(parse(from_os_str))]
    votes : PathBuf,

    /// The number of people to elect
    vacancies : usize,

    /// An optional .transcript file to store the output in.
    /// If not specified, defaults to votes_rules.transcript where votes and rules are from above.
    #[clap(short, long,parse(from_os_str))]
    transcript : Option<PathBuf>,


}


fn main() -> anyhow::Result<()> {
    let opt : Opts = Opts::parse();

    let votes : ElectionData = {
        let file = File::open(&opt.votes)?;
        serde_json::from_reader(file)?
    };

    let transcript = opt.rules.count(&votes,opt.vacancies,&HashSet::default(),&TieResolutionsMadeByEC::default());

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
