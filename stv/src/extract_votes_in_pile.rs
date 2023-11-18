// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Utilities designed to extract the votes in a particular pile


use std::fs::File;
use std::path::PathBuf;
use std::sync::Mutex;
use crate::ballot_metadata::CandidateIndex;
use crate::election_data::ElectionData;

/// Which votes you want to extract.
pub enum WhatToExtract {
    /// The votes used to elect a particular candidate, as defined in Schedule 4, part 4.3 Casual Vacancies in _Electoral Act 1992_
    ACTVotesUsedToElectCandidate(CandidateIndex)
}

pub enum WhatToDoWithExtractedVotes {
    SaveToFile(PathBuf),
    CallFunction(Box<Mutex<dyn FnMut(ElectionData)>>),
}

pub struct ExtractionRequest {
    pub what_to_extract : WhatToExtract,
    pub what_to_do_with_it : WhatToDoWithExtractedVotes,
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
