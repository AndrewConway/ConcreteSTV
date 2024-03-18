// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This tests how the official transcripts compare to the rules, with no knowledge of the actual votes.

use stv::datasource_description::ElectionDataSource;
use stv::parse_util::FileFinder;
use wa::parse_wa::WADataSource;


fn test_parse(year:&str) {
    let loader = WADataSource{}.get_loader_for_year(year,&FileFinder::find_ec_data_repository()).unwrap();
    for region in loader.all_electorates() {
        println!("Testing loading metadata for {} {}",region,year);
        let metadata = loader.read_raw_metadata(&region).unwrap();
        println!("{:?}",metadata);
        println!("Testing loading transcript for {} {}",region,year);
        let transcript = loader.read_official_dop_transcript(&metadata).unwrap();
        println!("Loaded {} counts",transcript.counts.len());
    }
}

#[test]
fn test_2005() { test_parse("2005"); }
#[test]
fn test_2008() { test_parse("2008"); }
#[test]
fn test_2013() { test_parse("2013"); }
#[test]
fn test_2017() { test_parse("2017"); }
#[test]
fn test_2021() { test_parse("2021"); }

#[test]
#[allow(non_snake_case)]
fn test_Agricultural_2017() {
    // assert_eq!(test::<WALegislativeCouncil>("2017","Agricultural").unwrap(),Ok(None));
    let loader = WADataSource{}.get_loader_for_year("2017",&FileFinder::find_ec_data_repository()).unwrap();
    let metadata = loader.read_raw_metadata("Agricultural").unwrap();
    println!("{:?}",metadata)
}

#[test]
#[allow(non_snake_case)]
fn test_Agricultural_2021() {
    // assert_eq!(test::<WALegislativeCouncil>("2017","Agricultural").unwrap(),Ok(None));
    let loader = WADataSource{}.get_loader_for_year("2021",&FileFinder::find_ec_data_repository()).unwrap();
    let metadata = loader.read_raw_metadata("Agricultural").unwrap();
    println!("{:?}",metadata);
    let transcript = loader.read_official_dop_transcript(&metadata).unwrap();
    transcript.print_table(&metadata);
}

