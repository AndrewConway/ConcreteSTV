// Copyright 2024 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! This module is used for comparing the same dataset on different rules.
//! There are two different levels of comparison of datasets one could do:
//! * First divide them into groups according to which candidates end up elected.
//! * Next divide each group into subgroups according to the order of electing candidates
//! * Next divide each group into subgroups that have different values at a particular count or different numbers of counts.


use std::fmt;
use serde::{Deserialize, Serialize};
use stv::ballot_metadata::{CandidateIndex, ElectionMetadata};
use stv::compare_transcripts::DifferenceBetweenTranscripts;
use stv::election_data::ElectionData;
use stv::random_util::Randomness;
use crate::rules::{PossibleTranscripts, Rules};

#[derive(Debug,Clone,Serialize,Deserialize)]
/// A comparison of different rules for a particular dataset.
///
/// For displaying, the precision affects the method of display:
/// * 1 precision means only care about who is elected
/// * 2 precision means only care aboue who is elected or the order thereof
/// * 3 precision (default) means care about everything.
///
pub struct RulesComparisonGroups {
    metadata : ElectionMetadata,
    groups : Vec<SameCandidatesElected>
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct SameCandidatesElected {
    pub candidates : Vec<CandidateIndex>, // candidates sorted numerically
    pub subgroups : Vec<SameOrderElected>
}
#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct SameOrderElected {
    pub candidates : Vec<CandidateIndex>, // candidates in order of election
    pub subgroups : Vec<SameTranscript>,
}
#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct SameTranscript {
    transcript : PossibleTranscripts, // a sample transcript
    pub rules : Vec<String>
}

impl SameOrderElected {
    pub fn all_rules(&self) -> Vec<String> {
        self.subgroups.iter().flat_map(|s|s.rules.iter().cloned()).collect()
    }
}

impl SameCandidatesElected {
    pub fn all_rules(&self) -> Vec<String> {
        self.subgroups.iter().flat_map(|s|s.all_rules().into_iter()).collect()
    }
}

impl fmt::Display for RulesComparisonGroups {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let precision = f.precision().unwrap_or(3);
        writeln!(f, "{}", self.metadata.name.human_readable_name())?;
        let candidates_as_string = |candidates:&[CandidateIndex]|{
            candidates.iter().map(|c|self.metadata.candidate(*c).name.as_str()).collect::<Vec<_>>().join(" & ")
        };
        for (same_elected_index,same_elected) in self.groups.iter().enumerate() {
            if precision==1 { // just print who was elected. Use the first group for ordering.
                writeln!(f," {}",candidates_as_string(&same_elected.subgroups[0].candidates))?;
                writeln!(f,"   {}",same_elected.all_rules().join(" "))?;
            } else {
                if same_elected_index!=0 { writeln!(f,"or")? } // indicate different winners.
                for same_order in &same_elected.subgroups {
                    writeln!(f," {}",candidates_as_string(&same_order.candidates))?;
                    if precision==2 { // just print who was elected in the same order
                        writeln!(f,"   {}",same_order.all_rules().join(" "))?;
                    } else {
                        for same_transcript in &same_order.subgroups {
                            writeln!(f,"   {}",same_transcript.rules.join(" "))?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl RulesComparisonGroups {
    pub fn create(data:&ElectionData,rules:&[Rules]) -> anyhow::Result<Self> {
        let mut res = RulesComparisonGroups { metadata: data.metadata.clone(), groups: vec![] };
        for rule in rules {
            let transcript = rule.count_simple(data,false,&mut Randomness::ReverseDonkeyVote,&[])?;
            let winners = transcript.elected();
            let mut ordered_winners = winners.clone();
            ordered_winners.sort_by_key(|c|c.0);
            let same_candidates_elected = {
                match res.groups.iter_mut().find(|g|g.candidates==ordered_winners) {
                    Some(existing) => existing,
                    None => { res.groups.push(SameCandidatesElected{ candidates: ordered_winners, subgroups: vec![] }); res.groups.last_mut().unwrap() }
                }
            };
            let same_candidates_order = {
                match same_candidates_elected.subgroups.iter_mut().find(|g|&g.candidates==winners) {
                    Some(existing) => existing,
                    None => { same_candidates_elected.subgroups.push(SameOrderElected{ candidates: winners.clone(), subgroups: vec![] }); same_candidates_elected.subgroups.last_mut().unwrap() }
                }
            };
            let same_transcript = {
                match same_candidates_order.subgroups.iter_mut().find(|g|g.transcript.compare_transcripts(&transcript)==DifferenceBetweenTranscripts::Same) {
                    Some(existing) => existing,
                    None => { same_candidates_order.subgroups.push(SameTranscript{ transcript, rules: vec![] }); same_candidates_order.subgroups.last_mut().unwrap() }
                }
            };
            same_transcript.rules.push(rule.to_string());
        }
        Ok(res)
    }

    pub fn has_different_winners(&self) -> bool { self.groups.len()>1 }
    pub fn has_different_orders(&self) -> bool { self.groups.iter().any(|g|g.subgroups.len()>1) }
}