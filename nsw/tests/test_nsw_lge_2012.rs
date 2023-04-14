// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use nsw::parse_lge::{get_nsw_lge_data_loader_2012};
use stv::parse_util::{FileFinder, RawDataSource};



#[test]
fn test_2012_plausible() {
    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let loader = get_nsw_lge_data_loader_2012(&finder).unwrap();
    println!("Made loader");
    assert_eq!(&loader.all_electorates()[0],"Albury City Council");
    for electorate in &loader.all_electorates() {
        println!("Testing Electorate {}",electorate);
        let _metadata = loader.read_raw_metadata(electorate).unwrap();
    }
}

