
use clap::{AppSettings, Clap};
use std::path::PathBuf;
use std::fs::File;
use stv::election_data::ElectionData;
//use anyhow::anyhow;
use stv::distribution_of_preferences_transcript::{TranscriptWithMetadata, ReasonForCount};
use stv::ballot_metadata::CandidateIndex;
use std::collections::HashSet;

#[derive(Clap)]
#[clap(version = "0.1", author = "Andrew Conway")]
#[clap(setting = AppSettings::ColoredHelp)]
/// Convert a .stv file or .transcript file to a LaTeX table.
struct Opts {
    /// The name of the .stv or transcript file to convert to latex.
    #[clap(parse(from_os_str))]
    file : PathBuf,

    /// An optional output file. If not specified, stdout is used. Currently not implemented.
    #[clap(short, long,parse(from_os_str))]
    out : Option<PathBuf>,

    /// If set, show the change effected in counts as well as the total after counts.
    /// This means two rows per count (possibly 3 if #papers are different from #votes)
    #[clap(short, long)]
    deltas : bool,

    /// An optional list of candidates to restrict the table to.
    #[clap(short, long,use_delimiter=true)]
    candidates : Option<Vec<usize>>,
}

fn possibly_blank(t:usize) -> String {
    if t==0 { "".to_string() }
    else { t.to_string() }
}
fn possibly_blank_delta(t_old:usize,t_new:usize) -> String {
    if t_old==t_new { "".to_string() }
    else if t_old < t_new { "+".to_string()+&(t_new-t_old).to_string() }
    else  { "-".to_string()+&(t_old-t_new).to_string() }
}

fn main() -> anyhow::Result<()> {
    let opt : Opts = Opts::parse();

    let use_candidate = |c:CandidateIndex|{ opt.candidates.is_none() || opt.candidates.as_ref().unwrap().contains(&c.0)};

    let separate_row_for_paper_deltas = false;
    let use_action_in_count = true;
    let use_purple_for_elected_deltas = false;

    if let Ok(votes) = { let file = File::open(&opt.file)?; serde_json::from_reader::<_,ElectionData>(file) } {
        let metadata = &votes.metadata;
        println!("{}",r"\begin{tabular}{|l|l|}");
        println!("{}",r"\hline");
        println!("{}",r"\textbf{Preference List} & \textbf{Occurrences} \\ \hline");
        for btl in &votes.btl {
            println!("{} & {} \\\\",btl.candidates.iter().map(|c|metadata.candidate(*c).name.clone()).collect::<Vec<_>>().join(", "),btl.n);
        }
        println!("{}",r"\hline");
        println!("{}",r"\end{tabular}");
        // we have some votes, make a latex table out of it.
    } else if let Ok(transcript) = { let file = File::open(&opt.file)?; serde_json::from_reader::<_,TranscriptWithMetadata<usize>>(file) } {
        let mut ever_used_superscript = false;
        let mut ever_used_papers_line: bool = false;
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
            headings.push("Exh.");
        }
        if use_rounding {
            heading_justifications.push("r");
            headings.push("Rounding");
        }
        if opt.candidates.is_none() {
            heading_justifications.push("|l");
            headings.push("TV");
            if !use_action_in_count {
                heading_justifications.push("l");
                headings.push("Action");
            }
        }
        println!("{}",r"\begin{table}");
        println!("{}{}{}",r"\begin{tabular}{",heading_justifications.join(""),"}");
        println!("{}{}",headings.join(" & "),r" \\ \hline");
        let mut is_elected : HashSet<CandidateIndex> = HashSet::default();
        let mut elected_names = vec![];
        let mut have_had_zero_for : Vec<bool> = vec![false;metadata.candidates.len()]; // have no more than one zero per candidate, and blanks thereafter.
        // we have some votes, make a latex table out of it.
        for count_no in 0..transcript.transcript.counts.len() {
            let count = &transcript.transcript.counts[count_no];
            let is_excluded_next_count = |candidate:CandidateIndex| count_no+1<transcript.transcript.counts.len()&&transcript.transcript.counts[count_no+1].not_continuing.contains(&candidate)&&!transcript.transcript.elected.contains(&candidate);
            let get_tv = || {
                if let Some(tv) = &count.created_transfer_value {
                    tv.transfer_value.to_string()
                } else if let Some(tv) = count.portion.transfer_value.as_ref() {
                    tv.to_string()
                } else { "".to_string() }
            };
            let status = &count.status;
            let use_deltas = count_no>0 && opt.deltas;
            let count_reason = match &count.reason {
                ReasonForCount::FirstPreferenceCount => "First Preferences".to_string(),
                ReasonForCount::ExcessDistribution(c) => format!("Surplus {}",metadata.candidate(*c).name),
                ReasonForCount::Elimination(cs) => format!("Exclude {}",cs.iter().map(|c|metadata.candidate(*c).name.clone()).collect::<Vec<_>>().join(", ")),
            };

            if use_deltas {
                let mut use_papers_line = false;
                let prev_status = &transcript.transcript.counts[count_no-1].status;
                let mut tally_line = String::new();
                let mut papers_line = String::new();
                for i in 0..metadata.candidates.len() {
                    let candidate = CandidateIndex(i);
                    if use_candidate(candidate) {
                        tally_line+=" & ";
                        papers_line+=" & ";
                        let mut candidate_delta_tally = possibly_blank_delta(prev_status.tallies.candidate[i], status.tallies.candidate[i]);
                        let candidate_delta_papers = possibly_blank_delta(prev_status.papers.candidate[i].0,status.papers.candidate[i].0);
                        if candidate_delta_tally!=candidate_delta_papers {
                            if separate_row_for_paper_deltas { use_papers_line =true; }
                            else {
                                if candidate_delta_tally.is_empty() { candidate_delta_tally=candidate_delta_papers[0..1].to_string()+"0"; }
                                ever_used_superscript =true;
                                candidate_delta_tally+="^{";
                                candidate_delta_tally+=&candidate_delta_papers[1..];
                                candidate_delta_tally+="}";
                            }
                        }
                        if use_purple_for_elected_deltas && is_elected.contains(&candidate) { tally_line+=r"{\color{RoyalPurple} ";}
                        else {
                            //if count.not_continuing.contains(&candidate) { tally_line+=r"$\downarrow$ " }
                            tally_line+=r"{\color{Gray} ";
                        }
                        tally_line+="$";
                        tally_line+=&candidate_delta_tally;
                        tally_line+="$";
                        if !candidate_delta_papers.is_empty() {
                            papers_line+=r"{\color{PineGreen}";
                            papers_line+=&candidate_delta_papers;
                            papers_line+="}";
                        }
                        tally_line+="}";
                    }
                }
                if use_exhausted {
                    tally_line+=" & ";
                    papers_line+=" & ";
                    let mut candidate_delta_tally = possibly_blank_delta(prev_status.tallies.exhausted,status.tallies.exhausted);
                    let candidate_delta_papers = possibly_blank_delta(prev_status.papers.exhausted.0,status.papers.exhausted.0);
                    if candidate_delta_tally!=candidate_delta_papers {
                        if separate_row_for_paper_deltas { use_papers_line =true; }
                        else {
                            if candidate_delta_tally.is_empty() { candidate_delta_tally=candidate_delta_papers[0..1].to_string()+"0"; }
                            ever_used_superscript =true;
                            candidate_delta_tally+="^{";
                            candidate_delta_tally+=&candidate_delta_papers[1..];
                            candidate_delta_tally+="}";
                        }
                    }
                    tally_line+=r"{\color{Gray}$ ";
                    tally_line+=&candidate_delta_tally;
                    tally_line+="$}";
                    if !candidate_delta_papers.is_empty() {
                        papers_line+=r"{\color{PineGreen}";
                        papers_line+=&candidate_delta_papers;
                        papers_line+="}";
                    }
                }
                if use_rounding {
                    tally_line+=" & ";
                    papers_line+=" & ";
                    tally_line+=r"{\color{Gray}$ ";
                    tally_line+=&possibly_blank_delta(prev_status.tallies.rounding,status.tallies.rounding);
                    tally_line+="$}";
                }
                if opt.candidates.is_none() {
                    tally_line+=" & ";
                    papers_line+=" & ";
                    tally_line+=&format!(r"\multirow{{{}}}{{*}}{{{}}}", if use_papers_line {3} else {2}, get_tv());
                    if !use_action_in_count {
                        tally_line+=" & ";
                        papers_line+=" & ";
                        tally_line+=&format!(r"\multirow{{{}}}{{*}}{{{}}}", if use_papers_line {3} else {2}, count_reason);
                    }
                }
                let lines_used_for_count = 1+(if use_papers_line {1} else {0})+(if use_action_in_count {0} else {1});
                let tally_line : String = if lines_used_for_count>1 {
                    format!(r"\multirow{{{}}}{{*}}{{{}}}",lines_used_for_count,count_no+1)+&tally_line
                } else {
                    format!("{}", count_no+1)+&tally_line
                };
                println!(r"{}\\",tally_line);
                if use_papers_line {
                    println!(r"{}\\", papers_line);
                    ever_used_papers_line =true;
                }
            }
            for c in &count.elected {is_elected.insert(c.who); elected_names.push(metadata.candidate(c.who).name.clone())}
            let mut line = if use_deltas {String::new()} else {(count_no+1).to_string()};
            if use_action_in_count && count_no>0 {
                if !use_deltas {line+=" "; }
                line+=&count_reason;
            }
            for i in 0..metadata.candidates.len() {
                let candidate = CandidateIndex(i);
                if use_candidate(candidate) {
                    line+=" & ";
                    if is_elected.contains(&candidate) { line+=r"{\color{RoyalPurple} "; }
                    // else if count.not_continuing.contains(&candidate) && !use_deltas { line+=r"$\downarrow$ " }
                    let mut entry = possibly_blank(status.tallies.candidate[i]);
                    if entry.is_empty() {
                        if !have_had_zero_for[i] { entry+="0"; have_had_zero_for[i]=true; }
                    }
                    if is_excluded_next_count(candidate) {
                        line+=r"\xcancel{"; line+=&entry; line+="}";
                    } else {
                        line+=&entry;
                    }
                    if is_elected.contains(&candidate) { line+=r"}"; }
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
                if !use_deltas { line+=&get_tv(); }
                if !use_action_in_count {
                    line+=" & ";
                    if !use_deltas { line+=&count_reason; }
                }
            }
            println!(r"{}\\",line);
        }
        let mut caption = format!("Elected : {}.",elected_names.join(", "));
        caption+=&format!(" Rules used : {}.",transcript.transcript.rules);
        caption+=" Elected candidates are colored purple once they have a quota.";
        if ever_used_papers_line { caption+=" Lines in green are papers rather than votes.";}
        if ever_used_superscript { caption+=" Superscripts are the number of papers corresponding to the given number of votes."}
        println!("{}",r"\hline");
        println!("{}",r"\end{tabular}");
        let tablename = opt.file.file_name().map(|s|s.to_string_lossy().trim_end_matches(".transcript").to_string()).unwrap_or_else(||"Unknown".to_string());
        println!(r"\caption{{\label{{tab:{}}}{}}}",tablename,caption);
        println!("{}",r"\end{table}");
    } else {
        println!("The input file was neither a valid .stv nor .transcript file, so I'm giving up and going to a corner to sulk.")
    }
    Ok(())
}
