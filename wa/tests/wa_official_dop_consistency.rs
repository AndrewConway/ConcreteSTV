// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This tests how the official transcripts compare to the rules, with no knowledge of the actual votes.

use stv::official_dop_transcript::{DifferenceBetweenOfficialDoPAndComputed, test_official_dop_without_actual_votes};
use stv::preference_distribution::PreferenceDistributionRules;
use stv::tie_resolution::TieResolutionExplicitDecisionInCount;
use wa::parse_wa::WADataSource;
use wa::WALegislativeCouncil;

/// Test a particular year & electorate against a particular set of rules.
/// Outermost error is IO type errors.
/// Innermost error is discrepancies with the official DoP.
fn test<Rules:PreferenceDistributionRules>(year:&str,region:&str) -> anyhow::Result<Result<Option<TieResolutionExplicitDecisionInCount>, DifferenceBetweenOfficialDoPAndComputed<Rules::Tally>>> where <Rules as PreferenceDistributionRules>::Tally: Send+Sync+'static {
    test_official_dop_without_actual_votes::<Rules,_>(&WADataSource{},year,region,true)
}

#[test]
#[allow(non_snake_case)]
fn test_Agricultural2008() {
    assert_eq!(test::<WALegislativeCouncil>("2008","Agricultural").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_EastMetropolitan2008() {
    assert_eq!(test::<WALegislativeCouncil>("2008","East Metropolitan").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_MiningandPastoral2008() {
    assert_eq!(test::<WALegislativeCouncil>("2008","Mining and Pastoral").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_NorthMetropolitan2008() {
    assert_eq!(test::<WALegislativeCouncil>("2008","North Metropolitan").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_SouthMetropolitan2008() {
    assert_eq!(test::<WALegislativeCouncil>("2008","South Metropolitan").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_SouthWest2008() {
    assert_eq!(test::<WALegislativeCouncil>("2008","South West").unwrap(),Ok(None));
}

