// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use std::str::FromStr;
use stv::election_data::ElectionData;
use stv::tie_resolution::TieResolutionsMadeByEC;
use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
use stv::ballot_metadata::{CandidateIndex, NumberOfCandidates};
use std::collections::HashSet;
use federal::{FederalRulesUsed2013, FederalRulesUsed2019, FederalRulesUsed2016, FederalRules};
use stv::preference_distribution::distribute_preferences;
use std::fmt::{Display, Formatter};
use anyhow::anyhow;
use act::{ACTPre2020, ACT2020, ACT2021};
use stv::fixed_precision_decimal::FixedPrecisionDecimal;
use serde::{Serialize,Deserialize};
use margin::record_changes::ElectionChanges;
use nsw::{NSWECLocalGov2021, NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation};
use crate::ChangeOptions;

#[derive(Copy, Clone,Serialize,Deserialize)]
pub enum Rules {
    AEC2013,
    AEC2016,
    AEC2019,
    FederalPre2021,
    ACTPre2020,
    ACT2020,
    ACT2021,
    NSWLocalGov2021,
    NSWECLocalGov2021,
}

impl FromStr for Rules {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AEC2013" => Ok(Rules::AEC2013),
            "AEC2016" => Ok(Rules::AEC2016),
            "AEC2019" => Ok(Rules::AEC2019),
            "Federal" => Ok(Rules::FederalPre2021), // this is a backwards compatability alias as the Federal rules changed in 2021. It can be deleted, and should be at some time.
            "FederalPre2021" => Ok(Rules::FederalPre2021),
            "ACTPre2020" => Ok(Rules::ACTPre2020),
            "ACT2020" => Ok(Rules::ACT2020),
            "ACT2021" => Ok(Rules::ACT2021),
            "NSWLocalGov2021" => Ok(Rules::NSWLocalGov2021),
            "NSWECLocalGov2021" => Ok(Rules::NSWECLocalGov2021),
            _ => Err("No such rule supported")
        }
    }
}

impl Display for Rules {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Rules::AEC2013 => "AEC2013",
            Rules::AEC2016 => "AEC2016",
            Rules::AEC2019 => "AEC2019",
            Rules::FederalPre2021 => "FederalPre2021",
            Rules::ACTPre2020 => "ACTPre2020",
            Rules::ACT2020 => "ACT2020",
            Rules::ACT2021 => "ACT2021",
            Rules::NSWLocalGov2021 => "NSWLocalGov2021",
            Rules::NSWECLocalGov2021 => "NSWECLocalGov2021",
        };
        f.write_str(s)
    }
}

impl Rules {

    pub fn count_simple(&self, data:&ElectionData, verbose:bool) -> anyhow::Result<PossibleTranscripts> {
        Ok(self.count(data,data.metadata.vacancies.ok_or_else(||anyhow!("Need to specify number of vacancies"))?,&data.metadata.excluded.iter().cloned().collect(),&data.metadata.tie_resolutions,verbose))
    }

    pub fn count(&self,data: &ElectionData,candidates_to_be_elected : NumberOfCandidates,excluded_candidates:&HashSet<CandidateIndex>,ec_resolutions:& TieResolutionsMadeByEC,print_progress_to_stdout:bool) -> PossibleTranscripts {
        let transcript = match self {
            Rules::AEC2013 => distribute_preferences::<FederalRulesUsed2013>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,print_progress_to_stdout),
            Rules::AEC2016 => distribute_preferences::<FederalRulesUsed2016>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,print_progress_to_stdout),
            Rules::AEC2019 => distribute_preferences::<FederalRulesUsed2019>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,print_progress_to_stdout),
            Rules::FederalPre2021 => distribute_preferences::<FederalRules>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,print_progress_to_stdout),
            Rules::ACTPre2020 => distribute_preferences::<ACTPre2020>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,print_progress_to_stdout),
            Rules::NSWLocalGov2021 => distribute_preferences::<NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,print_progress_to_stdout),
            Rules::NSWECLocalGov2021 => distribute_preferences::<NSWECLocalGov2021>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,print_progress_to_stdout),
            _ => { // handle 6 digit transcripts.
                let transcript = match self {
                    Rules::ACT2020 => distribute_preferences::<ACT2020>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,print_progress_to_stdout),
                    Rules::ACT2021 => distribute_preferences::<ACT2021>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,print_progress_to_stdout),
                    _ => panic!("Case not handled.")
                };
                return PossibleTranscripts::SixDigitDecimals(TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript })
            }
        };
        PossibleTranscripts::Integers(TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript })
    }

    pub fn find_changes(&self,data:&ElectionData,options:&ChangeOptions,verbose:bool) -> anyhow::Result<PossibleChanges> {
        Ok(match self {
            Rules::AEC2013 => PossibleChanges::Integers(options.find_changes::<FederalRulesUsed2013>(data,verbose)?),
            Rules::AEC2016 => PossibleChanges::Integers(options.find_changes::<FederalRulesUsed2016>(data,verbose)?),
            Rules::AEC2019 => PossibleChanges::Integers(options.find_changes::<FederalRulesUsed2019>(data,verbose)?),
            Rules::FederalPre2021 => PossibleChanges::Integers(options.find_changes::<FederalRules>(data,verbose)?),
            Rules::ACTPre2020 => PossibleChanges::Integers(options.find_changes::<ACTPre2020>(data,verbose)?),
            Rules::ACT2020 => PossibleChanges::SixDigitDecimals(options.find_changes::<ACT2020>(data,verbose)?),
            Rules::ACT2021 => PossibleChanges::SixDigitDecimals(options.find_changes::<ACT2021>(data,verbose)?),
            Rules::NSWLocalGov2021 => PossibleChanges::Integers(options.find_changes::<NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation>(data,verbose)?),
            Rules::NSWECLocalGov2021 => PossibleChanges::Integers(options.find_changes::<NSWECLocalGov2021>(data,verbose)?),
        })
    }

}

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct RulesDetails{
    pub name : String,
    pub description : String,
}

impl RulesDetails {
    pub fn list() -> Vec<RulesDetails> {
        vec![
            RulesDetails{ name: "AEC2013".to_string(), description: "My interpretation of the rules actually but incorrectly used by the AEC in 2013. Same as FederalPre2021, except countbacks in tie resolution did not require all candidates to have a different tally.".to_string() },
            RulesDetails{ name: "AEC2016".to_string(), description: "My interpretation of the rules actually but incorrectly used by the AEC in 2016. Same as AEC2013, except multiple elimination rules are ignored.".to_string() },
            RulesDetails{ name: "AEC2019".to_string(), description: "My interpretation of the rules actually but incorrectly used by the AEC in 2019. Same as AEC2016, except rule (18) is applied before any votes are transferred in the last elimination.".to_string() },
            RulesDetails{ name: "FederalPre2021".to_string(), description: "My interpretation of the rules that should have been used by the AEC in 2013, 2016 and 2019.".to_string() },
            RulesDetails{ name: "ACTPre2020".to_string(), description: "My interpretation of the rules that should have been, and indeed were, used by Elections ACT prior to the rule changes in 2020.".to_string() },
            RulesDetails{ name: "ACT2020".to_string(), description: "My interpretation of the rules actually but incorrectly used by Elections ACT in 2020.".to_string() },
            RulesDetails{ name: "ACT2021".to_string(), description: "My interpretation of the rules that should have been used by Elections ACT in 2020, and were actually used in 2021 to recount the 2020 election after we pointed out errors.".to_string() },
            RulesDetails{ name: "NSWLocalGov2021".to_string(), description: "My interpretation of the very ambiguous rules covering the NSW 2021 local government elections.".to_string() },
            RulesDetails{ name: "NSWECLocalGov2021".to_string(), description: "My interpretation of the rules actually used by the NSW electoral commission for the NSW 2021 local government elections. It is not how I would interpret the very ambiguous legislation, but not implausible.".to_string() },
        ]
    }
}


#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum PossibleChanges {
    Integers(ElectionChanges<usize>),
    SixDigitDecimals(ElectionChanges<FixedPrecisionDecimal<6>>),
}



#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum PossibleTranscripts {
    Integers(TranscriptWithMetadata<usize>),
    SixDigitDecimals(TranscriptWithMetadata<FixedPrecisionDecimal<6>>),
}

impl PossibleTranscripts {
    pub fn elected(&self) -> &Vec<CandidateIndex> {
        match self {
            PossibleTranscripts::Integers(t) => {&t.transcript.elected}
            PossibleTranscripts::SixDigitDecimals(t) => {&t.transcript.elected}
        }
    }
}