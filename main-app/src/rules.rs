use std::str::FromStr;
use stv::election_data::ElectionData;
use stv::tie_resolution::TieResolutionsMadeByEC;
use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
use stv::ballot_metadata::{CandidateIndex, NumberOfCandidates};
use std::collections::HashSet;
use federal::{FederalRulesUsed2013, FederalRulesUsed2019, FederalRulesUsed2016, FederalRules};
use stv::preference_distribution::distribute_preferences;
use std::fmt::{Display, Formatter};

#[derive(Copy, Clone)]
pub enum Rules {
    AEC2013,
    AEC2016,
    AEC2019,
    Federal,
}

impl FromStr for Rules {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AEC2013" => Ok(Rules::AEC2013),
            "AEC2016" => Ok(Rules::AEC2016),
            "AEC2019" => Ok(Rules::AEC2019),
            "Federal" => Ok(Rules::Federal),
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
        };
        f.write_str(s)
    }
}

impl Rules {

    pub fn count(&self,data: &ElectionData,candidates_to_be_elected : NumberOfCandidates,excluded_candidates:&HashSet<CandidateIndex>,ec_resolutions:& TieResolutionsMadeByEC) -> TranscriptWithMetadata<usize> {
        let transcript = match self {
            Rules::AEC2013 => distribute_preferences::<FederalRulesUsed2013>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
            Rules::AEC2016 => distribute_preferences::<FederalRulesUsed2016>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
            Rules::AEC2019 => distribute_preferences::<FederalRulesUsed2019>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
            Rules::Federal => distribute_preferences::<FederalRules>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions),
        };
        TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript }
    }
}
