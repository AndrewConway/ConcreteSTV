//! Information about the contest, such as candidates.

use serde::{Serialize,Deserialize};
use std::fmt;
use std::path::PathBuf;
use crate::election_data::ElectionData;
use std::fs::File;
use std::collections::HashMap;

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
    pub results : Option<Vec<CandidateIndex>>
}

/// Documentation on where the data files used for this data came from.
#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct DataSource {
    pub url : String,
    pub files : Vec<String>,
    pub comments : Option<String>
}

impl ElectionMetadata {
    pub fn party(&self,index:PartyIndex) -> &Party { &self.parties[index.0] }
    pub fn candidate(&self,index:CandidateIndex) -> &Candidate { &self.candidates[index.0] }
    pub fn get_candidate_name_lookup(&self) -> HashMap<String,CandidateIndex> {
        let mut res = HashMap::default();
        for i in 0..self.candidates.len() {
            res.insert(self.candidates[i].name.clone(),CandidateIndex(i));
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
    pub modifications : Vec<String>
}

impl ElectionName {
    pub fn human_readable_name(&self) -> String {
        format!("{} {} election for {}.{}",self.year,self.name,self.electorate,self.modifications.join(" & "))
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
    pub tickets : Vec<Vec<CandidateIndex>>
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
