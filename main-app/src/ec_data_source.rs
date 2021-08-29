// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use std::fmt::{Display, Formatter};
use std::str::FromStr;
use stv::election_data::ElectionData;
use federal::parse::{get_federal_data_loader_2013, get_federal_data_loader_2016, get_federal_data_loader_2019};
use stv::parse_util::{RawDataSource, FileFinder};

#[derive(Copy, Clone)]
pub enum ECDataSource {
    AEC2013,
    AEC2016,
    AEC2019,
}

impl FromStr for ECDataSource {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AEC2013" => Ok(ECDataSource::AEC2013),
            "AEC2016" => Ok(ECDataSource::AEC2016),
            "AEC2019" => Ok(ECDataSource::AEC2019),
            _ => Err("No such rule supported")
        }
    }
}

impl Display for ECDataSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ECDataSource::AEC2013 => "AEC2013",
            ECDataSource::AEC2016 => "AEC2016",
            ECDataSource::AEC2019 => "AEC2019",
        };
        f.write_str(s)
    }
}

impl ECDataSource {

    pub fn load(&self,electorate:&String,finder:&FileFinder) -> anyhow::Result<ElectionData> {
        match self {
            ECDataSource::AEC2013 => get_federal_data_loader_2013(finder).read_raw_data_checking_electorate_valid(electorate),
            ECDataSource::AEC2016 => get_federal_data_loader_2016(finder).read_raw_data_checking_electorate_valid(electorate),
            ECDataSource::AEC2019 => get_federal_data_loader_2019(finder).read_raw_data_checking_electorate_valid(electorate),
        }
    }
}
