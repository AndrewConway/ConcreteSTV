// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This tests how the official transcripts compare to the rules, with no knowledge of the actual votes.

use federal::{FederalRulesPost2021, FederalRulesPre2021, FederalRulesUsed2013, FederalRulesUsed2016, FederalRulesUsed2019};
use federal::parse::{FederalDataSource};
use stv::ballot_metadata::CandidateIndex;
use stv::datasource_description::ElectionDataSource;
use stv::distribution_of_preferences_transcript::CountIndex;
use stv::official_dop_transcript::{DifferenceBetweenOfficialDoPAndComputed, DifferenceBetweenOfficialDoPAndComputedOnParticularCount};
use stv::official_dop_transcript::DifferenceBetweenOfficialDoPAndComputed::DifferentOnCount;
use stv::parse_util::{FileFinder};
use stv::preference_distribution::PreferenceDistributionRules;
use stv::tie_resolution::TieResolutionExplicitDecision;
use stv::verify_official_transcript::{distribute_preferences_using_official_results, veryify_official_dop_transcript};

/// Test a particular year & electorate against a particular set of rules.
/// Outermost error is IO type errors.
/// Innermost error is discrepancies with the official DoP.
fn test<Rules:PreferenceDistributionRules>(year:&str,state:&str) -> anyhow::Result<Result<Option<TieResolutionExplicitDecision>, DifferenceBetweenOfficialDoPAndComputed<Rules::Tally>>> where <Rules as PreferenceDistributionRules>::Tally: Send+Sync+'static {
    let loader = FederalDataSource{}.get_loader_for_year(year,&FileFinder::find_ec_data_repository())?;
    let metadata = loader.read_raw_metadata(state)?;
    let official_transcript = loader.read_official_dop_transcript(&metadata)?;
    veryify_official_dop_transcript::<Rules>(&official_transcript,&metadata)?;
    let transcript = distribute_preferences_using_official_results::<Rules>(&official_transcript,&metadata)?;
    Ok(official_transcript.compare_with_transcript_checking_for_ec_decisions(&transcript,false))
}

#[test]
#[allow(non_snake_case)]
fn test_ACT2013() {
    assert_eq!(test::<FederalRulesUsed2013>("2013","ACT").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2013","ACT").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_NT2013() {
    assert_eq!(test::<FederalRulesUsed2013>("2013","NT").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2013","NT").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_QLD2013() {
    assert_eq!(test::<FederalRulesUsed2013>("2013","QLD").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2013","QLD").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_WA2013() {
    assert_eq!(test::<FederalRulesUsed2013>("2013","WA").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2013","WA").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_SA2013() {
    assert_eq!(test::<FederalRulesUsed2013>("2013","SA").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2013","SA").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Tas2013() {
    assert_eq!(test::<FederalRulesUsed2013>("2013","TAS").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2013","TAS").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Vic2013() {
    assert_eq!(test::<FederalRulesUsed2013>("2013","VIC").unwrap(),Ok(None));
    // In 2013 the AEC applied "better" multiple tie resolution rules than the legislation.
    let expected_error = Err(DifferentOnCount(CountIndex(59), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ExcludedCandidatesUnordered(vec![CandidateIndex(1)],vec![CandidateIndex(45)])));
    assert_eq!(test::<FederalRulesPre2021>("2013","VIC").unwrap(),expected_error);
}
#[test]
#[allow(non_snake_case)]
fn test_NSW2013() {
    assert_eq!(test::<FederalRulesUsed2013>("2013","NSW").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2013","NSW").unwrap(),Ok(None));
}


#[test]
#[allow(non_snake_case)]
fn test_ACT2016() {
    assert_eq!(test::<FederalRulesUsed2016>("2016","ACT").unwrap(),Ok(None));
    // In 2016 the AEC did not apply the multiple exclusion rules.
    let expected_error = Err(DifferentOnCount(CountIndex(10), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ExcludedCandidatesUnordered(vec![CandidateIndex(13)],vec![CandidateIndex(9),CandidateIndex(13)])));
    assert_eq!(test::<FederalRulesUsed2013>("2016","ACT").unwrap(),expected_error);
    assert_eq!(test::<FederalRulesPre2021>("2016","ACT").unwrap(),expected_error);
}
#[test]
#[allow(non_snake_case)]
fn test_NT2016() {
    assert_eq!(test::<FederalRulesUsed2016>("2016","NT").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesUsed2013>("2016","NT").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2016","NT").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_QLD2016() {
    assert_eq!(test::<FederalRulesUsed2016>("2016","QLD").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesUsed2013>("2016","QLD").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2016","QLD").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_WA2016() {
    assert_eq!(test::<FederalRulesUsed2016>("2016","WA").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesUsed2013>("2016","WA").unwrap(),Ok(None));
    // In 2016 the AEC applied "better" multiple tie resolution rules than the legislation.
    let expected_error = Err(DifferentOnCount(CountIndex(48), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ExcludedCandidatesUnordered(vec![CandidateIndex(42)],vec![CandidateIndex(53)])));
    assert_eq!(test::<FederalRulesPre2021>("2016","WA").unwrap(),expected_error);
}
#[test]
#[allow(non_snake_case)]
fn test_SA2016() {
    assert_eq!(test::<FederalRulesUsed2016>("2016","SA").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesUsed2013>("2016","SA").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2016","SA").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Tas2016() {
    assert_eq!(test::<FederalRulesUsed2016>("2016","TAS").unwrap(),Ok(None));
    // In 2016 the AEC did not apply the multiple exclusion rules.
    let expected_error = Err(DifferentOnCount(CountIndex(9), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ExcludedCandidatesUnordered(vec![CandidateIndex(31)],vec![CandidateIndex(31),CandidateIndex(38)])));
    assert_eq!(test::<FederalRulesUsed2013>("2016","TAS").unwrap(),expected_error);
    assert_eq!(test::<FederalRulesPre2021>("2016","TAS").unwrap(),expected_error);
}

#[test]
#[allow(non_snake_case)]
fn test_Vic2016() {
    assert_eq!(test::<FederalRulesUsed2016>("2016","VIC").unwrap(),Ok(None));
    // In 2016 the AEC did not apply the multiple exclusion rules.
    let expected_error = Err(DifferentOnCount(CountIndex(12), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ExcludedCandidatesUnordered(vec![CandidateIndex(64)],vec![CandidateIndex(64),CandidateIndex(70)])));
    assert_eq!(test::<FederalRulesUsed2013>("2016","VIC").unwrap(),expected_error);
    assert_eq!(test::<FederalRulesPre2021>("2016","VIC").unwrap(),expected_error);
}
#[test]
#[allow(non_snake_case)]
fn test_NSW2016() {
    assert_eq!(test::<FederalRulesUsed2016>("2016","NSW").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesUsed2013>("2016","NSW").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2016","NSW").unwrap(),Ok(None));
}


#[test]
#[allow(non_snake_case)]
fn test_ACT2019() {
    assert_eq!(test::<FederalRulesUsed2019>("2019","ACT").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesUsed2016>("2019","ACT").unwrap(),Ok(None));
    // In 2019 the AEC did not apply the multiple exclusion rules.
    let expected_error = Err(DifferentOnCount(CountIndex(2), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ExcludedCandidatesUnordered(vec![CandidateIndex(7)],vec![CandidateIndex(5),CandidateIndex(7),CandidateIndex(9),CandidateIndex(11),CandidateIndex(15)])));
    assert_eq!(test::<FederalRulesPre2021>("2019","ACT").unwrap(),expected_error);
}
#[test]
#[allow(non_snake_case)]
fn test_NT2019() {
    assert_eq!(test::<FederalRulesUsed2019>("2019","NT").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesUsed2016>("2019","NT").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2019","NT").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_QLD2019() {
    assert_eq!(test::<FederalRulesUsed2019>("2019","QLD").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesUsed2016>("2019","QLD").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2019","QLD").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_WA2019() {
    assert_eq!(test::<FederalRulesUsed2019>("2019","WA").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesUsed2016>("2019","WA").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2019","WA").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_SA2019() {
    assert_eq!(test::<FederalRulesUsed2019>("2019","SA").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesUsed2016>("2019","SA").unwrap(),Ok(None));
    // In 2019 the AEC did not apply the multiple exclusion rules.
    let expected_error = Err(DifferentOnCount(CountIndex(79), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ExcludedCandidatesUnordered(vec![CandidateIndex(1)],vec![CandidateIndex(1),CandidateIndex(23),CandidateIndex(39),CandidateIndex(41)])));
    assert_eq!(test::<FederalRulesPre2021>("2019","SA").unwrap(),expected_error);
}
#[test]
#[allow(non_snake_case)]
fn test_Tas2019() {
    assert_eq!(test::<FederalRulesUsed2019>("2019","TAS").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesUsed2016>("2019","TAS").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPre2021>("2019","TAS").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Vic2019() {
    assert_eq!(test::<FederalRulesUsed2019>("2019","VIC").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesUsed2016>("2019","VIC").unwrap(),Ok(None));
    // In 2019 the AEC did not apply the multiple exclusion rules.
    let expected_error = Err(DifferentOnCount(CountIndex(5), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ExcludedCandidatesUnordered(vec![CandidateIndex(26)],vec![CandidateIndex(26),CandidateIndex(77)])));
    assert_eq!(test::<FederalRulesPre2021>("2019","VIC").unwrap(),expected_error);
}
#[test]
#[allow(non_snake_case)]
fn test_NSW2019() {
    assert_eq!(test::<FederalRulesUsed2019>("2019","NSW").unwrap(),Ok(None));
    // In 2019 the AEC did not finish exclusions before applying subsection 18.
    let expected_error = Err(DifferentOnCount(CountIndex(428), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ElectedCandidatesUnordered(vec![CandidateIndex(9),CandidateIndex(17)],vec![])));
    assert_eq!(test::<FederalRulesUsed2016>("2019","NSW").unwrap(),expected_error);
    assert_eq!(test::<FederalRulesPre2021>("2019","NSW").unwrap(),expected_error);
}


#[test]
#[allow(non_snake_case)]
fn test_ACT2022() {
    assert_eq!(test::<FederalRulesUsed2019>("2022","ACT").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPost2021>("2022","ACT").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_NT2022() {
    assert_eq!(test::<FederalRulesUsed2019>("2022","NT").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPost2021>("2022","NT").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_QLD2022() {
    assert_eq!(test::<FederalRulesUsed2019>("2022","QLD").unwrap(),Ok(None));
    // In 2022 the AEC did not finish exclusions before applying subsection 18.
    let expected_error = Err(DifferentOnCount(CountIndex(265), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::ElectedCandidatesUnordered(vec![CandidateIndex(62),CandidateIndex(66)],vec![])));
    assert_eq!(test::<FederalRulesPost2021>("2022","QLD").unwrap(),expected_error);
}
#[test]
#[allow(non_snake_case)]
fn test_WA2022() {
    assert_eq!(test::<FederalRulesUsed2019>("2022","WA").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPost2021>("2022","WA").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_SA2022() {
    assert_eq!(test::<FederalRulesUsed2019>("2022","SA").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPost2021>("2022","SA").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Tas2022() {
    assert_eq!(test::<FederalRulesUsed2019>("2022","TAS").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPost2021>("2022","TAS").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Vic2022() {
    assert_eq!(test::<FederalRulesUsed2019>("2022","VIC").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPost2021>("2022","VIC").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_NSW2022() {
    assert_eq!(test::<FederalRulesUsed2019>("2022","NSW").unwrap(),Ok(None));
    assert_eq!(test::<FederalRulesPost2021>("2022","NSW").unwrap(),Ok(None));
}
