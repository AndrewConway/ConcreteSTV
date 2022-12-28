// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This tests how the official transcripts compare to the rules, with no knowledge of the actual votes.

use stv::official_dop_transcript::{DifferenceBetweenOfficialDoPAndComputed, test_official_dop_without_actual_votes};
use stv::preference_distribution::PreferenceDistributionRules;
use stv::tie_resolution::TieResolutionExplicitDecision;
use vic::parse_vic::VicDataSource;
use vic::Vic2018LegislativeCouncil;

/// Test a particular year & electorate against a particular set of rules.
/// Outermost error is IO type errors.
/// Innermost error is discrepancies with the official DoP.
fn test<Rules:PreferenceDistributionRules>(year:&str,state:&str) -> anyhow::Result<Result<Option<TieResolutionExplicitDecision>, DifferenceBetweenOfficialDoPAndComputed<Rules::Tally>>> where <Rules as PreferenceDistributionRules>::Tally: Send+Sync+'static {
    test_official_dop_without_actual_votes::<Rules,_>(&VicDataSource{},year,state,false)
}

#[test]
#[allow(non_snake_case)]
fn test_EastMet2014() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2014","Eastern Metropolitan Region").unwrap(),Ok(None));
}


#[test]
#[allow(non_snake_case)]
fn test_EastVic2014() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2014","Eastern Victoria Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_NorthMet2014() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2014","Northern Metropolitan Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_NorthVic2014() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2014","Northern Victoria Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_SEMet2014() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2014","South-Eastern Metropolitan Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_SouthMet2014() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2014","Southern Metropolitan Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_WestMet2014() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2014","Western Metropolitan Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_WestVic2014() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2014","Western Victoria Region").unwrap(),Ok(None));
}


#[test]
#[allow(non_snake_case)]
fn test_EastMet2018() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2018","Eastern Metropolitan Region").unwrap(),Ok(None));
}


#[test]
#[allow(non_snake_case)]
fn test_EastVic2018() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2018","Eastern Victoria Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_NorthMet2018() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2018","Northern Metropolitan Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_NorthVic2018() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2018","Northern Victoria Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_SEMet2018() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2018","South-Eastern Metropolitan Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_SouthMet2018() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2018","Southern Metropolitan Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_WestMet2018() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2018","Western Metropolitan Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_WestVic2018() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2018","Western Victoria Region").unwrap(),Ok(None));
}


#[test]
#[allow(non_snake_case)]
fn test_NEMet2022() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2022","North-Eastern Metropolitan Region").unwrap(),Ok(None));
}


#[test]
#[allow(non_snake_case)]
fn test_EastVic2022() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2022","Eastern Victoria Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_NorthMet2022() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2022","Northern Metropolitan Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_NorthVic2022() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2022","Northern Victoria Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_SEMet2022() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2022","South-Eastern Metropolitan Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_SouthMet2022() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2022","Southern Metropolitan Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_WestMet2022() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2022","Western Metropolitan Region").unwrap(),Ok(None));
}

#[test]
#[allow(non_snake_case)]
fn test_WestVic2022() {
    assert_eq!(test::<Vic2018LegislativeCouncil>("2022","Western Victoria Region").unwrap(),Ok(None));
}

