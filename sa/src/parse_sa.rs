// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! This is an attempt to parse the SA data files, which is problematic as they don't have everything.


use std::borrow::Cow;
use std::fs::File;
use std::path::PathBuf;
use anyhow::anyhow;
use zip::read::ZipFile;
use stv::ballot_metadata::{CandidateAndPartyBuilder, CandidateIndex, ElectionMetadata, ElectionName, NumberOfCandidates};
use stv::datasource_description::{AssociatedRules, Copyright, ElectionDataSource};
use stv::election_data::ElectionData;
use stv::official_dop_transcript::{OfficialDistributionOfPreferencesTranscript};
use stv::parse_util::{file_to_string_windows_1252, FileFinder, KnowsAboutRawMarkings, MissingFile, RawDataSource};
use stv::tie_resolution::TieResolutionsMadeByEC;
use stv::download::{CacheDir};

pub fn get_sa_data_loader_2022(finder:&FileFinder) -> anyhow::Result<SADataLoader> {
    SADataLoader::new(finder, "2022") // Note - not all needed files are public. "https://www.ecsa.sa.gov.au/elections/past-state-election-results?view=article&id=521:results&catid=12:elections"
}


/// Do not use on website as there is nothing useful here.
pub struct SADataSource {}

impl ElectionDataSource for SADataSource {
    fn name(&self) -> Cow<'static, str> { "SA Legislative Council".into() }
    fn ec_name(&self) -> Cow<'static, str> { "Electoral Commission South Australia".into() }
    fn ec_url(&self) -> Cow<'static, str> { "https://www.ecsa.sa.gov.au/".into() }
    fn years(&self) -> Vec<String> { vec!["2022".to_string()] }
    fn get_loader_for_year(&self,year: &str,finder:&FileFinder) -> anyhow::Result<Box<dyn RawDataSource+Send+Sync>> {
        Ok(Box::new(SADataLoader::new(finder, year)?))
    }
}

pub struct SADataLoader {
    finder : FileFinder,
    archive_location : String,
    year : String,
    page_url : String,
    cache : CacheDir,
}

impl KnowsAboutRawMarkings for SADataLoader {}

impl RawDataSource for SADataLoader {
    fn name(&self, electorate: &str) -> ElectionName {
        ElectionName {
            year: self.year.clone(),
            authority: "Electoral Commission South Australia".to_string(),
            name: "SA Legislative Council".to_string(),
            electorate: electorate.to_string(),
            modifications: vec![],
            comment: None,
        }
    }

    fn candidates_to_be_elected(&self, _region: &str) -> NumberOfCandidates {
        match self.year.as_str() {
            "2022" => NumberOfCandidates(11),
            _ => NumberOfCandidates(11), // assumed to not change
        }
    }

    fn ec_decisions(&self, _electorate: &str) -> TieResolutionsMadeByEC {
        Default::default()
    }

    fn excluded_candidates(&self, _electorate: &str) -> Vec<CandidateIndex> {
        Default::default()
    }

    fn find_raw_data_file(&self, filename: &str) -> Result<PathBuf, MissingFile> {
        self.finder.find_raw_data_file(filename, &self.archive_location, &self.page_url)
    }
    fn all_electorates(&self) -> Vec<String> { vec!["State".to_string()] }
    fn read_raw_data(&self, _electorate: &str) -> anyhow::Result<ElectionData> {
        Err(anyhow!("Raw data not available"))
    }

    fn read_raw_data_best_quality(&self, _electorate: &str) -> anyhow::Result<ElectionData> {
        Err(anyhow!("Raw data not available"))
        // read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<WALegislativeCouncil,Self>(self, electorate)
    }

    /// Get the metadata from the file like https://www.ecsa.sa.gov.au/component/edocman/first-preference-votes-for-the-state-pdf,-204kb/download?Itemid=0
    fn read_raw_metadata(&self,_electorate:&str) -> anyhow::Result<ElectionMetadata> {
        /*let url_candidates_and_parties = format!("https://www.ecsa.sa.gov.au/component/edocman/first-preference-votes-for-the-state-pdf,-204kb/download?Itemid=0"); // TODO only works 2022.
        let path_candidates_and_parties = self.find_raw_data_file_from_cache(&url_candidates_and_parties)?;
        parse_first_preferences_pdf(&path_candidates_and_parties,"2022".to_string(),&url_candidates_and_parties)*/
        let mut builder = CandidateAndPartyBuilder::default();
        match self.year.as_str() {
            "2022" => {
                let url_grouped = "https://www.ecsa.sa.gov.au/component/edocman/63-2022se-lc-grouped-candidates-1/download?Itemid=0";
                let url_grouped_path = self.find_raw_data_file_from_cache(url_grouped)?;
                builder.add_source_different_filename(url_grouped,"2022SE - LC Grouped candidates.csv");
                parse_list_of_candidates(&mut builder,&url_grouped_path,false)?;
                let url_ungrouped = "https://www.ecsa.sa.gov.au/component/edocman/63-2022se-lc-ungrouped-candidates-1/download?Itemid=0";
                let url_ungrouped_path = self.find_raw_data_file_from_cache(url_ungrouped)?;
                builder.add_source_different_filename(url_ungrouped,"2022SE - LC ungrouped candidates.csv");
                parse_list_of_candidates(&mut builder,&url_ungrouped_path,true)?;
                let url_elected = "https://www.ecsa.sa.gov.au/component/edocman/51-2022se-lc-order-of-election-of-members/download?Itemid=0";
                let url_elected_path = self.find_raw_data_file_from_cache(url_elected)?;
                builder.add_source_different_filename(url_elected,"2022SE LC Order of election of members.csv");
                parse_list_of_elected_candidates(&mut builder,&url_elected_path)?;
            }
            _ => return Err(anyhow!("Can't read_raw_metadata for SA {}",self.year)),
        }
        Ok(ElectionMetadata{
            name: ElectionName {
                year: self.year.clone(),
                authority: "Electoral Commission South Australia".to_string(),
                name: "Legislative Council".to_string(),
                electorate: "Whole state".to_string(),
                modifications: vec![],
                comment: None,
            },
            candidates: builder.candidates,
            parties: builder.parties,
            source: builder.source,
            results: builder.results,
            vacancies: Some(self.candidates_to_be_elected(_electorate)),
            enrolment: None,
            secondary_vacancies: None,
            excluded: vec![],
            tie_resolutions: Default::default(),
        })
    }

    fn copyright(&self) -> Copyright {
        Copyright {
            statement: Some("© Government of South Australia 2021".into()),
            url: Some("https://www.ecsa.sa.gov.au/copyright".into()),
            license_name: Some("Creative Commons Australia Attribution 3.0 Licence".into()),
            license_url: None,
        }
    }

    fn rules(&self, _electorate: &str) -> AssociatedRules {
        todo!()
    }


    fn read_official_dop_transcript(&self, metadata: &ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        let url = match metadata.name.year.as_str() {
            "2022" => "https://www.ecsa.sa.gov.au/component/edocman/65-2022se-lc-distrbpref-zip/download?Itemid=0", // called "2022SE LC DistrbPref.zip"
                // was a PDF file at "https://www.ecsa.sa.gov.au/component/edocman/lc-full-count-counts-1-to-1500-pdf,-157mb/download?Itemid=0"
            year => return Err(anyhow!("Year {} not supported",year)),
        };
        let url_path = self.find_raw_data_file_from_cache(url)?;
        parse_dop(&url_path,metadata)
    }
}


/// Parse the CSV file at https://www.ecsa.sa.gov.au/component/edocman/63-2022se-lc-grouped-candidates-1/download?Itemid=0
/// or https://www.ecsa.sa.gov.au/component/edocman/63-2022se-lc-ungrouped-candidates-1/download?Itemid=0
/// listing candidates and parties
fn parse_list_of_candidates(builder: &mut CandidateAndPartyBuilder, path: &PathBuf,is_ungrouped:bool) -> anyhow::Result<()> {
    let decoded = file_to_string_windows_1252(&mut File::open(path)?)?; // the file has a non-utf8 character - (non breaking space) in the header just before the close quotation marks
    let decoded = match decoded.find("\"Electoral Commission SA") { // there is some bug in the webserver that produces a whole lot of warnings at the start of the file when downloaded by some methods
        None => decoded,
        Some(pos) => decoded[pos..].to_string(),
    };
    //println!("Decoded : {}",decoded);
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(decoded.as_bytes());
    for result in reader.records() {
        let line = result?;
        if line.len()>=5 && line[3].trim()!="Last name" { // skip headings
            let party_name = if is_ungrouped {"Ungrouped"} else {&line[0]};
            let column_id = if is_ungrouped {"UG"} else {&line[1]};
            if !builder.last_party_is_called(party_name) { builder.add_party(column_id,party_name,None,!is_ungrouped); }
            let last_name = &line[3];
            let first_name = &line[4];
            // should possibly look at other names line, but AFAIK it is not used in any other file.
            let name = format!("{} {}",first_name,last_name);
            //println!("party {} name {}",party_name,name);
            let ec_id = format!("{} {},{}",column_id,last_name,&first_name[0..1]); // used in Distribution of Preferences.
            builder.add_candidate_to_last_party(&name,Some(&ec_id),None)?;
        }
    }
    Ok(())
}

/// Parse the CSV file at https://www.ecsa.sa.gov.au/component/edocman/51-2022se-lc-order-of-election-of-members/download?Itemid=0
/// listing the candidates
fn parse_list_of_elected_candidates(builder: &mut CandidateAndPartyBuilder, path: &PathBuf) -> anyhow::Result<()> {
    let decoded = file_to_string_windows_1252(&mut File::open(path)?)?; // I don't think this is necessary, as it is ASCII, but given the other ones had non-utf8 is is probably more future proof to have this
    let decoded = match decoded.find("\"Electoral Commission SA") { // there is some bug in the webserver that produces a whole lot of warnings at the start of the file when downloaded by some methods
        None => decoded,
        Some(pos) => decoded[pos..].to_string(),
    };
    // println!("Decoded : {}",decoded);
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(decoded.as_bytes());
    let mut found_headline = false;
    for result in reader.records() {
        let line = result?;
        if line.len()>=3 {
            let candidate_name = line[1].trim_start_matches("*");
            if found_headline && candidate_name.len()>0 {
                builder.declare_elected(candidate_name)?;
            } else if candidate_name=="Candidate" { found_headline=true; }
        }
    }
    Ok(())
}

/// Parse the Distribution of Preferences, which is in a CVS file inside a zip file.
fn parse_dop(path: &PathBuf, metadata: &ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
    let mut zipfile = zip::ZipArchive::new(File::open(path)?)?;
    {
        for i in 0..zipfile.len() {
            let file = zipfile.by_index(i)?;
            if file.name().ends_with(".csv") {
                return parse_dop_csv(file,metadata);
            }
        }
        Err(anyhow!("Could not find .csv file in zipfile"))
    }
}

/// Parse the Distribution of Preferences csv file, which is inside a zip file.
/// Unfortunately it does not have ballots or transfer values of lots of things.
fn parse_dop_csv(file:ZipFile,_metadata:&ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(file);
    for result in reader.records() {
        let _line = result?;
        //println!("{}",&line[0]);
    }
    todo!();/*
    Ok(OfficialDistributionOfPreferencesTranscript{
        quota: None,
        counts: vec![],
        missing_negatives_in_papers_delta: false,
        elected_candidates_are_in_order: false,
        all_exhausted_go_to_rounding: false,
        negative_values_in_surplus_distributions_and_rounding_may_be_off: false,
    })*/
}

/* You probably want to use the CSV file instead of the PDF file which was the only thing available when I started doing this.
struct ParseFirstPreferencesPDF {
    builder : CandidateAndPartyBuilder,
    last_number : Option<BallotPaperCount>,
    state : ParseFirstPreferencesPDFState,
    number_informal : BallotPaperCount,
    number_atl : Vec<BallotPaperCount>, // length = parties with ATL allowed.
    number_btl : Vec<BallotPaperCount>, // length =candidates
}

enum ParseFirstPreferencesPDFState {
    WaitingForParty,
    InParty,
    PostParties,
}

impl PDFInterpreter for ParseFirstPreferencesPDF {
    fn new_page(&mut self) {
       // println!("New Page");
    }

    fn text(&mut self, status: &TextStatus, text: Vec<String>) {
        if (status.size-12.0).abs()<0.1 {
            match self.state {
                ParseFirstPreferencesPDFState::WaitingForParty if text.len()==2 &&!text[1].starts_with("Page ") => { // expecting something like ["A", "LIBERAL DEMOCRATS LESS GOVERNMENT MORE FREEDOM"]
                    self.builder.add_party(&text[0],&text[1],None,false); // atl_allowed will be set to true if a GROUP VOTES is encountered
                    self.state = ParseFirstPreferencesPDFState::InParty;
                    return;
                }
                ParseFirstPreferencesPDFState::InParty => {
                    if text.len()==2 && text[0]==text[1] { // a number like ["32497", "32497"]
                        if let Ok(number) = text[0].parse::<usize>() {
                            self.last_number=Some(BallotPaperCount(number));
                            return;
                        }
                    } else if text.len()==1 && self.last_number.is_some() {
                        if text[0].as_str()=="GROUP VOTES" {
                            self.number_atl.push(self.last_number.take().unwrap());
                            self.builder.last_party_mut().unwrap().atl_allowed=true;
                        } else { // candidate name
                            self.number_btl.push(self.last_number.take().unwrap());
                            self.builder.add_candidate_to_last_party(&text[0],None,None).unwrap();
                        }
                        return;
                    }
                }
                ParseFirstPreferencesPDFState::PostParties if text.len()==2 && text[0].as_str()=="Total Informal" => {
                    if let Ok(total_informal)= text[1].replace(',',"").parse::<usize>() {
                        self.number_informal = BallotPaperCount(total_informal);
                    }
                }
                _ => {}
            }
        } else if (status.size-13.333).abs()<0.1 && text.len()==1 {
            if text[0].as_str()=="Group Total" { self.state=ParseFirstPreferencesPDFState::WaitingForParty; }
            else if text[0].as_str()=="Ungrouped Total" { self.state=ParseFirstPreferencesPDFState::PostParties; }
        }
        // println!("{:?} : {:?}",status,text);
    }
}


fn parse_first_preferences_pdf(path:&PathBuf,year:String,url:&str) -> anyhow::Result<ElectionMetadata> {
    let mut parser = ParseFirstPreferencesPDF{
        builder: Default::default(),
        last_number: None,
        state: ParseFirstPreferencesPDFState::WaitingForParty,
        number_informal: BallotPaperCount(0),
        number_atl: vec![],
        number_btl: vec![],
    };
    parser.parse_pdf(path)?;
    Ok(ElectionMetadata{
        name: ElectionName {
            year,
            authority: "Electoral Commission South Australia".to_string(),
            name: "Legislative Council".to_string(),
            electorate: "whole state".to_string(),
            modifications: vec![],
            comment: None,
        },
        candidates: parser.builder.candidates,
        parties: parser.builder.parties,
        source: vec![
            DataSource::new(url,path)
        ],
        results: None,
        vacancies: None,
        enrolment: None,
        secondary_vacancies: None,
        excluded: vec![],
        tie_resolutions: Default::default(),
    })
}
*/
impl SADataLoader {
    fn new(finder:&FileFinder,year:&str) -> anyhow::Result<Self> {
        let archive_location = format!("SA/State{}",year);
        let cache = CacheDir::new(finder.path.join(&archive_location));
        Ok(SADataLoader {
            finder : finder.clone(),
            archive_location,
            year: year.to_string(),
            page_url: format!("https://www.ecsa.sa.gov.au/elections/past-state-election-results?view=article&id=521:results&catid=12:elections"), // TODO only works for 2022. And doesn't even work for that.
            cache,
        })
    }

    // Find the path to an existing file, or useful error if it doesn't exist.
    /// Don't try to download.
    fn find_raw_data_file_from_cache(&self,url:&str) -> anyhow::Result<PathBuf> {
        Ok(self.cache.find_raw_data_file_from_cache(url)?)
    }

}

