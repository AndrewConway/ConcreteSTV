//! Some utility routines that make parsing files easier.


use crate::ballot_metadata::{Candidate, Party, CandidateIndex, PartyIndex, ElectionName, NumberOfCandidates};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{BufReader, Seek, BufRead, SeekFrom};
use crate::election_data::ElectionData;
use crate::tie_resolution::TieResolutionsMadeByEC;
use anyhow::anyhow;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

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

#[derive(Debug)]
pub struct MissingFile {
    pub file_name : String,
    pub where_to_get : String,
}

impl Display for MissingFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Missing file {} look in {}",self.file_name,self.where_to_get)
    }
}
impl Error for MissingFile {
}

pub trait RawDataSource {
    fn name(&self,electorate:&str) -> ElectionName;
    /// The number of candidates to be elected in this election.
    fn candidates_to_be_elected(&self,electorate:&str) -> NumberOfCandidates;
    /// Get tie breaking decisions made by the EC.
    fn ec_decisions(&self,electorate:&str) -> TieResolutionsMadeByEC;
    /// Get candidates that are excluded by default for whatever reason.
    fn excluded_candidates(&self,electorate:&str) -> Vec<CandidateIndex>;
    /// Read the data for a given electorate.
    fn read_raw_data(&self,electorate:&str) -> anyhow::Result<ElectionData>;
    /// Get a list of all the electorates
    fn all_electorates(&self) -> Vec<String>;
    /// Find a raw data file, or give a meaningful message about where it could be obtained from.
    fn find_raw_data_file(&self,filename:&str) -> Result<PathBuf,MissingFile>;

    fn load_cached_data(&self,electorate:&str) -> anyhow::Result<ElectionData> {
        match self.name(electorate).load_cached_data() {
            Ok(data) => Ok(data),
            Err(_) => {
                let data = self.read_raw_data(electorate)?;
                data.save_to_cache()?;
                Ok(data)
            }
        }
    }

    /// Like read_raw_data, but with a better error message for invalid electorates.
    fn read_raw_data_checking_electorate_valid(&self,electorate:&String) -> anyhow::Result<ElectionData> {
        if !self.all_electorates().contains(electorate) { Err(anyhow!("No such electorate as {}. Supported electorates are : {}.",electorate,self.all_electorates().join(", "))) }
        else { self.read_raw_data(electorate) }
    }

}

/// Datafiles from Electoral Commissions could be stored in the current working directory,
/// but may also be in some other (reference) folder. Alternatively, they could be in
/// some archive like xxx/Federal/2013/file_used_in_federal2013election.csv
/// A FileFinder will find a file in such a place.
#[derive(Debug,Clone)]
pub struct FileFinder {
    pub path : PathBuf,

}

impl FromStr for FileFinder {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(s);
        if !path.is_dir() { Err(format!("Path {} is not a readable directory",s))}
        else { Ok(FileFinder{path})}
    }
}

impl Default for FileFinder {
    fn default() -> Self {
        FileFinder{path:PathBuf::from(".")}
    }
}
impl FileFinder {

    /// Find where a file is, looking first in the directory this implies (self.path/filename),
    /// and secondly in self.path/archive_location/filename. If found in either it will
    /// return it, otherwise it will return an error message recommending looking for it
    /// in the given url.
    pub fn find_raw_data_file(&self,filename:&str,archive_location:&str,source_url:&str) -> Result<PathBuf,MissingFile> {
        let expect = self.path.join(filename);
        if expect.exists() { return Ok(expect) }
        let expect = self.path.join(archive_location).join(filename);
        if expect.exists() { return Ok(expect) }
        Err(MissingFile{ file_name: filename.to_string(), where_to_get: source_url.to_string() })
    }

    /// Used to find an archive for testing.
    pub fn find_ec_data_repository() -> FileFinder {
        for possible_path in vec![
            "votecounting/CountPreferentialVotes/Elections",
            "../votecounting/CountPreferentialVotes/Elections",
            "../../votecounting/CountPreferentialVotes/Elections",
            "../../../votecounting/CountPreferentialVotes/Elections"
        ] {
            if Path::new(possible_path).exists() { return FileFinder{path: PathBuf::from(possible_path)} }
        }
        println!("Warning - unable to find testing data archive");
        FileFinder{path: PathBuf::from(".")}
    }

}