// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! This crate tries to provide a unified API to different file formats and counting algorithms.
//! It also contains the main binaries.

use clap::Args;
use std::collections::HashSet;
use std::num::ParseIntError;
use anyhow::anyhow;
use margin::choose_votes::ChooseVotesOptions;
use margin::find_outcome_changes::find_outcome_changes;
use margin::record_changes::ElectionChanges;
use stv::ballot_metadata::CandidateIndex;
use stv::election_data::ElectionData;
use stv::preference_distribution::PreferenceDistributionRules;

pub mod rules;
pub mod ec_data_source;

/// Utility that is helpful for parsing in clap a Vec<Vec<CandidateIndex>>.
pub fn try_parse_candidate_list(s:&str) -> Result<Vec<CandidateIndex>,ParseIntError> {
    s.split(',').map(|s|s.trim().parse::<CandidateIndex>()).collect()
}

/// Options that pertain to what ballots are to be considered for changing
#[derive(Args)]
pub struct ChangeOptions {
    /// Should be followed by true, false, or both (separated by commas)
    /// Whether above the line votes should be allowed. Default true.
    /// If both true and false are specified, changes will be searched for both with an without above the line votes (slower).
    #[clap(long,use_delimiter=true,require_delimiter=true,default_value="true")]
    allow_atl : Vec<bool>,

    /// Should be followed by true, false, or both (separated by commas)
    /// Whether changes to the first preferences votes should be allowed. Default true.
    /// If both true and false are specified, changes will be searched for both with an without first preference modifications (slower).
    #[clap(long,use_delimiter=true,require_delimiter=true,default_value="true")]
    allow_first : Vec<bool>,

    /// Should be followed by true, false, or both (separated by commas)
    /// Whether changes to ballots that are in principle verifiable. Default true.
    /// This option only makes sense if the `--unverifiable` flag is also used.
    /// If both true and false are specified, changes will be searched for both with an without this restriction (slower).
    #[clap(long,use_delimiter=true,require_delimiter=true,default_value="true")]
    allow_verifiable : Vec<bool>,

    /// What types of votes are considered unverifiable for the purposes of allow_verifiable.
    /// The string (or strings separated by commas) following this are election specific, and correspond to types specified by the electoral commission.
    #[clap(long,use_delimiter=true,require_delimiter=true)]
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
        let mut res = res.ok_or_else(||anyhow!("No votes allowed to be modifed"))?;
        res.sort();
        Ok(res)
    }
}