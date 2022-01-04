// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Code to compare two transcripts to see if they are the same


use crate::ballot_metadata::{CandidateIndex, ElectionMetadata};
use crate::distribution_of_preferences_transcript::{CountIndex, Transcript};
use std::cmp::min;
use serde::{Serialize,Deserialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// The result of comparing two transcripts, in order of most
/// serious to least serious. The most serious is reported.
#[derive(Clone,Debug,Serialize,Deserialize,Eq, PartialEq)]
pub enum DifferenceBetweenTranscripts {
    DifferentCandidatesElected(DifferentCandidateLists),
    CandidatesOrderedDifferentWay(DifferentCandidateLists),
    DifferentValues(CountIndex),
    DifferentNumberOfCounts,
    Same
}

impl Display for DifferenceBetweenTranscripts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DifferenceBetweenTranscripts::DifferentCandidatesElected(way) => write!(f,"*** DIFFERENT CANDIDATES ELECTED : {}",way),
            DifferenceBetweenTranscripts::CandidatesOrderedDifferentWay(way) => write!(f,"Candidates elected different order : {}",way),
            DifferenceBetweenTranscripts::DifferentValues(c) => write!(f,"Different values at count {}",c.0+1),
            DifferenceBetweenTranscripts::DifferentNumberOfCounts => write!(f,"Different number of counts"),
            DifferenceBetweenTranscripts::Same => f.write_str("Same"),
        }

    }
}

#[derive(Clone,Debug,Serialize,Deserialize,Eq, PartialEq)]
pub struct DifferentCandidateLists {
    pub list1 : Vec<CandidateIndex>,
    pub list2 : Vec<CandidateIndex>,
}

impl Display for DifferentCandidateLists {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{} vs {}",self.list1.iter().map(|c|c.to_string()).collect::<Vec<_>>().join(","),self.list2.iter().map(|c|c.to_string()).collect::<Vec<_>>().join(","))
    }
}

/// Describe the difference between two lists of candidate indices. Order is not considered (and indeed is supressed to make equality comparison easier).
#[derive(Clone,Debug,Serialize,Deserialize,Eq,PartialEq)]
pub struct DeltasInCandidateLists {
    pub common : Vec<CandidateIndex>,
    pub list1only : Vec<CandidateIndex>,
    pub list2only : Vec<CandidateIndex>,
}

impl From<DifferentCandidateLists> for DeltasInCandidateLists {
    fn from(cl: DifferentCandidateLists) -> Self {
        let mut common : Vec<CandidateIndex> = cl.list1.iter().cloned().filter(|c|cl.list2.contains(c)).collect();
        common.sort_by_key(|c|c.0);
        let mut list1only : Vec<CandidateIndex> = cl.list1.iter().cloned().filter(|c|!cl.list2.contains(c)).collect();
        list1only.sort_by_key(|c|c.0);
        let mut list2only : Vec<CandidateIndex> = cl.list2.iter().cloned().filter(|c|!cl.list1.contains(c)).collect();
        list2only.sort_by_key(|c|c.0);
        DeltasInCandidateLists{common,list1only,list2only}
    }
}

pub fn pretty_print_candidate_list(candidates:&[CandidateIndex],metadata:&ElectionMetadata) -> String {
    format!("[{:?}]",candidates.iter().map(|&c|&metadata.candidate(c).name).collect::<Vec<_>>())
}

impl DeltasInCandidateLists {
    /// See if the two lists were actually the same - that is list1only
    pub fn is_empty(&self) -> bool { self.list1only.is_empty() && self.list2only.is_empty() }

    pub fn pretty_print(&self,metadata : &ElectionMetadata) -> String {
        format!("Common {} Different {} vs {}",pretty_print_candidate_list(&self.common,metadata),pretty_print_candidate_list(&self.list1only,metadata),pretty_print_candidate_list(&self.list2only,metadata))
    }
}


pub fn compare_transcripts<Tally:PartialEq+Clone+Display+FromStr>(transcript1:&Transcript<Tally>,transcript2:&Transcript<Tally>) -> DifferenceBetweenTranscripts {
    // first compare who was elected.
    if transcript1.elected!=transcript2.elected { // High priority!
        let dcl = DifferentCandidateLists{list1:transcript1.elected.clone(),list2:transcript2.elected.clone()};
        if transcript1.elected.iter().all(|c|transcript2.elected.contains(c)) { DifferenceBetweenTranscripts::CandidatesOrderedDifferentWay(dcl) }
        else { DifferenceBetweenTranscripts::DifferentCandidatesElected(dcl)}
    } else { // same candidates elected.
        for count_index in 0..min(transcript1.counts.len(),transcript2.counts.len()) {
            let count1 = &transcript1.counts[count_index];
            let count2 = &transcript2.counts[count_index];
            if count1.elected!=count2.elected || count1.not_continuing!=count2.not_continuing || count1.status!=count2.status { return DifferenceBetweenTranscripts::DifferentValues(CountIndex(count_index))}
        }
        if transcript1.counts.len()==transcript2.counts.len() { DifferenceBetweenTranscripts::Same } else { DifferenceBetweenTranscripts::DifferentNumberOfCounts }
    }
}