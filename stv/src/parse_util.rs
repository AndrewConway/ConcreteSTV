// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Some utility routines that make parsing files easier.


use std::env::temp_dir;
use crate::ballot_metadata::{Candidate, Party, CandidateIndex, PartyIndex, ElectionName, NumberOfCandidates, ElectionMetadata};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{BufReader, Seek, BufRead, SeekFrom, Read};
use crate::election_data::ElectionData;
use crate::tie_resolution::{TieResolutionAtom, TieResolutionExplicitDecisionInCount, TieResolutionsMadeByEC};
use anyhow::{anyhow, Context};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::process::Command;
use std::str::FromStr;
use std::sync::Mutex;
use reqwest::Url;
use crate::ballot_paper::{RawBallotMarkings};
use crate::compare_transcripts::{DeltasInCandidateLists, DifferentCandidateLists};
use crate::datasource_description::{AssociatedRules, Copyright};
use crate::errors_btl::ObviousErrorsInBTLVotes;
use crate::find_vote::{FindMyVoteQuery, FindMyVoteResult};
use crate::official_dop_transcript::OfficialDistributionOfPreferencesTranscript;
use crate::preference_distribution::PreferenceDistributionRules;

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
            // println!("Candidate index {} name {} party {:?} position {:?}",candidate_index,candidate.name,candidate.party,candidate.position);
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
    pub where_to_get_is_exact_url : bool,
}

#[derive(Debug)]
pub struct MissingAlternateNamedFiles {
    pub alternates : Vec<MissingFile>,
}

impl Display for MissingFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.where_to_get_is_exact_url {
            write!(f,"Missing file {} from URL {} : try wget -O {} {}",self.file_name,self.where_to_get,self.file_name,self.where_to_get)
        } else {
            write!(f,"Missing file {} look in {}",self.file_name,self.where_to_get)
        }
    }
}
impl Error for MissingFile {
}
impl Display for MissingAlternateNamedFiles {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"There is a missing file which may have alternate names.")?;
        for mf in &self.alternates {
            write!(f," May be file {} from {}",mf.file_name,mf.where_to_get)?;
        }
        Ok(())
    }
}
impl Error for MissingAlternateNamedFiles {
}



pub trait RawDataSource : KnowsAboutRawMarkings {
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
                let data = self.read_raw_data_best_quality(electorate)?;
                data.save_to_cache()?;
                Ok(data)
            }
        }
    }

    /// Read the data for a given electorate. Usually this just calls  read_raw_data,
    /// but it can be overridden to call something else, such as to (expensively) deduce EC decisions.
    fn read_raw_data_best_quality(&self,electorate:&str) -> anyhow::Result<ElectionData> { self.read_raw_data(electorate) }


    /// Like read_raw_data, but with a better error message for invalid electorates.
    fn read_raw_data_checking_electorate_valid(&self,electorate:&str) -> anyhow::Result<ElectionData> {
        if !self.all_electorates().iter().any(|s|s==electorate) { Err(self.bad_electorate(electorate)) }
        else { self.read_raw_data(electorate) }
    }

    fn bad_electorate(&self,electorate:&str) -> anyhow::Error {
        anyhow!("No such electorate as {}. Supported electorates are : {}.",electorate,self.all_electorates().join(", "))
    }

    /// if it is possible to run the iterate_over_raw_markings function
    fn can_iterate_over_raw_btl_preferences(&self) -> bool { false }
    /// if it is possible to run the read_raw_data function
    fn can_load_full_data(&self,_state:&str) -> bool { true }
    fn read_raw_metadata(&self,state:&str) -> anyhow::Result<ElectionMetadata>;
    fn copyright(&self) -> Copyright;
    fn rules(&self,electorate:&str) -> AssociatedRules;
    fn can_read_raw_markings(&self) -> bool { false}
    /// Get the official transcript for the election. May not be available for all electorates.
    fn read_official_dop_transcript(&self,metadata:&ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript>;

}

pub trait KnowsAboutRawMarkings {
    fn find_my_vote(&self,_electorate:&str,_query:&FindMyVoteQuery) -> anyhow::Result<FindMyVoteResult> { Err(anyhow!("Reading raw markings not supported.")) }
    fn find_btl_errors(&self,_electorate:&str) -> anyhow::Result<ObviousErrorsInBTLVotes> { Err(anyhow!("Reading raw markings not supported.")) }
}

impl <T:CanReadRawMarkings+RawDataSource> KnowsAboutRawMarkings for T {
    fn find_my_vote(&self,electorate:&str,query:&FindMyVoteQuery) -> anyhow::Result<FindMyVoteResult> {
        FindMyVoteResult::compute(self,electorate,query)
    }
    fn find_btl_errors(&self,electorate:&str) -> anyhow::Result<ObviousErrorsInBTLVotes> {
        ObviousErrorsInBTLVotes::compute(self,electorate)
    }
}
/*
impl <T:CantReadRawMarkings> KnowsAboutRawMarkings for T {
    fn can_read_raw_markings(&self) -> bool { false }
}
*/

pub trait CantReadRawMarkings {

}
pub trait CanReadRawMarkings {
    fn iterate_over_raw_markings<F>(&self,_electorate:&str,_callback:F)  -> anyhow::Result<ElectionMetadata>
        where F:FnMut(&RawBallotMarkings,RawBallotPaperMetadata)
    {
        Err(anyhow!("Iterating over raw btl preferences not supported."))
    }
}



/// Like read_raw_data, except also try to deduce the tie breaking decisions that were used by the electoral commission.
/// This is a powerful function, but it will be slow and panic if anything goes even slightly wrong.
/// Also deduce the offical results, possibly reordering to better match the actual order here.
pub fn read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions<Rules:PreferenceDistributionRules,Source:RawDataSource>(loader:&Source, electorate: &str) -> anyhow::Result<ElectionData> where <Rules as PreferenceDistributionRules>::Tally : Send+Sync+'static {
    println!("Trying to deduce ec resolutions for {}",electorate);
    let mut data = loader.read_raw_data(electorate)?;
    if electorate.ends_with("Mayoral") { return Ok(data); } // don't have DOP file for mayoral elections. Besides, STV is not necessarily exactly a generalization of IRV... e.g. early termination conditions.
    // let mut tie_resolutions = TieResolutionsMadeByEC::default();
    let official_transcript = loader.read_official_dop_transcript(&data.metadata)?;
    // let mut initial_ec_decisions = data.metadata.tie_resolutions.clone(); // should be empty, unless we set it up some way else.
    data.metadata.tie_resolutions=TieResolutionsMadeByEC::default(); // Get rid of less fine grained decisions that may be entered.
    loop {
        println!("Looping...");
        let transcript = data.distribute_preferences::<Rules>(&mut Randomness::ReverseDonkeyVote);
        if let Some(decision) = official_transcript.compare_with_transcript_checking_for_ec_decisions(&transcript,false).context("Trying to determine EC decisions")? {
            println!("Observed tie resolution {}", decision.decision);
            assert!(!decision.decision.is_reverse_donkey_vote(), "favoured candidate should be lower as higher candidates are assumed favoured.");
            data.metadata.tie_resolutions.tie_resolutions.push(TieResolutionAtom::ExplicitDecision(decision));
        } else {
            // rebuild the list of decisions based upon the values in the transcript. This will get any decisions that "happened naturally", and put them all in order.
            let mut final_ec_decisions = TieResolutionsMadeByEC::default();
            for (count_index,count) in transcript.counts.iter().enumerate() {
                for decision in &count.decisions {
                    let ecdecision = TieResolutionAtom::ExplicitDecision(TieResolutionExplicitDecisionInCount {decision: decision.clone(),came_up_in:Some(CountIndex(count_index))});
                    final_ec_decisions.tie_resolutions.push(ecdecision);
                }
            }
            // check the newly deduced list contains every decision previously deduced.
            for decision in &data.metadata.tie_resolutions.tie_resolutions {
                if !final_ec_decisions.tie_resolutions.contains(decision) {
                    panic!("EC decision {:?} was not in the re-deduced set",decision);
                }
            }
            data.metadata.tie_resolutions= final_ec_decisions; // overwrite to get decisions in count order.
            // if there is no "winning candidates" section in the metadata, add it.
            if let Some(official_results) = data.metadata.results.clone() {
                // check the official results match the actual
                let diffs : DeltasInCandidateLists = DifferentCandidateLists{list1:official_results,list2:transcript.elected.clone()}.into();
                if !diffs.is_empty() { return Err(anyhow!("Elected candidates in official transcript differ from in metadata : {}",diffs.pretty_print(&data.metadata))); }
            }
            data.metadata.results = Some(transcript.elected); // replace existing even if it matches to get better order.
            return Ok(data);
        }
    }
}



/// Raw ballot paper metadata is a slice of pairs of strings.
/// The first string is the name of the metadata (typically a constant static string)
/// The second string is the value of the corresponding metadata.
pub type RawBallotPaperMetadata<'a> = &'a[(&'a str,&'a str)];
/*
/// The type of functions that can be used as a callback when iterating over all btl preferences.
/// The first argument is a list of strings, being the marks next to the candidates in candidate order.
/// The third argument is information about the current vote being iterated over, such as polling station. It is a set of pairs of metadata, the first in the pair being the name of the metadata and the second being the actual metadata.
pub trait RawPreferencesCallbackFunction : FnMut(&RawBallotMarkings,&[(&str,&str)])-> () {

}

/// We don't want to explicitly implement RawPreferencesCallbackFunction in client code;
/// this is a trick (see https://www.worthe-it.co.za/blog/2017-01-15-aliasing-traits-in-rust.html)
/// to get an effective trait alias in rust. If rust ever makes trait aliases stable, this should
/// be replaced by a trait alias.
impl <T> RawPreferencesCallbackFunction for T where T : FnMut(&RawBallotMarkings,&[(&str,&str)])-> () {}
*/

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
        Err(MissingFile{ file_name: filename.to_string(), where_to_get: source_url.to_string(), where_to_get_is_exact_url : false })
    }

    pub fn find_raw_data_file_with_extra_url_info(&self,filename:&str,archive_location:&str,source_url_base:&str,source_url_relative:&str) -> Result<PathBuf,MissingFile> {
        let expect = self.path.join(filename);
        if expect.exists() { return Ok(expect) }
        let expect = self.path.join(archive_location).join(filename);
        // println!("Looking in {}",expect.to_string_lossy());
        if expect.exists() { return Ok(expect) }
        let where_to_get : String = if source_url_relative.is_empty() { source_url_base.to_string() } else {
            let url = Url::parse(source_url_base).unwrap().join(source_url_relative).unwrap();
            url.as_str().to_string()
        };
        Err(MissingFile{ file_name: filename.to_string(), where_to_get, where_to_get_is_exact_url: true })
    }


    /// find an expected path in the current dir. If not there, check the parent, and continue recursively. Return the full path if found.
    fn look_in_ancestral_paths(expected_path:&str) -> Option<PathBuf> {
        let mut search = Path::new(".").canonicalize().ok();
        while let Some(p) = search {
            let possible = p.join(expected_path);
            if possible.exists() { return Some(possible)}
            search = p.parent().map(|p|p.to_path_buf());
        }
        None
    }

    /// Used to find an archive for testing.
    pub fn find_ec_data_repository() -> FileFinder {
        let expected_path = "vote_data/Elections";
        if let Some(path) = Self::look_in_ancestral_paths(expected_path) {
            FileFinder{path}
        } else {
            println!("Warning - unable to find testing data archive");
            FileFinder{path: PathBuf::from(".")}
        }
    }

}

/// Read a file to a string. Like file.read_to_string but doesn't need a provided buffer.
pub fn file_to_string(file:&mut File) -> anyhow::Result<String> {
    let mut res = String::new();
    file.read_to_string(&mut res)?;
    Ok(res)
}

// Read a file to a string. Like file_to_string but Windows 1252 character encoding instead of utf-8.
pub fn file_to_string_windows_1252(file:&mut File) -> anyhow::Result<String> {
    let mut bytes = Vec::new();
    file.read_to_end( &mut bytes)?;
    let (cow,_,had_errors) = encoding_rs::WINDOWS_1252.decode(&bytes);
    if had_errors { return Err(anyhow!("Had errors decoding")) }
    Ok(cow.to_string())
}


use once_cell::sync::Lazy;
use crate::distribution_of_preferences_transcript::CountIndex;
use crate::random_util::Randomness;

// The openoffice CLI seems to be unreliable if running multiple simultaneously. This is used as a lock.
static OPENOFFICE_CLI_LOCK: Lazy<Mutex<u64>> = Lazy::new(||Mutex::new(1u64));

/// Parse an xslx file by the horrible convoluted method of running libreoffice to convert it
/// into a csv file in the temporary directory, reading that into an array of strings, and
/// then deleting the csv file. Requires openoffice to be installed!
///
/// It is generally better to use a library like calamine, but if that doesn't work for some reason,
/// this is a fall back.
///
pub fn parse_xlsx_by_converting_to_csv_using_openoffice(path:&PathBuf) -> anyhow::Result<Vec<Vec<String>>> {
    // run open office
//    println!("Converting {:?}",path);
    let lock = OPENOFFICE_CLI_LOCK.lock().unwrap();
    let temp_path = temp_dir();
    let convert_to = if path.ends_with(".xlsx") {"csv"} else {"csv:Text - txt - csv (StarCalc):44,34,0,1,,0"}; // needed for .xls files to get unicode correctly.
    Command::new("libreoffice").arg("--headless").arg("--convert-to").arg(convert_to).arg(path).arg("--outdir").arg(&temp_path).output().context("Problem running libreoffice")?;
    let filename = path.file_name().ok_or_else(||anyhow!("Provided path {:?} doesn't seem to have a file name",&path))?;
    let mut output_path = temp_path.join(filename);
    output_path.set_extension("csv");
//    println!("Created at {:?}",output_path);
    let mut res = vec![];
    {
        let reader = csv::ReaderBuilder::new().has_headers(false).from_path(&output_path)?;
        for record in reader.into_records() {
            let record=record?;
            let cols = record.iter().map(|s|s.to_string()).collect::<Vec<_>>();
            res.push(cols);
        }
    }
    std::fs::remove_file(output_path)?;
    assert_eq!(lock.count_ones(),1);
    Ok(res)
}

/// A wrapper around parse_xlsx_by_converting_to_csv_using_openoffice result that has an API somewhat like Calamine's for cases where it doesn't work.
pub struct CalamineLikeWrapper {
    pub contents : Vec<Vec<String>>,
}

#[derive(Debug,Clone)]
pub struct CalamineLikeCellWrapper {
    pub contents : String,
}
impl CalamineLikeWrapper {
    pub fn open(path:&PathBuf) -> anyhow::Result<Self> {
        Ok(CalamineLikeWrapper{ contents: parse_xlsx_by_converting_to_csv_using_openoffice(path).with_context(|| format!("Processing {}",path.to_string_lossy()))? })
    }
    pub fn height(&self) -> usize { self.contents.len() }
    pub fn width(&self) -> usize { self.contents.iter().map(|v|v.len()).max().unwrap_or(0) }
    pub fn get_value(&self,coords:(u32,u32)) -> Option<CalamineLikeCellWrapper> {
        let (row,col) = coords;
        let row = row as usize;
        let col = col as usize;
        if row < self.contents.len() && col < self.contents[row].len() {
            let contents = &self.contents[row][col];
            if contents.is_empty() { None } else { Some(CalamineLikeCellWrapper{contents:contents.to_string()}) }
        } else { None }
    }
}
impl CalamineLikeCellWrapper {
    pub fn get_string(&self) -> Option<String> { Some(self.contents.to_string()) }
}