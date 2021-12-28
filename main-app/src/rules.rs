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
use act::{ACTPre2020, ACT2020, ACT2021};
use stv::fixed_precision_decimal::FixedPrecisionDecimal;
use serde::{Serialize,Deserialize};
use nsw::{NSWECLocalGov2021, NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation};

#[derive(Copy, Clone)]
pub enum Rules {
    AEC2013,
    AEC2016,
    AEC2019,
    Federal,
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
            "Federal" => Ok(Rules::Federal),
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
            Rules::Federal => "Federal",
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

    pub fn count(&self,data: &ElectionData,candidates_to_be_elected : NumberOfCandidates,excluded_candidates:&HashSet<CandidateIndex>,ec_resolutions:& TieResolutionsMadeByEC) -> PossibleTranscripts {
        let transcript = match self {
            Rules::AEC2013 => distribute_preferences::<FederalRulesUsed2013>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
            Rules::AEC2016 => distribute_preferences::<FederalRulesUsed2016>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
            Rules::AEC2019 => distribute_preferences::<FederalRulesUsed2019>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
            Rules::Federal => distribute_preferences::<FederalRules>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
            Rules::ACTPre2020 => distribute_preferences::<ACTPre2020>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
            Rules::NSWLocalGov2021 => distribute_preferences::<NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
            Rules::NSWECLocalGov2021 => distribute_preferences::<NSWECLocalGov2021>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
            _ => { // handle 6 digit transcripts.
                let transcript = match self {
                    Rules::ACT2020 => distribute_preferences::<ACT2020>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
                    Rules::ACT2021 => distribute_preferences::<ACT2021>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
                    _ => panic!("Case not handled.")
                };
                return PossibleTranscripts::SixDigitDecimals(TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript })
            }
        };
        PossibleTranscripts::Integers(TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum PossibleTranscripts {
    Integers(TranscriptWithMetadata<usize>),
    SixDigitDecimals(TranscriptWithMetadata<FixedPrecisionDecimal<6>>),
}