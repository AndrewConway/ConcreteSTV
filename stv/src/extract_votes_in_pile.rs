// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Utilities designed to extract the votes in a particular pile


use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use crate::ballot_metadata::CandidateIndex;
use crate::election_data::ElectionData;

/// Which votes you want to extract from the transcript.
#[derive(Debug,Clone)]
pub enum WhatToExtract {
    /// The votes used to elect a particular candidate, as defined in Schedule 4, part 4.3 Casual Vacancies in _Electoral Act 1992_
    ACTVotesUsedToElectCandidate(CandidateIndex)
}

#[derive(thiserror::Error, Debug)]
pub enum ExtractError {
    #[error("unknown thing to extract")]
    UnknownThingToExtract,
    #[error("could not parse as a candidate number")]
    CouldNotParseCandidateNumber,
    #[error("unknown thing to do with extracted votes")]
    UnknownThingToDo,
    #[error("an extraction request should be what to extract, followed by a semicolon, followed by what to do with it, and there was no semicolon")]
    ExpectingSemicolonInExtractionRequest,
}
impl FromStr for WhatToExtract {
    type Err = ExtractError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(candidate) = s.strip_prefix("UsedToElectACT:") {
            candidate.parse::<CandidateIndex>().map_err(|_| ExtractError::CouldNotParseCandidateNumber).map(|c|WhatToExtract::ACTVotesUsedToElectCandidate(c))
        } else {
            Err(ExtractError::UnknownThingToExtract)
        }
    }
}

#[derive(Clone)]
pub enum WhatToDoWithExtractedVotes {
    SaveToFile(PathBuf),
    CallFunction(Arc<Mutex<dyn FnMut(ElectionData)+ Send + Sync>>),
}

impl FromStr for WhatToDoWithExtractedVotes {
    type Err = ExtractError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(filename) = s.strip_prefix("file:") {
            Ok(WhatToDoWithExtractedVotes::SaveToFile(PathBuf::from(filename)))
        } else {
            Err(ExtractError::UnknownThingToExtract)
        }
    }
}

/// A request to extract some set of votes from the transcript, and do something with it.
#[derive(Clone)]
pub struct ExtractionRequest {
    pub what_to_extract : WhatToExtract,
    pub what_to_do_with_it : WhatToDoWithExtractedVotes,
}

impl FromStr for ExtractionRequest {
    type Err = ExtractError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((what_to_extract,what_to_do_with_it)) = s.split_once(';') {
            let what_to_extract : WhatToExtract = what_to_extract.parse()?;
            let what_to_do_with_it : WhatToDoWithExtractedVotes = what_to_do_with_it.parse()?;
            Ok(ExtractionRequest{ what_to_extract, what_to_do_with_it })
        } else {
            Err(ExtractError::UnknownThingToExtract)
        }
    }
}


impl WhatToDoWithExtractedVotes {
    pub fn do_it(&self,data:ElectionData) {
        match self {
            WhatToDoWithExtractedVotes::SaveToFile(path) => {
                let out = File::create(path).expect("Error creating file to write out election data");
                serde_json::to_writer(out,&data).expect("Error writing out election data");
            }
            WhatToDoWithExtractedVotes::CallFunction(f) => { f.lock().unwrap()(data) }
        }
    }
}
