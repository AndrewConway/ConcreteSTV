//! Some utility routines that make parsing files easier.


use crate::ballot_metadata::{Candidate, Party, CandidateIndex, PartyIndex};
use std::path::Path;
use std::fs::File;
use std::io::{BufReader, Seek, BufRead, SeekFrom};

/// A utility for helping to read a list of candidates and parties.
#[derive(Default)]
pub struct CandidateAndGroupInformationBuilder {
    pub candidates : Vec<Candidate>,
    //candidate_by_id : HashMap<String,CandidateIndex>,
    pub parties : Vec<GroupBuilder>,
}

pub struct GroupBuilder {
    pub name : String,
    pub group_id : String, // e.g. "A" or "UG"
    pub abbreviation : Option<String>,
    pub ticket_id : Option<String>, // the dummy candidate id for the ticket vote.
    pub tickets : Vec<Vec<CandidateIndex>>, // a list of tickets
}

/// Read a file, skipping the first line. This is useful for parsing CSV files where the
/// first line is some status message, which the csv crate does not deal with.
pub fn skip_first_line_of_file(path:&Path) -> anyhow::Result<File> {
    let file = File::open(path)?;
    // want to jump to the first newline. Simplest efficient way to do this is make a buffered reader to get the position...
    let mut buffered = BufReader::new(file);
    buffered.read_line(&mut String::new())?;
    let position = buffered.stream_position()?;
    let mut file = buffered.into_inner(); // get back the file.
    file.seek(SeekFrom::Start(position))?;
    Ok(file)
}

impl CandidateAndGroupInformationBuilder {

    pub fn extract_parties(&self) -> Vec<Party> {
        let mut res : Vec<Party> = self.parties.iter().map(|g|Party{
            column_id: g.group_id.clone(),
            name: g.name.clone(),
            abbreviation: g.abbreviation.clone(),
            atl_allowed: g.ticket_id.is_some(),
            candidates: vec![],
            tickets: g.tickets.clone(),
        }).collect();
        for candidate_index in 0..self.candidates.len() {
            let candidate = & self.candidates[candidate_index];
            if let Some(party) = candidate.party {
                res[party.0].candidates.push(CandidateIndex(candidate_index));
                assert_eq!(Some(res[party.0].candidates.len()),candidate.position);
            }
        }
        res
    }

    pub fn group_from_group_id(&self,group_id:&str) -> Option<PartyIndex> {
        self.parties.iter().position(|g|&g.group_id==group_id)
                           .map(|index|PartyIndex(index))
    }
}

