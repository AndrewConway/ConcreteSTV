// Copyright 2024 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.




use std::borrow::Cow;
use std::marker::PhantomData;
use std::path::PathBuf;
use anyhow::anyhow;
use stv::ballot_metadata::{CandidateIndex, ElectionMetadata, ElectionName, NumberOfCandidates};
use stv::datasource_description::{AssociatedRules, Copyright, ElectionDataSource};
use stv::election_data::ElectionData;
use stv::official_dop_transcript::OfficialDistributionOfPreferencesTranscript;
use stv::parse_util::{FileFinder, KnowsAboutRawMarkings, MissingFile, RawDataSource};
use stv::tie_resolution::TieResolutionsMadeByEC;

/// A data source representing the made up data in the examples directory.
pub struct ExampleDataSource {}

impl ElectionDataSource for ExampleDataSource {
    fn name(&self) -> Cow<'static, str> { "Oddities".into() }
    fn ec_name(&self) -> Cow<'static, str> { "Made up examples by Andrew Conway".into() }
    fn ec_url(&self) -> Cow<'static, str> { "https://vote.andrewconway.org/".into() }
    fn years(&self) -> Vec<String> { vec!["Federal".to_string(),"NSW LGE".to_string(),"Generic".to_string()] }
    fn get_loader_for_year(&self,year: &str,_finder:&FileFinder) -> anyhow::Result<Box<dyn RawDataSource+Send+Sync>> {
        match year {
            "Federal" => Ok(Box::new(ExampleDataLoader::<FederalExamples>{ phantom_data: Default::default() })),
            "NSW LGE" => Ok(Box::new(ExampleDataLoader::<NSWLGEExamples>{ phantom_data: Default::default() })),
            "Generic" => Ok(Box::new(ExampleDataLoader::<GenericExamples>{ phantom_data: Default::default() })),
            _ => Err(anyhow!("Not a valid example category")),
        }

    }
}

struct ExampleDataLoader<T:SimpleExampleFromText> {
    phantom_data: PhantomData<T>
}

/// Examples relevant particularly to the Federal Senate
pub struct FederalExamples {}

impl SimpleExampleFromText for FederalExamples {
    fn all_electorates() -> Vec<String> {
        vec!["MultipleExclusionOrdering".to_string(),"MultipleExclusionRounding".to_string(),"FourQuotasDoesntMeanFourSeats".to_string(),"FiveQuotasDoesntMeanFourSeats".to_string()]
    }

    fn get_raw_data_as_string(electorate: &str) -> anyhow::Result<&'static str> {
        match electorate {
            "MultipleExclusionOrdering" => Ok(include_str!("../MultipleExclusionOrdering.stv")),
            "MultipleExclusionRounding" => Ok(include_str!("../MultipleExclusionRounding.stv")),
            "FourQuotasDoesntMeanFourSeats" => Ok(include_str!("../FourQuotasDoesntMeanFourSeats.stv")),
            "FiveQuotasDoesntMeanFourSeats" => Ok(include_str!("../FiveQuotasDoesntMeanFourSeats.stv")),
            _ => Err(anyhow!("No such Federal example {}",electorate))
        }
    }

    fn rules(electorate: &str) -> AssociatedRules {
        AssociatedRules {
            rules_used: None,
            rules_recommended: Some("FederalPost2021".into()),
            comment: match electorate {
                "MultipleExclusionOrdering" => Some("This produces different candidates elected with AEC2013, AEC2016 (due to missing bulk exclusion) and AEC2019 (due to missing bulk exclusion and the distribution of preferences for the last candidate excluded). After the legislation changes in 2021, it produces different candidates elected if counted by computer (FederalPost2021) or manually (FederalPost2021Manual) due to the different handling of bulk exclusion. The difference is due to a subtle issue in the conditions for bulk exclusion in the federal legislation".into()),
                "MultipleExclusionRounding" => Some("This produces different candidates elected with AEC2013 versus AEC2016 or AEC2019 (due to missing bulk exclusion in both). After the legislation changes in 2021, it produces different candidates elected if counted by computer (FederalPost2021) or manually (FederalPost2021Manual) due to the different handling of bulk exclusion. The difference is due to a changes in rounding due to bulk exclusion.".into()),
                "FourQuotasDoesntMeanFourSeats" => Some("An example of a set of votes where one party wins over 4 quotas of above the line votes but does not get four candidates elected.".into()),
                "FiveQuotasDoesntMeanFourSeats" => Some("An example of a set of votes where one party wins over 5 quotas of above the line votes but does not even get four candidates elected.".into()),
                _ => None,
            },
            reports:  match electorate { // TODO add report when done
                "MultipleExclusionOrdering" | "MultipleExclusionRounding" => vec!["https://github.com/AndrewConway/ConcreteSTV/blob/main/reports/RecommendedAmendmentsSenateCountingAndScrutiny.pdf".into()],
                "FourQuotasDoesntMeanFourSeats" | "FiveQuotasDoesntMeanFourSeats" => vec![],
                _ => vec![],
            },
        }
    }
}

/// Examples relevant particularly to the New South Wales local government elections
pub struct NSWLGEExamples {}

impl SimpleExampleFromText for crate::example_datasource::NSWLGEExamples {
    fn all_electorates() -> Vec<String> {
        vec!["NegativeTally".to_string(),"TransferValueOverOne".to_string()]
    }

    fn get_raw_data_as_string(electorate: &str) -> anyhow::Result<&'static str> {
        match electorate {
            "NegativeTally" => Ok(include_str!("../NegativeTally.stv")),
            "TransferValueOverOne" => Ok(include_str!("../TransferValueOverOne.stv")),
            _ => Err(anyhow!("No such Federal example {}",electorate))
        }
    }

    fn rules(electorate: &str) -> AssociatedRules {
        AssociatedRules {
            rules_used: None,
            rules_recommended: Some("NSWECLocalGov2021Literal".into()),
            comment: match electorate {
                "NegativeTally" => Some("This produces negative transfer values with the NSWECLocalGov2021Literal ruleset, due to a idiosyncratic formula in the legislation. As a result one of the elected candidates is elected on a negative number of votes.".into()),
                "TransferValueOverOne" => Some("The product of two uses of the idiosyncratic formula for transfer values results in a transfer value over one, resulting in too many candidates going over quota and getting elected when using the NSWECLocalGov2021Literal ruleset.".into()),
                _ => None,
            },
            reports:  match electorate { // TODO add report when done
                "NegativeTally" | "TransferValueOverOne" => vec![],
                _ => vec![],
            },
        }
    }
}


/// Examples relevant not particular to any STV variant
pub struct GenericExamples {}

impl SimpleExampleFromText for crate::example_datasource::GenericExamples {
    fn all_electorates() -> Vec<String> {
        vec!["DummyFirstPreference".to_string(),"DummyFirstPreferenceAlternative".to_string(),"DummyFirstPreference2".to_string(),"DummyFirstPreferenceAlternative2".to_string(),"WinByGivingAwayVotes".to_string(),"WinByGivingAwayVotesAlternative".to_string()]
    }

    fn get_raw_data_as_string(electorate: &str) -> anyhow::Result<&'static str> {
        match electorate {
            "DummyFirstPreference" => Ok(include_str!("../DummyFirstPreference.stv")),
            "DummyFirstPreferenceAlternative" => Ok(include_str!("../DummyFirstPreferenceAlternative.stv")),
            "DummyFirstPreference2" => Ok(include_str!("../DummyFirstPreference2.stv")),
            "DummyFirstPreferenceAlternative2" => Ok(include_str!("../DummyFirstPreferenceAlternative2.stv")),
            "WinByGivingAwayVotes" => Ok(include_str!("../WinByGivingAwayVotes.stv")),
            "WinByGivingAwayVotesAlternative" => Ok(include_str!("../WinByGivingAwayVotesAlternative.stv")),
            _ => Err(anyhow!("No such Federal example {}",electorate))
        }
    }

    fn rules(electorate: &str) -> AssociatedRules {
        AssociatedRules {
            rules_used: None,
            rules_recommended: Some("FederalPost2021".into()),
            comment: match electorate {
                "DummyFirstPreference" => Some("This is an example of tactical voting, where a voter adds an undesired candidate E as first preference before C1,C2,O to increase the power of the vote in some situations. Applicable to most rulesets. See DummyFirstPreferenceAlternative".into()),
                "DummyFirstPreferenceAlternative" => Some("This is like DummyFirstPreference except the tactical voter here expresses their true preferences C1,C2,O, resulting in hated P winning instead of O.".into()),
                "DummyFirstPreference2" => Some("This is an example of tactical voting, where two voters add an undesired candidate E as first preference before C1,C2,O to increase the power of the vote in some situations. Applicable to most rulesets. See DummyFirstPreferenceAlternative2. Similar to DummyFirstPreference, except 2 voters instead of 1 means it does not have to rely on tie resolution via countbacks.".into()),
                "DummyFirstPreferenceAlternative2" => Some("This is like DummyFirstPreference2 except the tactical voters here expresses their true preferences C1,C2,O, resulting in hated P winning instead of O. . Similar to DummyFirstPreferenceAlternative, except 2 voters instead of 1 means it does not have to rely on tie resolution via countbacks.".into()),
                "WinByGivingAwayVotes" => Some("Candidate C loses in this election, but would win by persuading some people who voted for C to instead vote for other candidates. See WinByGivingAwayVotesAlternative. Applicable to most rulesets.".into()),
                "WinByGivingAwayVotesAlternative" => Some("Like WinByGivingAwayVotes except candidate C wins a seat after persuading two voters who previously just voted for C to instead vote for A then B.".into()),
                _ => None,
            },
            reports:  match electorate { // TODO add report when done
                "DummyFirstPreference" | "DummyFirstPreferenceAlternative" | "DummyFirstPreference2" | "DummyFirstPreferenceAlternative2" | "WinByGivingAwayVotes" | "WinByGivingAwayVotesAlternative" => vec![],
                _ => vec![],
            },
        }
    }
}



pub trait SimpleExampleFromText {

    //
    // Functions you should implement
    //

    fn all_electorates() -> Vec<String>;

    fn get_raw_data_as_string(electorate: &str) -> anyhow::Result<&'static str>;

    fn rules(electorate: &str) -> AssociatedRules;
}

impl <T:SimpleExampleFromText> KnowsAboutRawMarkings for ExampleDataLoader<T> {} // empty body means doesn't do anything.
impl <T:SimpleExampleFromText> RawDataSource for ExampleDataLoader<T> {

    fn name(&self, electorate: &str) -> ElectionName {
        if let Ok(metadata) = self.read_raw_metadata(electorate) {
            metadata.name
        } else {
            ElectionName {
                year: "error".to_string(),
                authority: "Made up by Andrew Conway".to_string(),
                name: "Synthetic example".to_string(),
                electorate: "Corrupt".to_string(),
                modifications: vec![],
                comment: None,
            }
        }
    }

    fn candidates_to_be_elected(&self, electorate: &str) -> NumberOfCandidates {
        if let Ok(metadata) = self.read_raw_metadata(electorate) {
            if let Some(vacancies) = metadata.vacancies {
                return vacancies
            }
        }
        NumberOfCandidates(0)
    }

    fn ec_decisions(&self, _electorate: &str) -> TieResolutionsMadeByEC { Default::default() }
    fn excluded_candidates(&self, _electorate: &str) -> Vec<CandidateIndex> { Default::default()  }

    fn read_raw_data(&self, electorate: &str) -> anyhow::Result<ElectionData> {
        let json = T::get_raw_data_as_string(electorate)?;
        let data : ElectionData = serde_json::from_str(json)?;
        Ok(data)
    }

    fn all_electorates(&self) -> Vec<String> {
        T::all_electorates()
    }

    fn find_raw_data_file(&self, filename: &str) -> Result<PathBuf, MissingFile> {
        Err(MissingFile{
            file_name: filename.to_string(),
            where_to_get: "There are no accessible example datafiles at runtime. Go to the ConcreteSTV source and look in the examples directory".to_string(),
            where_to_get_is_exact_url: false,
        } )
    }

    fn read_raw_metadata(&self, state: &str) -> anyhow::Result<ElectionMetadata> {
        self.read_raw_data(state).map(|s|s.metadata)
    }

    fn copyright(&self) -> Copyright {
        Copyright {
            statement: Some("© Andrew Conway. See notes for more details.".into()),
            url: Some("https://vote.andrewconway.org".into()),
            license_name: Some("GNU Affero General Public License version 3".into()),
            license_url: Some("https://www.gnu.org/licenses/agpl-3.0.en.html".into())
        }
    }

    fn rules(&self, electorate: &str) -> AssociatedRules {
        T::rules(electorate)
    }

    fn read_official_dop_transcript(&self, _metadata: &ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        Err(anyhow!("No official DoP transcript available for synthetic examples"))
    }
}

