// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! This crate tries to provide a unified API to different file formats and counting algorithms.
//! It also contains the main binaries.

use std::num::ParseIntError;
use stv::ballot_metadata::CandidateIndex;

pub mod rules;
pub mod ec_data_source;

/// Utility that is helpful for parsing in clap a Vec<Vec<CandidateIndex>>.
pub fn try_parse_candidate_list(s:&str) -> Result<Vec<CandidateIndex>,ParseIntError> {
    s.split(',').map(|s|s.trim().parse::<CandidateIndex>()).collect()
}