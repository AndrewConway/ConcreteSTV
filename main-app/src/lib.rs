// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! This crate tries to provide a unified API to different file formats and counting algorithms.
//! It also contains the main binaries.

use clap::Args;
use std::collections::HashSet;
use std::fs::File;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::FromStr;
use anyhow::anyhow;
use margin::choose_votes::ChooseVotesOptions;
use margin::find_outcome_changes::find_outcome_changes;
use margin::record_changes::ElectionChanges;
use stv::ballot_metadata::{CandidateIndex, NumberOfCandidates};
use stv::distribution_of_preferences_transcript::CountIndex;
use stv::election_data::ElectionData;
use stv::preference_distribution::PreferenceDistributionRules;
use stv::tie_resolution::{TieResolutionAtom, TieResolutionExplicitDecision, TieResolutionExplicitDecisionInCount, TieResolutionsMadeByEC, TieResolutionUsage};
use crate::rules::Rules;

pub mod rules;
pub mod ec_data_source;
pub mod compare_different_rules;


/// Utility that is helpful for parsing tie resolution description string in clap a Vec<Vec<CandidateIndex>>.
pub fn try_parse_candidate_list(s:&str) -> anyhow::Result<TieResolutionAtom> {
    fn string_to_candidate_list(s:&str) -> Result<Vec<CandidateIndex>,ParseIntError> {
        s.split('/').map(|s|s.trim().parse::<CandidateIndex>()).collect()
    }
    let mut came_up_in : Option<CountIndex> = None;
    let mut usage : Option<TieResolutionUsage> = None;
    let mut split_colon = s.split(':');
    let main_list = split_colon.next().ok_or_else(||anyhow!("Empty tie string"))?;
    for colon_option in split_colon {
        if let Ok(parsed_usage) = TieResolutionUsage::from_str(colon_option) { usage=Some(parsed_usage); }
        else if let Ok(count) = usize::from_str(colon_option) {
            if count>0 { came_up_in=Some(CountIndex(count-1))} else { return Err(anyhow!("The first count is 1, not 0.")) }
        } else { return Err(anyhow!("Do not understand option {}",colon_option)); }
    }
    let mut increasing_favour : Vec<Vec<CandidateIndex>> = vec![];
    for same_level in main_list.split(',') {
        increasing_favour.push(string_to_candidate_list(same_level)?);
    }
    Ok(TieResolutionAtom::ExplicitDecision(TieResolutionExplicitDecisionInCount{ decision: TieResolutionExplicitDecision { increasing_favour, usage }, came_up_in }))
}

/// Options that pertain to what ballots are to be considered for changing
#[derive(Args)]
#[clap(next_help_heading="Options for which ballots to consider changing")]
pub struct ChangeOptions {
    /// Should be followed by true, false, or both (separated by commas)
    /// Whether above the line votes should be allowed to be changed. Default true.
    /// If both true and false are specified, changes will be searched for both with and without above the line votes (slower).
    #[clap(long, value_delimiter=',',default_value="true")]
    allow_atl : Vec<bool>,

    /// Should be followed by true, false, or both (separated by commas)
    /// Whether changes to the first preferences votes should be allowed. Default true.
    /// If both true and false are specified, changes will be searched for both with and without first preference modifications (slower).
    #[clap(long, value_delimiter=',',default_value="true")]
    allow_first : Vec<bool>,

    /// Should be followed by true, false, or both (separated by commas)
    /// Whether changes to ballots that are in principle verifiable should be allowed. Default true.
    /// This option only makes sense if the `--unverifiable` flag is also used.
    /// If both true and false are specified, changes will be searched for both with and without this restriction (slower).
    #[clap(long, value_delimiter=',',default_value="true")]
    allow_verifiable : Vec<bool>,

    /// What types of votes are considered unverifiable for the purposes of allow_verifiable.
    /// The string (or strings separated by commas) following this are election specific, and correspond to types specified by the electoral commission.
    #[clap(long, value_delimiter=',')]
    unverifiable : Vec<String>,
}


impl ChangeOptions {
    fn find_changes<Rules:PreferenceDistributionRules>(&self,data:&ElectionData,verbose:bool) -> anyhow::Result<ElectionChanges<Rules::Tally>> {
        let ballot_types_considered_unverifiable = self.unverifiable.iter().cloned().collect::<HashSet<_>>();
        let mut res : Option<ElectionChanges<Rules::Tally>> = None;
        for &allow_atl in  &self.allow_atl {
            for &allow_first_pref in &self.allow_first {
                for &allow_verifiable in &self.allow_verifiable {
                    let options = ChooseVotesOptions{allow_atl,allow_first_pref,allow_verifiable,ballot_types_considered_unverifiable:ballot_types_considered_unverifiable.clone()};
                    let results = find_outcome_changes::<Rules>(&data,&options,verbose);
                    if res.is_none() { res=Some(results)} else { res.as_mut().unwrap().merge(results,false) }
                }
            }
        }
        let mut res = res.ok_or_else(||anyhow!("No votes allowed to be modified"))?;
        res.sort();
        Ok(res)
    }
}

/// Options that pertain to modifications to a vote data file
#[derive(Args)]
#[clap(next_help_heading="Options for overriding the input .stv file")]
pub struct ModifyStvFileOptions {
    /// The number of people to elect. If used, overrides the value in the .stv file.
    #[clap(short, long)]
    vacancies : Option<NumberOfCandidates>,

    /// An optional list of candidates to exclude. This is a comma separated list of numbers,
    /// starting counting at zero. E.g. --exclude=5,6 would do the count assuming the candidates
    /// with 5 and 6 other candidates listed before them are ineligible. If specified, this overrides
    /// any candidates specified as excluded in the .stv file.
    #[clap(short, long, value_delimiter=',')]
    exclude : Option<Vec<CandidateIndex>>,

    /// If a .vchange file is used for input instead of a .stv file, one of the vote manipulations in it can be applied first, specified here. 1 means the first one in the file, 2 the second, etc.
    /// This can be used to prove an upper bound on the margin.
    #[clap(short, long)]
    modification : Option<usize>,

    /// Specified resolution of ties that need to be resolved by the electoral commission, often by lot.
    ///
    /// ConcreteSTV, by default, chooses in favour of the candidate in a worse donkey-vote position (higher indices favoured),
    /// or by pseudo-random draw if the `--seed` argument is set.
    /// This is overriden by explicit tie resolutions specified when creating the .stv file.
    /// This flag overrides both of these.
    ///
    /// You can override this by specifying a list of candidate indices (starting counting at 0) to favour in said priority order.
    /// For example in a tie resolved between candidates 27 and 43, ConcreteSTV would favour 43 by default. Enter `--tie 43,27` to
    /// indicate that 27 should be favoured over 43 in a decision between them.
    ///
    /// To indicate that in a situation where there are 5 candidates 2,4,6,8 and 9, and two of them (2 & 6) end up being
    /// disfavoured over the other three when such a decision needs to be made, enter `--tie 2/6,4/8/9` which does
    /// not indicate relative ranking between 2 and 6, or between 4,8 and 9, but implies that both 2 and 6 are less favoured
    /// than any of 4, 8 or 9. There can be more than one comma character for more complex rankings.
    ///
    /// The time or situation where the ranking is used may be specified more precisely by appending `:6` to the
    /// tie argument which would mean that this only applies on the 6th count or `:Exclusion`, `:OrderElected`,
    /// `:ShortcutWinner`, `:OrderSurplusDistributed` or `:RoundingUp` to indicate what the tie resolution will
    /// be used in. Both a count number and a description are allowed. So `--tie 3,4:5:Exclusion` will mean that
    /// candidate 3 is less favoured than candidate 4, in the fifth count, for the purpose of working out who is
    /// excluded. Otherwise `--tie 3,4` would be used for all situations in which a tie between candidates 3 and 4
    /// needed to be resolved.
    ///
    /// This flag may be used multiple times for multiple tie resolutions. If multiple entries are applicable for
    /// a given case, the first applicable one will be used.
    #[clap(long,value_parser=try_parse_candidate_list)]
    tie : Vec<TieResolutionAtom>,

}

impl ModifyStvFileOptions {
    pub fn get_data(&self,input_path:&PathBuf,verbose:bool) -> anyhow::Result<ElectionData> {
        let mut votes : ElectionData = {
            let file = File::open(input_path)?;
            if input_path.as_os_str().to_string_lossy().ends_with(".vchange") {
                let vchange : ElectionChanges<f64> = serde_json::from_reader(file)?; // Everything so far will parse as f64, and the values are not used in way here so accuracy is irrelevant.
                if let Some(modification_number_1_based) = self.modification {
                    if modification_number_1_based>vchange.changes.len() || modification_number_1_based==0 {
                        return Err(anyhow!("Modification number {} should be between 1 and {} (the number of modifications in that file)",modification_number_1_based,vchange.changes.len()));
                    }
                    vchange.changes[modification_number_1_based-1].ballots.apply_to_votes(&vchange.original,verbose)
                } else { vchange.original }
            } else {
                serde_json::from_reader(file)?
            }
        };

        if let Some(vacancies) = self.vacancies { votes.metadata.vacancies=Some(vacancies); }
        if let Some(ineligible) = self.exclude.as_ref() { votes.metadata.excluded = ineligible.clone(); }
        if !self.tie.is_empty() { votes.metadata.tie_resolutions=TieResolutionsMadeByEC{tie_resolutions:self.tie.clone()}; }
        Ok(votes)
    }

    pub fn result_file_name(&self,input_path:&PathBuf,explicit_out_path:Option<&PathBuf>,extension:&str,rules:&Rules) -> PathBuf {
        match explicit_out_path {
            None => {
                let votename = input_path.file_name().map(|o|o.to_string_lossy()).unwrap_or_default();
                let votename = votename.trim_end_matches(".stv").trim_end_matches(".vchange");
                let modname = if let Some(modification) = self.modification { modification.to_string()+"_"} else {"".to_string()};
                let rulename = rules.to_string();
                let combined = votename.to_string()+"_"+&modname+&rulename+extension;
                input_path.with_file_name(combined)
            }
            Some(tf) => tf.clone(),
        }
    }
}
