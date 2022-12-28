// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This tests how the official transcripts compare to the rules, with no knowledge of the actual votes.

use std::str::FromStr;
use std::vec;
use act::{ACT2020, ACT2021, ACTPre2020};
use act::parse::ACTDataSource;
use stv::distribution_of_preferences_transcript::CountIndex;
use stv::fixed_precision_decimal::FixedPrecisionDecimal;
use stv::official_dop_transcript::{DifferenceBetweenOfficialDoPAndComputed, DifferenceBetweenOfficialDoPAndComputedOnParticularCount, ECTally, test_official_dop_without_actual_votes};
use stv::official_dop_transcript::DifferenceBetweenOfficialDoPAndComputed::DifferentOnCount;
use stv::preference_distribution::PreferenceDistributionRules;
use stv::signed_version::SignedVersion;
use stv::tie_resolution::TieResolutionExplicitDecision;

/// Test a particular year & electorate against a particular set of rules.
/// Outermost error is IO type errors.
/// Innermost error is discrepancies with the official DoP.
fn test<Rules:PreferenceDistributionRules>(year:&str,state:&str) -> anyhow::Result<Result<Option<TieResolutionExplicitDecision>, DifferenceBetweenOfficialDoPAndComputed<Rules::Tally>>> where <Rules as PreferenceDistributionRules>::Tally: Send+Sync+'static {
    test_official_dop_without_actual_votes::<Rules,_>(&ACTDataSource{},year,state,false)
}

#[test]
#[allow(non_snake_case)]
fn test_Brindabella2008() {
    assert_eq!(test::<ACTPre2020>("2008","Brindabella").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Ginninderra2008() {
    assert_eq!(test::<ACTPre2020>("2008","Ginninderra").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Molonglo2008() {
    assert_eq!(test::<ACTPre2020>("2008","Molonglo").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_Brindabella2012() {
    assert_eq!(test::<ACTPre2020>("2012","Brindabella").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Ginninderra2012() {
    assert_eq!(test::<ACTPre2020>("2012","Ginninderra").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Molonglo2012() {
    assert_eq!(test::<ACTPre2020>("2012","Molonglo").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_Brindabella2016() {
    assert_eq!(test::<ACTPre2020>("2016","Brindabella").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Ginninderra2016() {
    assert_eq!(test::<ACTPre2020>("2016","Ginninderra").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Kurrajong2016() {
    assert_eq!(test::<ACTPre2020>("2016","Kurrajong").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Murrumbidgee2016() {
    assert_eq!(test::<ACTPre2020>("2016","Murrumbidgee").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Yerrabi2016() {
    assert_eq!(test::<ACTPre2020>("2016","Yerrabi").unwrap(),Ok(None));
}


#[test]
#[allow(non_snake_case)]
fn test_Brindabella2020() {
    assert_eq!(test::<ACT2020>("2020.0","Brindabella").unwrap(),Ok(None));
    // In 2020, Elections ACT considered first preference votes with transfer value 1 to have a different TV than other sources of transfer value 1, and so distributed then on a separate count, reducing the number of votes processed on count 21 (CountIndex 20). See section 3 of our report "Errors in the ACT’s electronic counting code" in the reports folder.
    // This could be manifested in a variety of errors - the particular one here comes from the oracle not knowing where to assign votes when using an inappropriate distribution of preferences.
    let expected_error = Err(DifferentOnCount(CountIndex(20), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalExhausted(ECTally::from(2862.),FixedPrecisionDecimal::<6>::from(2943))));
    assert_eq!(test::<ACT2021>("2020.0","Brindabella").unwrap(),expected_error);
    // converse of previous error.
    let expected_error = Err(DifferentOnCount(CountIndex(20), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalExhausted(ECTally::from(2898.),FixedPrecisionDecimal::<6>::from(2817))));
    assert_eq!(test::<ACT2020>("2020","Brindabella").unwrap(),expected_error);
    assert_eq!(test::<ACT2021>("2020","Brindabella").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Ginninderra2020() {
    assert_eq!(test::<ACT2020>("2020.0","Ginninderra").unwrap(),Ok(None));
    // In 2020 exhausted votes were rounded down to integers gratuitously. See section 7.2 of our report
    let expected_error = Err(DifferentOnCount(CountIndex(24), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalExhausted(ECTally::from(1108.),FixedPrecisionDecimal::<6>::from_str("1108.85").unwrap())));
    assert_eq!(test::<ACT2021>("2020.0","Ginninderra").unwrap(),expected_error);
    let expected_error = Err(DifferentOnCount(CountIndex(24), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalExhausted(ECTally::from(1108.85),FixedPrecisionDecimal::<6>::from_str("1108").unwrap())));
    assert_eq!(test::<ACT2020>("2020","Ginninderra").unwrap(),expected_error);
    assert_eq!(test::<ACT2021>("2020","Ginninderra").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Kurrajong2020() {
    // There is a particularly bizarre error in the From counts field that I suspect may be hand edited as the formatting is different to others, so I haven't tried to reproduce. This error was noticed after we wrote the report.
    let expected_error = Err(DifferentOnCount(CountIndex(55), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::FromCounts(vec![CountIndex(54)],vec![CountIndex(51)])));
    assert_eq!(test::<ACT2020>("2020.0","Kurrajong").unwrap(),expected_error);
    // In 2020, votes were rounded to nearest, rather than rounded down. See section 4 of our report.
    let expected_error = Err(DifferentOnCount(CountIndex(1), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalRounding(ECTally::from(0.),SignedVersion::from(FixedPrecisionDecimal::<6>::from_str("0.000015").unwrap()))));
    assert_eq!(test::<ACT2021>("2020.0","Kurrajong").unwrap(),expected_error);
    // converse of prior error.
    let expected_error = Err(DifferentOnCount(CountIndex(1), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalRounding(ECTally::from(0.000015),SignedVersion::from(FixedPrecisionDecimal::<6>::from_str("0").unwrap()))));
    assert_eq!(test::<ACT2020>("2020","Kurrajong").unwrap(),expected_error);
    assert_eq!(test::<ACT2021>("2020","Kurrajong").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Murrumbidgee2020() {
    assert_eq!(test::<ACT2020>("2020.0","Murrumbidgee").unwrap(),Ok(None));
    // In 2020, votes were rounded to nearest, rather than rounded down. See section 4 of our report.
    let expected_error = Err(DifferentOnCount(CountIndex(21), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalRounding(ECTally::from(0.000001),SignedVersion::from(FixedPrecisionDecimal::<6>::from_str("0.000003").unwrap()))));
    assert_eq!(test::<ACT2021>("2020.0","Murrumbidgee").unwrap(),expected_error);
    // converse of prior error.
    let expected_error = Err(DifferentOnCount(CountIndex(21), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalRounding(ECTally::from(0.000003),SignedVersion::from(FixedPrecisionDecimal::<6>::from_str("0.000001").unwrap()))));
    assert_eq!(test::<ACT2020>("2020","Murrumbidgee").unwrap(),expected_error);
    assert_eq!(test::<ACT2021>("2020","Murrumbidgee").unwrap(),Ok(None));
}
#[test]
#[allow(non_snake_case)]
fn test_Yerrabi2020() {
    assert_eq!(test::<ACT2020>("2020.0","Yerrabi").unwrap(),Ok(None));
    // In 2020, votes were rounded to nearest, rather than rounded down. See section 4 of our report.
    let expected_error = Err(DifferentOnCount(CountIndex(12), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalRounding(ECTally::from(0.000002),SignedVersion::from(FixedPrecisionDecimal::<6>::from_str("0.000004").unwrap()))));
    assert_eq!(test::<ACT2021>("2020.0","Yerrabi").unwrap(),expected_error);
    // converse of prior error.
    let expected_error = Err(DifferentOnCount(CountIndex(12), None, DifferenceBetweenOfficialDoPAndComputedOnParticularCount::TallyTotalRounding(ECTally::from(0.000004),SignedVersion::from(FixedPrecisionDecimal::<6>::from_str("0.000002").unwrap()))));
    assert_eq!(test::<ACT2020>("2020","Yerrabi").unwrap(),expected_error);
    assert_eq!(test::<ACT2021>("2020","Yerrabi").unwrap(),Ok(None));
}