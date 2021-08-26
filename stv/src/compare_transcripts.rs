//! Code to compare two transcripts to see if they are the same


use crate::ballot_metadata::CandidateIndex;
use crate::distribution_of_preferences_transcript::{CountIndex, Transcript};
use std::cmp::min;
use serde::{Serialize,Deserialize};
use std::fmt::{Display, Formatter};

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


pub fn compare_transcripts<Tally:PartialEq>(transcript1:&Transcript<Tally>,transcript2:&Transcript<Tally>) -> DifferenceBetweenTranscripts {
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