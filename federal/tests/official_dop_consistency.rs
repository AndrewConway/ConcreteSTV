// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This tests how the official transcripts compare to the rules, with no knowledge of the actual votes.

use federal::FederalRulesUsed2013;
use federal::parse::{FederalDataSource};
use stv::datasource_description::ElectionDataSource;
use stv::parse_util::{FileFinder};
use stv::preference_distribution::PreferenceDistributionRules;
use stv::verify_official_transcript::{distribute_preferences_using_official_results, veryify_official_dop_transcript};

fn test<Rules:PreferenceDistributionRules>(year:&str,state:&str) -> anyhow::Result<()> where <Rules as PreferenceDistributionRules>::Tally: Send+Sync+'static {
    let loader = FederalDataSource{}.get_loader_for_year(year,&FileFinder::find_ec_data_repository())?;
    let metadata = loader.read_raw_metadata(state)?;
    let official_transcript = loader.read_official_dop_transcript(&metadata)?;
    veryify_official_dop_transcript::<Rules>(&official_transcript,&metadata)?;
    let transcript = distribute_preferences_using_official_results::<Rules>(&official_transcript,&metadata)?;
    official_transcript.compare_with_transcript(&transcript);
    Ok(())
}

#[test]
#[allow(non_snake_case)]
fn test_ACT2013() { test::<FederalRulesUsed2013>("2013","ACT").unwrap() }
