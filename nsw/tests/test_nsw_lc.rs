// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This does some very early tests for NSW Legislative Council.


#[cfg(test)]
mod tests {
    use nsw::nsw_random_rules::{NSWECRandomLC2015, NSWECRandomLC2019};
    use stv::parse_util::{FileFinder, RawDataSource};
    use nsw::parse_lc::{get_nsw_lc_data_loader_2015, get_nsw_lc_data_loader_2019, get_nsw_lc_data_loader_2023, NSWLCDataSource};
    use stv::official_dop_transcript::{DifferenceBetweenOfficialDoPAndComputed, test_official_dop_without_actual_votes};
    use stv::preference_distribution::PreferenceDistributionRules;
    use stv::tie_resolution::TieResolutionExplicitDecisionInCount;


    #[test]
    fn test_2015_data() {
        let finder = FileFinder::find_ec_data_repository();
        println!("Found files at {:?}",finder.path);
        let loader = get_nsw_lc_data_loader_2015(&finder).unwrap();
        println!("Made loader");
        let metadata = loader.read_raw_metadata("").unwrap();
        println!("{:?}",metadata);
        let data = loader.read_raw_data("").unwrap();
        data.print_summary();
        let official = loader.read_official_dop_transcript(&metadata).unwrap();
        assert!(official.quota.is_some());
        assert_eq!(391,official.counts.len());
    }

    #[test]
    fn test_2019_data() {
        let finder = FileFinder::find_ec_data_repository();
        println!("Found files at {:?}",finder.path);
        let loader = get_nsw_lc_data_loader_2019(&finder).unwrap();
        println!("Made loader");
        let metadata = loader.read_raw_metadata("").unwrap();
        println!("{:?}",metadata);
        let data = loader.read_raw_data("").unwrap();
        data.print_summary();
        let official = loader.read_official_dop_transcript(&metadata).unwrap();
        assert!(official.quota.is_some());
        assert_eq!(343,official.counts.len());
        assert_eq!("Some(0.8688)",format!("{:?}",official.counts[1].transfer_value));
    }

    #[test]
    fn test_2023_metadata_and_dop() {
        let finder = FileFinder::find_ec_data_repository();
        println!("Found files at {:?}",finder.path);
        let loader = get_nsw_lc_data_loader_2023(&finder).unwrap();
        println!("Made loader");
        let metadata = loader.read_raw_metadata("").unwrap();
        println!("{:?}",metadata);
        //let data = loader.read_raw_data("").unwrap();
        //data.print_summary();
        let official = loader.read_official_dop_transcript(&metadata).unwrap();
        assert!(official.quota.is_some());
        assert_eq!(287,official.counts.len());
        assert_eq!("Some(0.8751)",format!("{:?}",official.counts[1].transfer_value));
    }

    #[test]
    fn test_2015_internally_consistent() {
        assert_eq!(test_internally_consistent::<NSWECRandomLC2015>("2015").unwrap(),Ok(None));
    }

    #[test]
    fn test_2019_internally_consistent() {
        assert_eq!(test_internally_consistent::<NSWECRandomLC2019>("2019").unwrap(),Ok(None));
    }

    #[test]
    fn test_2023_internally_consistent() {
        assert_eq!(test_internally_consistent::<NSWECRandomLC2019>("2023").unwrap(),Ok(None));
    }


    /// Test a particular year & electorate against a particular set of rules.
    /// Outermost error is IO type errors.
    /// Innermost error is discrepancies with the official DoP.
    fn test_internally_consistent<Rules:PreferenceDistributionRules>(year:&str) -> anyhow::Result<Result<Option<TieResolutionExplicitDecisionInCount>, DifferenceBetweenOfficialDoPAndComputed<Rules::Tally>>> where <Rules as PreferenceDistributionRules>::Tally: Send+Sync+'static {
        test_official_dop_without_actual_votes::<Rules,_>(&NSWLCDataSource{},year,"",true)
    }

}

