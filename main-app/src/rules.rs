// Copyright 2021-2024 Andrew Conway.
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
use federal::{FederalRulesUsed2013, FederalRulesUsed2019, FederalRulesUsed2016, FederalRulesPre2021, FederalRulesPost2021, FederalRulesPost2021Manual};
use stv::preference_distribution::{distribute_preferences_with_extractors};
use std::fmt::{Display, Formatter};
use anyhow::anyhow;
use act::{ACTPre2020, ACT2020, ACT2021};
use stv::fixed_precision_decimal::FixedPrecisionDecimal;
use serde::{Serialize,Deserialize};
use margin::record_changes::ElectionChanges;
use nsw::{NSWECLocalGov2021, NSWECLocalGov2021Literal, NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation, SimpleIRVAnyDifferenceBreaksTies};
use nsw::nsw_random_rules::{NSWECRandomLC2015, NSWECRandomLC2019, NSWECRandomLGE2012, NSWECRandomLGE2016, NSWECRandomLGE2017};
use stv::compare_transcripts::{compare_transcripts, DifferenceBetweenTranscripts};
use stv::extract_votes_in_pile::ExtractionRequest;
use stv::random_util::Randomness;
use vic::Vic2018LegislativeCouncil;
use wa::WALegislativeCouncil;
use crate::ChangeOptions;

#[derive(Copy, Clone,Serialize,Deserialize)]
pub enum Rules {
    AEC2013,
    AEC2016,
    AEC2019,
    FederalPre2021,
    FederalPost2021,
    FederalPost2021Manual,
    ACTPre2020,
    ACT2020,
    ACT2021,
    NSWLocalGov2021,
    NSWECLocalGov2021,
    NSWECLocalGov2021Literal,
    NSWECRandomLGE2012,
    NSWECRandomLGE2016,
    NSWECRandomLGE2017,
    NSWECRandomLC2015,
    NSWECRandomLC2019,
    Vic2018,
    WA2008,
    IRV,
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
            "FederalPost2021" => Ok(Rules::FederalPost2021),
            "FederalPost2021Manual" => Ok(Rules::FederalPost2021Manual),
            "ACTPre2020" => Ok(Rules::ACTPre2020),
            "ACT2020" => Ok(Rules::ACT2020),
            "ACT2021" => Ok(Rules::ACT2021),
            "NSWLocalGov2021" => Ok(Rules::NSWLocalGov2021),
            "NSWECLocalGov2021" => Ok(Rules::NSWECLocalGov2021),
            "NSWECLocalGov2021Literal" => Ok(Rules::NSWECLocalGov2021Literal),
            "NSWECRandomLGE2012" => Ok(Rules::NSWECRandomLGE2012),
            "NSWECRandomLGE2016" => Ok(Rules::NSWECRandomLGE2016),
            "NSWECRandomLC2015" => Ok(Rules::NSWECRandomLC2015),
            "NSWECRandomLC2019" => Ok(Rules::NSWECRandomLC2019),
            "Vic2018" => Ok(Rules::Vic2018),
            "WA2008" => Ok(Rules::WA2008),
            "IRV" => Ok(Rules::IRV),
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
            Rules::FederalPost2021 => "FederalPost2021",
            Rules::FederalPost2021Manual => "FederalPost2021Manual",
            Rules::ACTPre2020 => "ACTPre2020",
            Rules::ACT2020 => "ACT2020",
            Rules::ACT2021 => "ACT2021",
            Rules::NSWLocalGov2021 => "NSWLocalGov2021",
            Rules::NSWECLocalGov2021 => "NSWECLocalGov2021",
            Rules::NSWECLocalGov2021Literal => "NSWECLocalGov2021Literal",
            Rules::NSWECRandomLGE2012 => "NSWECRandomLGE2012",
            Rules::NSWECRandomLGE2016 => "NSWECRandomLGE2016",
            Rules::NSWECRandomLGE2017 => "NSWECRandomLGE2017",
            Rules::NSWECRandomLC2015 => "NSWECRandomLC2015",
            Rules::NSWECRandomLC2019 => "NSWECRandomLC2019",
            Rules::Vic2018 => "Vic2018",
            Rules::WA2008 => "WA2008",
            Rules::IRV => "IRV",
        };
        f.write_str(s)
    }
}

impl Rules {

    pub fn count_simple(&self, data:&ElectionData, verbose:bool,randomness:&mut Randomness,extractors:&[ExtractionRequest],include_list_of_votes_in_transcript:bool) -> anyhow::Result<PossibleTranscripts> {
        Ok(self.count(data,data.metadata.vacancies.ok_or_else(||anyhow!("Need to specify number of vacancies"))?,&data.metadata.excluded.iter().cloned().collect(),&data.metadata.tie_resolutions,None,verbose,randomness,extractors,include_list_of_votes_in_transcript))
    }

    pub fn count(&self,data: &ElectionData,candidates_to_be_elected : NumberOfCandidates,excluded_candidates:&HashSet<CandidateIndex>,ec_resolutions:& TieResolutionsMadeByEC,vote_types : Option<&[String]>,print_progress_to_stdout:bool,randomness:&mut Randomness,extractors:&[ExtractionRequest],include_list_of_votes_in_transcript:bool) -> PossibleTranscripts {
        let transcript = match self {
            Rules::AEC2013 => distribute_preferences_with_extractors::<FederalRulesUsed2013>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::AEC2016 => distribute_preferences_with_extractors::<FederalRulesUsed2016>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::AEC2019 => distribute_preferences_with_extractors::<FederalRulesUsed2019>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::FederalPre2021 => distribute_preferences_with_extractors::<FederalRulesPre2021>(data, candidates_to_be_elected, excluded_candidates, ec_resolutions, vote_types, print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::FederalPost2021 => distribute_preferences_with_extractors::<FederalRulesPost2021>(data, candidates_to_be_elected, excluded_candidates, ec_resolutions, vote_types, print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::FederalPost2021Manual => distribute_preferences_with_extractors::<FederalRulesPost2021Manual>(data, candidates_to_be_elected, excluded_candidates, ec_resolutions, vote_types, print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::ACTPre2020 => distribute_preferences_with_extractors::<ACTPre2020>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::NSWLocalGov2021 => distribute_preferences_with_extractors::<NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::NSWECLocalGov2021 => distribute_preferences_with_extractors::<NSWECLocalGov2021>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::NSWECLocalGov2021Literal => {
                let transcript = distribute_preferences_with_extractors::<NSWECLocalGov2021Literal>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript);
                return PossibleTranscripts::SignedIntegers(TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript })
            },
            Rules::NSWECRandomLGE2012 => distribute_preferences_with_extractors::<NSWECRandomLGE2012>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::NSWECRandomLGE2016 => distribute_preferences_with_extractors::<NSWECRandomLGE2016>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::NSWECRandomLGE2017 => distribute_preferences_with_extractors::<NSWECRandomLGE2017>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::NSWECRandomLC2015 => distribute_preferences_with_extractors::<NSWECRandomLC2015>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::NSWECRandomLC2019 => distribute_preferences_with_extractors::<NSWECRandomLC2019>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::Vic2018 => distribute_preferences_with_extractors::<Vic2018LegislativeCouncil>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::WA2008 => distribute_preferences_with_extractors::<WALegislativeCouncil>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            Rules::IRV => distribute_preferences_with_extractors::<SimpleIRVAnyDifferenceBreaksTies>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
            _ => { // handle 6 digit transcripts.
                let transcript = match self {
                    Rules::ACT2020 => distribute_preferences_with_extractors::<ACT2020>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
                    Rules::ACT2021 => distribute_preferences_with_extractors::<ACT2021>(data,candidates_to_be_elected,excluded_candidates,ec_resolutions,vote_types,print_progress_to_stdout,randomness,extractors,include_list_of_votes_in_transcript),
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
            Rules::FederalPre2021 => PossibleChanges::Integers(options.find_changes::<FederalRulesPre2021>(data, verbose)?),
            Rules::FederalPost2021 => PossibleChanges::Integers(options.find_changes::<FederalRulesPost2021>(data, verbose)?),
            Rules::FederalPost2021Manual => PossibleChanges::Integers(options.find_changes::<FederalRulesPost2021Manual>(data, verbose)?),
            Rules::ACTPre2020 => PossibleChanges::Integers(options.find_changes::<ACTPre2020>(data,verbose)?),
            Rules::ACT2020 => PossibleChanges::SixDigitDecimals(options.find_changes::<ACT2020>(data,verbose)?),
            Rules::ACT2021 => PossibleChanges::SixDigitDecimals(options.find_changes::<ACT2021>(data,verbose)?),
            Rules::NSWLocalGov2021 => PossibleChanges::Integers(options.find_changes::<NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation>(data,verbose)?),
            Rules::NSWECLocalGov2021 => PossibleChanges::Integers(options.find_changes::<NSWECLocalGov2021>(data,verbose)?),
            Rules::NSWECLocalGov2021Literal => PossibleChanges::SignedIntegers(options.find_changes::<NSWECLocalGov2021Literal>(data,verbose)?),
            Rules::Vic2018 => PossibleChanges::Integers(options.find_changes::<Vic2018LegislativeCouncil>(data,verbose)?),
            Rules::WA2008 => PossibleChanges::Integers(options.find_changes::<WALegislativeCouncil>(data,verbose)?),
            Rules::IRV => PossibleChanges::Integers(options.find_changes::<SimpleIRVAnyDifferenceBreaksTies>(data,verbose)?),
            Rules::NSWECRandomLGE2012 => PossibleChanges::Integers(options.find_changes::<NSWECRandomLGE2012>(data, verbose)?),
            Rules::NSWECRandomLGE2016 => PossibleChanges::Integers(options.find_changes::<NSWECRandomLGE2016>(data, verbose)?),
            Rules::NSWECRandomLGE2017 => PossibleChanges::Integers(options.find_changes::<NSWECRandomLGE2017>(data, verbose)?),
            Rules::NSWECRandomLC2015 => PossibleChanges::Integers(options.find_changes::<NSWECRandomLC2015>(data,verbose)?),
            Rules::NSWECRandomLC2019 => PossibleChanges::Integers(options.find_changes::<NSWECRandomLC2019>(data,verbose)?),
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
            RulesDetails{ name: "FederalPost2021".to_string(), description: "My interpretation of the rules that should have been used by the AEC in 2022.".to_string() },
            RulesDetails{ name: "FederalPost2021Manual".to_string(), description: "My interpretation of the rules that should have been used by the AEC in 2022, if counting by hand instead of computer. Same as FederalPost2021 apart from allowing use of rule 13(a).".to_string() },
            RulesDetails{ name: "ACTPre2020".to_string(), description: "My interpretation of the rules that should have been, and indeed were, used by Elections ACT prior to the rule changes in 2020.".to_string() },
            RulesDetails{ name: "ACT2020".to_string(), description: "My interpretation of the rules actually but incorrectly used by Elections ACT in 2020.".to_string() },
            RulesDetails{ name: "ACT2021".to_string(), description: "My interpretation of the rules that should have been used by Elections ACT in 2020, and were actually used in 2021 to recount the 2020 election after we pointed out errors.".to_string() },
            RulesDetails{ name: "NSWLocalGov2021".to_string(), description: "My interpretation of the very ambiguous rules covering the NSW 2021 local government elections.".to_string() },
            RulesDetails{ name: "NSWECLocalGov2021".to_string(), description: "My interpretation of the rules actually used by the NSW electoral commission for the NSW 2021 local government elections, assuming they didn't take (7)(4)(a) literally. It is not how I would interpret the very ambiguous legislation, but not implausible.".to_string() },
            RulesDetails{ name: "NSWECLocalGov2021Literal".to_string(), description: "My interpretation of the rules actually used by the NSW electoral commission for the NSW 2021 local government elections, assuming they did take (7)(4)(a) literally. It is not how I would interpret the very ambiguous legislation, but not implausible.".to_string() },
            RulesDetails{ name: "NSWECRandomLGE2012".to_string(), description: "My interpretation of the rules actually used by the NSW electoral commission for the NSW 2012 local government elections. Note that there is considerable randomness so recounting with a different random choices will probably produce different results. Same as NSWECRandomLGE2016 except sometimes incorrectly computes last parcel.".to_string() },
            RulesDetails{ name: "NSWECRandomLGE2016".to_string(), description: "My interpretation of the rules actually used by the NSW electoral commission for the NSW 2016 local government elections. Note that there is considerable randomness so recounting with a different random choices will probably produce different results. Same as NSWECRandomLGE2017 except gets some fractions wrong and gets some tie resolutions wrong.".to_string() },
            RulesDetails{ name: "NSWECRandomLGE2017".to_string(), description: "My interpretation of the rules actually used by the NSW electoral commission for the NSW 2017 local government elections. Note that there is considerable randomness so recounting with a different random choices will probably produce different results.".to_string() },
            RulesDetails{ name: "NSWECRandomLC2015".to_string(), description: "My interpretation of the rules actually used by the NSW electoral commission for the NSW 2015 legislative council elections. Note that there is considerable randomness so recounting with a different random choices will probably produce different results. Same as NSWECRandomLC2019 except with the same last parcel error as NSWECRandomLGE2012 (which didn't come up so may or may not be present).".to_string() },
            RulesDetails{ name: "NSWECRandomLC2019".to_string(), description: "My interpretation of the rules actually used by the NSW electoral commission for the NSW 2019 and 2023 legislative council elections. Note that there is considerable randomness so recounting with a different random choices will probably produce different results. ".to_string() },
            RulesDetails{ name: "Vic2018".to_string(), description: "My interpretation of the rules that should have been used by the VEC since the 2018 modification to 114A(28)(c) of the Electoral Act 2002, and a plausible if not literal interpretation of the rules prior to that.".to_string() },
            RulesDetails{ name: "WA2018".to_string(), description: "My interpretation of the Western Australian Legislative Council rules consistent with the 2008 published official distribution of preferences.".to_string() },
            RulesDetails{ name: "IRV".to_string(), description: "IRV with tie resolution by count backs with any non-equality breaking ties where possible.".to_string() },
        ]
    }
}


#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum PossibleChanges {
    Integers(ElectionChanges<usize>),
    SignedIntegers(ElectionChanges<isize>),
    SixDigitDecimals(ElectionChanges<FixedPrecisionDecimal<6>>),
}



#[derive(Serialize, Deserialize,Debug,Clone)]
#[serde(untagged)]
pub enum PossibleTranscripts {
    Integers(TranscriptWithMetadata<usize>),
    SignedIntegers(TranscriptWithMetadata<isize>),
    SixDigitDecimals(TranscriptWithMetadata<FixedPrecisionDecimal<6>>),
}

impl PossibleTranscripts {
    pub fn elected(&self) -> &Vec<CandidateIndex> {
        match self {
            PossibleTranscripts::Integers(t) => {&t.transcript.elected}
            PossibleTranscripts::SignedIntegers(t) => {&t.transcript.elected}
            PossibleTranscripts::SixDigitDecimals(t) => {&t.transcript.elected}
        }
    }

    pub fn compare_transcripts(&self, other:&PossibleTranscripts) -> DifferenceBetweenTranscripts {
        match (self,other) {
            (PossibleTranscripts::Integers(t1), PossibleTranscripts::Integers(t2)) => compare_transcripts(&t1.transcript,&t2.transcript),
            (PossibleTranscripts::Integers(t1), PossibleTranscripts::SignedIntegers(t2)) => compare_transcripts(&t1.transcript,&t2.transcript),
            (PossibleTranscripts::Integers(t1), PossibleTranscripts::SixDigitDecimals(t2)) => compare_transcripts(&t1.transcript,&t2.transcript),
            (PossibleTranscripts::SignedIntegers(t1), PossibleTranscripts::Integers(t2)) => compare_transcripts(&t1.transcript,&t2.transcript),
            (PossibleTranscripts::SignedIntegers(t1), PossibleTranscripts::SignedIntegers(t2)) => compare_transcripts(&t1.transcript,&t2.transcript),
            (PossibleTranscripts::SignedIntegers(t1), PossibleTranscripts::SixDigitDecimals(t2)) => compare_transcripts(&t1.transcript,&t2.transcript),
            (PossibleTranscripts::SixDigitDecimals(t1), PossibleTranscripts::Integers(t2)) => compare_transcripts(&t1.transcript,&t2.transcript),
            (PossibleTranscripts::SixDigitDecimals(t1), PossibleTranscripts::SignedIntegers(t2)) => compare_transcripts(&t1.transcript,&t2.transcript),
            (PossibleTranscripts::SixDigitDecimals(t1), PossibleTranscripts::SixDigitDecimals(t2)) => compare_transcripts(&t1.transcript,&t2.transcript),
        }
    }
}