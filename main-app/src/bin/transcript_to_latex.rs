
use clap::{AppSettings, Clap};
use std::path::PathBuf;
use std::fs::File;
use stv::election_data::ElectionData;
//use anyhow::anyhow;
use stv::distribution_of_preferences_transcript::{TranscriptWithMetadata, ReasonForCount};
use stv::ballot_metadata::CandidateIndex;

#[derive(Clap)]
#[clap(version = "0.1", author = "Andrew Conway")]
#[clap(setting = AppSettings::ColoredHelp)]
/// Convert a .stv file or .transcript file to a LaTeX table.
struct Opts {
    /// The name of the .stv or transcript file to convert to latex.
    #[clap(parse(from_os_str))]
    file : PathBuf,

    /// An optional output file. If not specified, stdout is used.
    #[clap(short, long,parse(from_os_str))]
    out : Option<PathBuf>,

    /// An optional list of candidates
    /// raw(use_delimiter = "true")
    #[clap(short, long,use_delimiter=true)]
    candidates : Option<Vec<usize>>,
}

fn possibly_blank(t:usize) -> String {
    if t==0 { "".to_string() }
    else { t.to_string() }
}
fn possibly_blank_tally(t_old:usize,t_new:usize) -> String {
    if t_old==t_new { "".to_string() }
    else if t_old < t_new { "+".to_string()+&(t_new-t_old).to_string() }
    else  { "-".to_string()+&(t_old-t_new).to_string() }
}

fn main() -> anyhow::Result<()> {
    let opt : Opts = Opts::parse();

    let use_candidate = |c:CandidateIndex|{ opt.candidates.is_none() || opt.candidates.as_ref().unwrap().contains(&c.0)};

    if let Ok(votes) = { let file = File::open(&opt.file)?; serde_json::from_reader::<_,ElectionData>(file) } {
        let metadata = &votes.metadata;
        println!("{}",r"\begin{tabular}{|l|l|}");
        println!("{}",r"\textbf{Preference List} & \textbf{times used} \\ \hline");
        for btl in &votes.btl {
            println!("{} & {} \\\\",btl.candidates.iter().map(|c|metadata.candidate(*c).name.clone()).collect::<Vec<_>>().join(", "),btl.n);
        }
        println!("{}",r"\end{tabular}");
        // we have some votes, make a latex table out of it.
    } else if let Ok(transcript) = { let file = File::open(&opt.file)?; serde_json::from_reader::<_,TranscriptWithMetadata<usize>>(file) } {
        let metadata = &transcript.metadata;
        let use_exhausted : bool = opt.candidates.is_none() && transcript.transcript.counts.iter().any(|v|v.status.tallies.exhausted>0 || v.status.papers.exhausted.0>0);
        let use_rounding : bool = opt.candidates.is_none() && transcript.transcript.counts.iter().any(|v|v.status.tallies.rounding>0 );
        let mut heading_justifications = vec!["r|"];
        let mut headings = vec!["Count"];
        for i in 0..metadata.candidates.len() {
            if use_candidate(CandidateIndex(i)) {
                heading_justifications.push("r");
                headings.push(&metadata.candidates[i].name);
            }
        }
        if use_exhausted {
            heading_justifications.push("r");
            headings.push("Exhausted");
        }
        if use_rounding {
            heading_justifications.push("r");
            headings.push("Rounding");
        }
        if opt.candidates.is_none() {
            heading_justifications.push("|l");
            headings.push("Transfer Value");
            heading_justifications.push("l");
            headings.push("Count action");
        }
        println!("{}{}{}",r"\begin{tabular}{",heading_justifications.join(""),"}");
        println!("{}{}",headings.join(" & "),r" \\ \hline");
        // we have some votes, make a latex table out of it.
        for count_no in 0..transcript.transcript.counts.len() {
            let count = &transcript.transcript.counts[count_no];
            let status = &count.status;
            if count_no>0 {
                let prev_status = &transcript.transcript.counts[count_no-1].status;
                // TODO implement Delta lines
            }
            let mut line = (count_no+1).to_string();
            for i in 0..metadata.candidates.len() {
                if use_candidate(CandidateIndex(i)) {
                    line+=" & ";
                    if count.elected.iter().any(|e|e.who.0==i) { line+=r"\bigstar " }
                    else if count.not_continuing.contains(&CandidateIndex(i)) { line+=r"\downarrow " }
                    line+=&possibly_blank(status.tallies.candidate[i]);
                }
            }
            if use_exhausted {
                line+=" & ";
                line+=&possibly_blank(status.tallies.exhausted);
            }
            if use_rounding {
                line+=" & ";
                line+=&possibly_blank(status.tallies.rounding);
            }
            if opt.candidates.is_none() {
                line+=" & ";
                if let Some(tv) = count.portion.transfer_value.as_ref() {
                    line+=&tv.to_string();
                }
                line+=" & ";
                line+=&match &count.reason {
                    ReasonForCount::FirstPreferenceCount => "First Preferences".to_string(),
                    ReasonForCount::ExcessDistribution(c) => format!("Surplus distribution for {}",metadata.candidate(*c).name),
                    ReasonForCount::Elimination(cs) => format!("Elimination of {}",cs.iter().map(|c|metadata.candidate(*c).name.clone()).collect::<Vec<_>>().join(" and ")),
                };
            }
            println!(r"{}\\",line);
        }
        println!("{}",r"\end{tabular}");
    } else {
        println!("The input file was neither a valid .stv nor .transcript file, so I'm giving up and going to a corner to sulk.")
    }
    Ok(())
}
