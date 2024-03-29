// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Information about the contest, such as candidates.

use serde::{Serialize,Deserialize};
use std::fmt;
use std::path::PathBuf;
use crate::election_data::ElectionData;
use std::fs::File;
use std::collections::HashMap;
use std::iter::Map;
use std::ops::{Range, Sub};
use std::str::FromStr;
use thiserror::Error;
use crate::tie_resolution::TieResolutionsMadeByEC;

/// a candidate, referred to by position on the ballot paper, 0 being first
#[derive(Clone, Copy, PartialEq, Eq, Hash,Serialize,Deserialize)]
pub struct CandidateIndex(pub usize);
// type alias really, don't want long display
impl fmt::Display for CandidateIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}
// type alias really, don't want long display
impl fmt::Debug for CandidateIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "#{}", self.0) }
}

impl FromStr for CandidateIndex {
    type Err = <usize as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(CandidateIndex(usize::from_str(s)?))
    }
}



/// a party, referred to by position on the ballot paper, 0 being first
#[derive(Clone, Copy, PartialEq, Eq, Hash,Serialize,Deserialize)]
pub struct PartyIndex(pub usize);

// type alias really, don't want long display
impl fmt::Display for PartyIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}
// type alias really, don't want long display
impl fmt::Debug for PartyIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "#{}", self.0) }
}


/// Represent a number of candidates. E.g. number of seats, number of remaining seats.
#[derive(Clone, Copy, PartialEq, Eq, Hash,Serialize,Deserialize,Ord, PartialOrd)]
pub struct NumberOfCandidates(pub usize);

impl FromStr for NumberOfCandidates {
    type Err = <usize as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> { Ok(NumberOfCandidates(usize::from_str(s)?)) }
}
// type alias really, don't want long display
impl fmt::Display for NumberOfCandidates {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}
// type alias really, don't want long display
impl fmt::Debug for NumberOfCandidates {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "#{}", self.0) }
}

impl Sub for NumberOfCandidates {
    type Output = NumberOfCandidates;
    fn sub(self, rhs: Self) -> Self::Output { NumberOfCandidates(self.0-rhs.0) }
}

/// Information about the election
#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct ElectionMetadata {
    pub name : ElectionName,
    pub candidates : Vec<Candidate>,
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub parties : Vec<Party>,
    /// where the data came from, such as a URL.
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub source : Vec<DataSource>,
    /// the official results, if available.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub results : Option<Vec<CandidateIndex>>,
    /// the number of positions to be filled, default.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub vacancies : Option<NumberOfCandidates>,
    /// the number of eligible voters.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub enrolment : Option<NumberOfCandidates>,
    /// Another number of positions to be filled. Useful for a double dissolution, where two counts are held, some candidates to get longer terms.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub secondary_vacancies : Option<NumberOfCandidates>,
    /// Candidates who are usually excluded, e.g. if they died on the election day or were ruled ineligible to stand. Looking at you 2016.
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub excluded : Vec<CandidateIndex>,
    #[serde(flatten)]
    pub tie_resolutions : TieResolutionsMadeByEC,
}

/// Documentation on where the data files used for this data came from.
#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct DataSource {
    pub url : String,
    pub files : Vec<String>,
    pub comments : Option<String>,
}

impl DataSource {
    pub fn new(url:&str,path:&PathBuf) -> Self {
        DataSource{ url:url.to_string(), files:vec![path.file_name().as_ref().unwrap().to_string_lossy().to_string()],comments:None}
    }
}

impl ElectionMetadata {
    pub fn party(&self,index:PartyIndex) -> &Party { &self.parties[index.0] }
    pub fn candidate(&self,index:CandidateIndex) -> &Candidate { &self.candidates[index.0] }
    /// Get a hashmap going from candidate name to index
    pub fn get_candidate_name_lookup(&self) -> HashMap<String,CandidateIndex> {
        let mut res = HashMap::default();
        for i in 0..self.candidates.len() {
            res.insert(self.candidates[i].name.clone(),CandidateIndex(i));
        }
        res
    }
    /// Get a hashmap going from candidate name to index, converting SMITH Fred to Fred SMITH
    pub fn get_candidate_name_lookup_with_capital_letters_afterwards(&self) -> HashMap<String,CandidateIndex> {
        let mut res = HashMap::default();
        fn is_surname(s:&str) -> bool {
            let no_mac = s.trim_start_matches("Mac").trim_start_matches("Mc");
            s.len()>1 && no_mac.len()>0 && no_mac.chars().all(|c|!c.is_lowercase()) // one letter is probably an initial.
        }
        for i in 0..self.candidates.len() {
            let name_components = self.candidates[i].name.split_ascii_whitespace().collect::<Vec<_>>();
            let capital_components = name_components.iter().take_while(|&&s|is_surname(s)).collect::<Vec<_>>();
            let lower_case_components = name_components.iter().skip(capital_components.len()).map(|&s|s);
            let reordered_name = lower_case_components.chain(capital_components.iter().map(|&&s|s)).collect::<Vec<_>>().join(" ");
            res.insert(reordered_name,CandidateIndex(i));
        }
        res
    }
    /// Get a hashmap going from party name to index
    pub fn get_party_name_lookup(&self) -> HashMap<String,PartyIndex> {
        let mut res = HashMap::default();
        for i in 0..self.parties.len() {
            res.insert(self.parties[i].name.clone(),PartyIndex(i));
        }
        res
    }
    /// Get a hashmap going from party column id to index
    pub fn get_party_id_lookup(&self) -> HashMap<String,PartyIndex> {
        let mut res = HashMap::default();
        for i in 0..self.parties.len() {
            res.insert(self.parties[i].column_id.clone(),PartyIndex(i));
        }
        res
    }
    /// Get a hashmap going from candidate name to index. Include both candidate name and no_comma_name
    pub fn get_candidate_name_lookup_multiple_ways(&self) -> HashMap<String,CandidateIndex> {
        let mut res = HashMap::default();
        for i in 0..self.candidates.len() {
            res.insert(self.candidates[i].name.clone(),CandidateIndex(i));
            res.insert(self.candidates[i].no_comma_name(),CandidateIndex(i));
        }
        res
    }
    pub fn get_candidate_ec_id_lookup(&self) -> HashMap<String,CandidateIndex> {
        let mut res = HashMap::default();
        for i in 0..self.candidates.len() {
            if let Some(id) = self.candidates[i].ec_id.as_ref() {
                res.insert(id.to_string(),CandidateIndex(i));
            }
        }
        res
    }
    /// An iterator over all the candidate indices starting at 0.
    pub fn candidate_indices(&self) -> Map<Range<usize>, fn(usize) -> CandidateIndex> { (0..self.candidates.len()).map(|i|CandidateIndex(i)) }

    pub fn candidate_list_to_string(&self,list : &[CandidateIndex]) -> String {
        list.iter().map(|&c|self.candidate(c).name.as_str()).collect::<Vec<_>>().join(", ")
    }
    pub fn party_list_to_string(&self,list : &[PartyIndex]) -> String {
        list.iter().map(|&c|self.party(c).best_name()).collect::<Vec<_>>().join(", ")
    }
}

/// Which election it was.
#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct ElectionName {
    /// The year this election was held
    pub year : String,
    /// The name of the authority running the election, e.g. AEC
    pub authority : String,
    /// the overall name of the election, e.g. Federal
    pub name : String,
    /// region in this contest, e.g. Vic
    pub electorate : String,
    /// modifications made to this data, e.g. simulating errors, hackers. Usually empty.
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub modifications : Vec<String>,
    /// Whatever you want.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub comment : Option<String>,
}

impl ElectionName {
    pub fn human_readable_name(&self) -> String {
        format!("{} {} election for {}.{}",self.year,self.name,self.electorate,self.modifications.join(" & "))
    }

    /// An identifier Name_Year_Electorate that could be used as a filename component for this.
    pub fn identifier(&self) -> String {
        self.name.clone()+"_"+&self.year+"_"+&self.electorate+&self.modifications.join(",")
    }

    pub fn cache_file_name(&self) -> PathBuf {
        let path = PathBuf::from("Cache");
        path.join(&self.name).join(&self.year).join(self.electorate.clone()+&self.modifications.join(",")+".stv")
    }

    pub fn load_cached_data(&self) -> std::io::Result<ElectionData> {
        let name = self.cache_file_name();
        let file = File::open(name)?;
        Ok(serde_json::from_reader(file)?)
    }
}

/// information about a party in the contest. This may be used as a pseudo-party, such as "ungrouped"
#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct Party {
    /// The name of the column on the ballot paper, typically a letter.
    pub column_id : String,
    /// The name of the party
    pub name : String,
    /// an abbreviation for the party
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub abbreviation : Option<String>,
    /// true if one is allowed to vote atl for this party. "Ungrouped" it is false for, also conceivably some rare other situations (for instance, in a ticket election, where the party did not submit a ticket).
    pub atl_allowed : bool,
    /// the candidates in this party, in preference order.
    pub candidates : Vec<CandidateIndex>,
    /// the group voting tickets for this party, if any.
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub tickets : Vec<Vec<CandidateIndex>>,
}

impl Party {
    pub fn best_name(&self) -> &str {
        if self.name.is_empty() { self.column_id.as_str() } else {self.name.as_str()}
    }
}

/// information about a candidate in the contest.
#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct Candidate {
    pub name : String,
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub party : Option<PartyIndex>,
    // position on the party ticket. 1 means first place.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub position : Option<usize>,
    // Electoral Commission internal identifier.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub ec_id : Option<String>,
}

impl Candidate {
    /// if the candidate name is "Surname, first", change to "first Surname"
    pub fn no_comma_name(&self) -> String {
        if let Some((surname,first)) = self.name.split_once(',') {
            first.trim().to_string()+" "+surname.trim()
        } else { self.name.clone() }
    }

    pub fn from_name(name:&str) -> Self { Candidate{name:name.to_string(),party:None,position:None,ec_id:None}}
}

/// There are lots of places where one needs to parse some file to extract a list of candidates
/// and parties in order to build the ElectionMetadata. These typically come accross a list
/// of parties and candidates intermingled. Very frequently in these files the following
/// things are all true
/// * The parties are listed in ballot paper order
/// * The candidates are listed in ballot paper order
/// * Candidates are listed after the party they are in.
///
/// These parsing routines have a lot in common, and this code is designed to encapsulate
/// the common code. Convenience and correctness is more important than efficiency as the metadata tends
/// to be small.
///
/// It would of course be ideal to refactor all the existing parsing code to use this. Maybe I will get around to that some day.
#[derive(Debug,Default,Clone)]
pub struct CandidateAndPartyBuilder {
    pub candidates : Vec<Candidate>,
    pub parties : Vec<Party>,
    pub source : Vec<DataSource>,
    pub results : Option<Vec<CandidateIndex>>,
}
/// An internal error indicating a problem parsing a metadata file
#[derive(Error, Debug)]
pub enum ParseMetadataError {
    #[error("there were no parties when one was expected in CandidateAndPartyBuilder")]
    PartyExpectedButNotAvailable,
    #[error("could not find candidate name : {0}")]
    UnknownCandidateName(String),
}

impl CandidateAndPartyBuilder {
    pub fn last_party(&self) -> Result<&Party, ParseMetadataError> { self.parties.last().ok_or(ParseMetadataError::PartyExpectedButNotAvailable {}) }
    pub fn last_party_mut(&mut self) -> Result<&mut Party, ParseMetadataError> { self.parties.last_mut().ok_or(ParseMetadataError::PartyExpectedButNotAvailable {}) }

    /// Add a new party. Candidates will be added later.
    pub fn add_party(&mut self,column_id : &str,name:&str,abbreviation:Option<&str>,atl_allowed:bool) {
        self.parties.push(Party{
            column_id: column_id.to_string(),
            name: name.to_string(),
            abbreviation: abbreviation.map(|s|s.to_string()),
            atl_allowed,
            candidates: vec![],
            tickets: vec![],
        })
    }

    /// Add a new candidate to the last party. Assumes that this candidate is next in position for the candidates.
    /// alternate_name is not currently used but may be added in the future if it is common. Possibly a list of aliases would be better?
    pub fn add_candidate_to_last_party(&mut self,name:&str,ec_id:Option<&str>,_alternate_name:Option<&str>) -> Result<(), ParseMetadataError> {
        let candidate_index = CandidateIndex(self.candidates.len());
        self.last_party_mut()?.candidates.push(candidate_index);
        self.candidates.push(Candidate{
            name : name.to_string(),
            party: Some(PartyIndex(self.parties.len()-1)),
            position: Some(self.last_party()?.candidates.len()),
            ec_id: ec_id.map(|s|s.to_string()),
        });
        Ok(())
    }

    /// Returns true if there is a last party added and it has the provided name.
    pub fn last_party_is_called(&self,name:&str) -> bool {
        match self.parties.last() {
            None => false,
            Some(party) => party.name.as_str()==name,
        }
    }

    pub fn add_source(&mut self,url:&str,path:&PathBuf) {
        self.source.push(DataSource::new(url,path))
    }
    pub fn add_source_different_filename(&mut self,url:&str,filename:&str) {
        self.source.push(DataSource{
            url: url.to_string(),
            files: vec![filename.to_string()],
            comments: None,
        })
    }

    pub fn candidate_index(&self,candidate_name:&str) -> Result<CandidateIndex,ParseMetadataError> {
        for (index,candidate) in self.candidates.iter().enumerate() {
            if candidate_name==candidate.name.as_str() { return Ok(CandidateIndex(index))}
        }
        Err(ParseMetadataError::UnknownCandidateName(candidate_name.to_string()))
    }

    pub fn declare_elected(&mut self,candidate_name:&str) -> Result<(),ParseMetadataError> {
        let candidate = self.candidate_index(candidate_name)?;
        self.results.get_or_insert_with(||vec![]).push(candidate);
        Ok(())
    }
}
