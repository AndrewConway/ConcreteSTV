// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This does some very early tests for NSW Legislative Council.


#[cfg(test)]
mod tests {
    use stv::parse_util::{FileFinder, RawDataSource};
    use nsw::parse_lc::{get_nsw_lc_data_loader_2015, get_nsw_lc_data_loader_2019};

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


}