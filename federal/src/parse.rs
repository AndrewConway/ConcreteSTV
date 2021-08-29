// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use std::path::{Path, PathBuf};
use std::fs::File;
use stv::ballot_metadata::{ElectionName, Candidate, CandidateIndex, PartyIndex, ElectionMetadata, DataSource, NumberOfCandidates};
use stv::ballot_paper::{RawBallotMarking, parse_marking, RawBallotMarkings, FormalVote, ATL, BTL};
use std::collections::{HashMap};
use csv::{StringRecord, StringRecordsIntoIter};
use zip::ZipArchive;
use zip::read::ZipFile;
use anyhow::anyhow;
use stv::election_data::ElectionData;
use stv::distribution_of_preferences_transcript::QuotaInfo;
use serde::Deserialize;
use stv::ballot_pile::BallotPaperCount;
use stv::official_dop_transcript::{candidate_elem, OfficialDistributionOfPreferencesTranscript};
use stv::tie_resolution::TieResolutionsMadeByEC;
use stv::parse_util::{CandidateAndGroupInformationBuilder, skip_first_line_of_file, GroupBuilder, RawDataSource, MissingFile, FileFinder};
use crate::parse2013::{read_from_senate_group_voting_tickets_download_file2013, read_ticket_votes2013, read_btl_votes2013};

pub fn get_federal_data_loader_2013(finder:&FileFinder) -> FederalDataLoader {
    FederalDataLoader::new(finder,"2013",false,"https://results.aec.gov.au/17496/Website/SenateDownloadsMenu-17496-Csv.htm",17496)
}

pub fn get_federal_data_loader_2016(finder:&FileFinder) -> FederalDataLoader {
    FederalDataLoader::new(finder,"2016",true,"https://results.aec.gov.au/20499/Website/SenateDownloadsMenu-20499-Csv.htm",20499)
}

pub fn get_federal_data_loader_2019(finder:&FileFinder) -> FederalDataLoader {
    FederalDataLoader::new(finder,"2019",false,"https://results.aec.gov.au/24310/Website/SenateDownloadsMenu-24310-Csv.htm",24310)
}


pub struct FederalDataLoader {
    finder : FileFinder,
    archive_location : String,
    year : String,
    double_dissolution : bool,
    page_url : String,
    election_number : usize,
}

impl RawDataSource for FederalDataLoader {
    fn name(&self,state:&str) -> ElectionName {
        ElectionName{
            year: self.year.clone(),
            authority: "AEC".to_string(),
            name: "Federal Senate".to_string(),
            electorate: state.to_string(),
            modifications: vec![],
            comment: None,
        }
    }

    fn candidates_to_be_elected(&self,state:&str) -> NumberOfCandidates {
        NumberOfCandidates(
            if state=="ACT" || state=="NT" { 2 }
            else if self.double_dissolution { 12 }
            else { 6 }
        )
    }

    /// These are deduced by looking at the actual transcript of results.
    /// I have not included anything if all decisions are handled by the fallback "earlier on the ballot paper candidates are listed in worse positions.
    fn ec_decisions(&self,state:&str) -> TieResolutionsMadeByEC {
        match self.year.as_str() {
            "2013" => match state {
                "VIC" => TieResolutionsMadeByEC{ tie_resolutions: vec![vec![CandidateIndex(54), CandidateIndex(23),CandidateIndex(85),CandidateIndex(88)]] } , // 4 way tie at count 10.
                "NSW" => TieResolutionsMadeByEC{ tie_resolutions: vec![vec![CandidateIndex(82),CandidateIndex(52),CandidateIndex(54)], vec![CandidateIndex(104),CandidateIndex(68),CandidateIndex(72)], vec![CandidateIndex(56),CandidateIndex(7)], vec![CandidateIndex(20),CandidateIndex(12),CandidateIndex(96)]] } ,
                _ => Default::default(),
            },
            _ => Default::default(),
        }
    }

    /// These are due to a variety of events.
    fn excluded_candidates(&self,state:&str) -> Vec<CandidateIndex> {
        match self.year.as_str() {
            "2016" => match state {
                "SA" => vec![CandidateIndex(38)], // Bob Day was excluded because of indirect pecuniary interest.
                "WA" => vec![CandidateIndex(45)], // Rod Cullerton was excluded because of bankruptcy and larceny.
                _ => Default::default(),
            },
            _ => Default::default(),
        }
    }

    // This below should be made more general and most of it factored out into a separate function.
    fn read_raw_data(&self,state:&str) -> anyhow::Result<ElectionData> {
        if self.year=="2013" { return self.read_raw_data2013(state); }
        let mut metadata = self.read_raw_metadata(state)?;
        let filename = self.name_of_vote_source(state);
        let preferences_zip_file = self.find_raw_data_file(&filename)?;
        println!("Parsing {}",&preferences_zip_file.to_string_lossy());
        metadata.source[0].files.push(filename);
        let mut parties_that_can_get_atls = vec![];
        for i in 0..metadata.parties.len() {
            if metadata.parties[i].atl_allowed { parties_that_can_get_atls.push(PartyIndex(i)); }
        }
        let mut zipfile = zip::ZipArchive::new(File::open(preferences_zip_file)?)?;
        let mut atls : HashMap<Vec<PartyIndex>,usize> = HashMap::default();
        let mut btls : HashMap<Vec<CandidateIndex>,usize> = HashMap::default();
        let mut informal = 0;
        for record in ParsedRawVoteIterator::new(&mut zipfile)? {
            let record=record?;
            let markings = RawBallotMarkings::new(&parties_that_can_get_atls,&record.markings);
            //println!("Markings {:#?}",record.markings);
            //println!("Interpretatation {:#?}",markings.interpret_vote(1,6));
            match markings.interpret_vote(1,6) {
                None => { informal+=1 }
                Some(FormalVote::Btl(btl)) => { *btls.entry(btl.candidates).or_insert(0)+=btl.n }
                Some(FormalVote::Atl(atl)) => { *atls.entry(atl.parties).or_insert(0)+=atl.n }
            }
        }
        let atl = atls.into_iter().map(|(parties,n)|ATL{ parties, n }).collect();
        let btl = btls.into_iter().map(|(candidates,n)|BTL{ candidates , n }).collect();
        Ok(ElectionData{ metadata, atl, btl, informal })
    }

    fn find_raw_data_file(&self,filename:&str) -> Result<PathBuf,MissingFile> {
        self.finder.find_raw_data_file(filename,&self.archive_location,&self.page_url)
    }
    fn all_electorates(&self) -> Vec<String> {
        vec!["ACT".to_string(),"NT".to_string(),"TAS".to_string(),"VIC".to_string(),"NSW".to_string(),"QLD".to_string(),"SA".to_string(),"WA".to_string()]
    }

}

impl FederalDataLoader {


    pub fn new(finder:&FileFinder,year:&'static str,double_dissolution:bool,page_url:&'static str,election_number:usize) -> Self {
        FederalDataLoader {
            finder : finder.clone(),
            archive_location: "Federal/".to_string()+year,
            year: year.to_string(),
            double_dissolution,
            page_url: page_url.to_string(),
            election_number,
        }
    }

    fn name_of_candidate_source_post_election(&self) -> String {
        if self.year=="2013" { "SenateGroupVotingTicketsDownload-17496.csv".to_string() }
        else {
            format!("SenateFirstPrefsByStateByVoteTypeDownload-{}.csv",self.election_number)
        }
    }
    fn name_of_vote_source(&self,state:&str) -> String {
        format!("aec-senate-formalpreferences-{}-{}.zip",self.election_number,state)
    }
    fn name_of_official_transcript_zip_file(&self) -> String {
        format!("SenateDopDownload-{}.zip",self.election_number)
    }
    pub fn read_raw_metadata(&self,state:&str) -> anyhow::Result<ElectionMetadata> {
        let mut builder = CandidateAndGroupInformationBuilder::default();
        if self.year=="2013" { read_from_senate_group_voting_tickets_download_file2013(&mut builder,self.find_raw_data_file(&self.name_of_candidate_source_post_election())?.as_path(),state)?; }
        else { read_from_senate_first_prefs_by_state_by_vote_typ_download_file2016(&mut builder,self.find_raw_data_file(&self.name_of_candidate_source_post_election())?.as_path(),state)?; }
        let vacancies = self.candidates_to_be_elected(state);
        Ok(ElectionMetadata{
            name: self.name(state),
            candidates: builder.candidates.clone(),
            parties: builder.extract_parties(),
            source: vec![DataSource{
                url: self.page_url.clone(),
                files: vec![self.name_of_candidate_source_post_election()],
                comments: None
            }],
            results: None,
            vacancies: Some(vacancies),
            secondary_vacancies: if vacancies==NumberOfCandidates(12) { Some(NumberOfCandidates(6)) } else {None},
            excluded: self.excluded_candidates(state),
            tie_resolutions : self.ec_decisions(state),
        })
    }


    fn read_raw_data2013(&self,state:&str) -> anyhow::Result<ElectionData> {
        let mut metadata = self.read_raw_metadata(state)?;
        let filename = "SenateUseOfGvtByGroupDownload-17496.csv".to_string();
        let preferences_zip_file = self.find_raw_data_file(&filename)?;
        println!("Parsing {}",&preferences_zip_file.to_string_lossy());
        metadata.source[0].files.push(filename);
        let ticket_votes = read_ticket_votes2013(&metadata,&preferences_zip_file,state)?;
        let filename = format!("SenateStateBtlDownload-{}-{}.zip",self.election_number,state);
        let preferences_zip_file = self.find_raw_data_file(&filename)?;
        println!("Parsing {}",&preferences_zip_file.to_string_lossy());
        metadata.source[0].files.push(filename);
        let (mut btl,informal) = read_btl_votes2013(&metadata, &preferences_zip_file, 1)?; // The 2013 formality rules are quite complex. I am assuming the AEC has applied them already to all with a 1 vote. This is a dubious assumption as there are some without a 1 vote. However since we don't get all the informal votes, it is hard to check formality properly.
        btl.extend_from_slice(&ticket_votes);
        Ok(ElectionData{ metadata, atl:vec![], btl, informal })
    }

    pub fn read_official_dop_transcript(&self,metadata:&ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        let filename = self.name_of_official_transcript_zip_file();
        let preferences_zip_file = self.find_raw_data_file(&filename)?;
        println!("Parsing {}",&preferences_zip_file.to_string_lossy());
        let mut zipfile = zip::ZipArchive::new(File::open(preferences_zip_file)?)?;
        {
            for i in 0..zipfile.len() {
                let file = zipfile.by_index(i)?;
                if file.name().contains(&metadata.name.electorate) {
                    return read_official_dop_transcript_work(file,metadata);
                }
            }
            Err(anyhow!("Could not find file in zipfile for {}",&metadata.name.electorate))
/*
            if let Some(file_name) = zipfile.file_names().find(|&n|n.contains(&data.metadata.name.electorate)).map(|file_name|zipfile.by_name(file_name)) {
                let zip_contents = file_name?; //zipfile.by_name(file_name)?;
            } else {}*/
        }
    }

    pub fn all_states_data<'a>(&'a self) -> impl Iterator<Item=anyhow::Result<ElectionData>> + 'a {
        ["ACT","NT","TAS","VIC","NSW","QLD","SA","WA"].iter().map(move |&state|self.load_cached_data(state))
    }
}


fn read_official_dop_transcript_work(file : ZipFile,metadata : &ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
    let mut reader = csv::ReaderBuilder::new().flexible(false).has_headers(true).from_reader(file);
    #[derive(Debug, Deserialize)]
    struct Record {
        #[serde(rename = "State")] state: String,
        #[serde(rename = "No Of Vacancies")] vacancies: usize,
        #[serde(rename = "Total Formal Papers")] formal_papers: usize,
        #[serde(rename = "Quota")] quota : usize,
        #[serde(rename = "Count")] count : usize,
        #[serde(rename = "Ballot Position")] ballot_position : usize,
        #[serde(rename = "Ticket")] ticket : String,
        #[serde(rename = "Surname")] surname : String,
        #[serde(rename = "GivenNm")] given_name : String,
        #[serde(rename = "Papers")] papers_transferred : isize,
        #[serde(rename = "VoteTransferred")] votes_transferred : isize,
        #[serde(rename = "ProgressiveVoteTotal")] votes_total : usize,
        #[serde(rename = "Transfer Value")] transfer_value : f64,
        #[serde(rename = "Status")] status : String, // blank, Elected, Excluded
        #[serde(rename = "Changed")] changed : String, // True or blank.
        #[serde(rename = "Order Elected")] order_elected : usize,
        #[serde(rename = "Comment")] comment: Option<String>,
    }
    let lookup_names : HashMap<String,CandidateIndex> = metadata.get_candidate_name_lookup();
    let mut res = OfficialDistributionOfPreferencesTranscript::default();
    let mut last_count : usize = 0;
    let mut order_elected : HashMap<CandidateIndex,usize> = Default::default(); // value is order elected, which is not necessarily as encountered.
    let mut excluded_last : Vec<CandidateIndex> = vec![]; // transcript marks them as excluded the round before they are excluded in.
    for result in reader.deserialize() {
        let record : Record = result?;
        if last_count==0 {
            res.quota=Some(QuotaInfo{
                papers: BallotPaperCount(record.formal_papers),
                vacancies : NumberOfCandidates(record.vacancies),
                quota: record.quota as f64
            });
        }
        if record.count!=last_count {
            last_count=record.count;
            res.finished_count();
            res.count().excluded.extend(excluded_last.drain(..));
        }
        if record.transfer_value!=0.0 { res.count().transfer_value = Some(record.transfer_value) }
        if record.surname=="Exhausted" {
            res.count().paper_delta().exhausted= record.papers_transferred as isize;
            res.count().vote_delta().exhausted= record.votes_transferred as f64;
            res.count().vote_total().exhausted= record.votes_total as f64;
        } else if record.surname=="Gain/Loss" {
            res.count().paper_delta().rounding= record.papers_transferred as isize;
            res.count().vote_delta().rounding= record.votes_transferred as f64;
            res.count().vote_total().rounding= record.votes_total as f64;
        } else {
            let name = record.surname+", "+&record.given_name;
            match lookup_names.get(&name) {
                None => return Err(anyhow!("Could not find name {}",name)),
                Some(&candidate) => {
                    * candidate_elem(&mut res.count().paper_delta().candidate,candidate) = record.papers_transferred as isize;
                    * candidate_elem(&mut res.count().vote_delta().candidate,candidate)= record.votes_transferred as f64;
                    * candidate_elem(&mut res.count().vote_total().candidate,candidate)= record.votes_total as f64;
                    if &record.changed=="True" {
                        match record.status.as_str() {
                            "Excluded" => excluded_last.push(candidate),
                            "Elected" => {
                                //println!("Elected {} at count {}",candidate,res.counts.len());
                                res.count().elected.push(candidate);
                                order_elected.insert(candidate,record.order_elected);
                                res.count().elected.sort_by_key(|c|order_elected.get(c));
                            }
                            _ => return Err(anyhow!("Could not understand status {}",record.status)),
                        }
                    }
                }
            }
        }
    }
    Ok(res)
}


/// the candidate information file doesn't list the place on the ticket.
/// the SenateFirstPrefsByStateByVoteTypeDownload file does, but it isn't available until after the election.
/// the file that is available before the election is not available well after the election :-)
/// so need to be able to parse both.
/// This format is used in 2016 and 2019
fn read_from_senate_first_prefs_by_state_by_vote_typ_download_file2016(builder: &mut CandidateAndGroupInformationBuilder,path:&Path,state:&str) -> anyhow::Result<()> {
    let mut rdr = csv::Reader::from_reader(skip_first_line_of_file(path)?);
    for result in rdr.records() {
        let record = result?;
        if state==&record[0] { // right state
            let group_id = &record[1]; // something like A, B, or UG
            let candidate_id = &record[2]; // something like 32847
            if candidate_id!="0" {
                let position_in_ticket = record[3].parse::<usize>()?; // 0, 1, .. 0 means a dummy id for the group ticket.
                if builder.parties.len()==0 || &builder.parties[builder.parties.len()-1].group_id != group_id {
                    builder.parties.push(GroupBuilder{name:record[5].to_string(), abbreviation:None, group_id:group_id.to_string(),ticket_id:if position_in_ticket==0 {Some(candidate_id.to_string())} else {None}, tickets: vec![] });
                }
                if position_in_ticket!=0 { // real candidate.
                    // self.candidate_by_id.insert(candidate_id.to_string(),CandidateIndex(self.candidates.len()));
                    builder.candidates.push(Candidate{
                        name: record[4].to_string(),
                        party: Some(PartyIndex(builder.parties.len()-1)),
                        position: Some(position_in_ticket),
                        ec_id: Some(candidate_id.to_string()),
                    })
                }
            }
        }
    }
    Ok(())
}



struct ParsedRawVoteIterator<'a> {
    electorate_column : usize,
    collection_column : usize,
    preferences_column : Option<usize>,
    // reader : Reader<ZipFile<'a>>,
    records : StringRecordsIntoIter<ZipFile<'a>>
}


impl<'a> ParsedRawVoteIterator<'a> {
    fn new(zipfile : &'a mut ZipArchive<File>) -> anyhow::Result<Self> {
        let zip_contents = zipfile.by_index(0)?;
        let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(zip_contents);
        let headings = reader.headers()?;
        let electorate_column = if &headings[0]=="ElectorateNm" {0} else if &headings[1]=="Division" {1} else { return Err(anyhow!("Could not find a division heading"))};
        let collection_column = if &headings[1]=="VoteCollectionPointNm" {1} else if &headings[2]=="Vote Collection Point Name" {2} else {return Err(anyhow!("Could not find a collection point heading"))};
        let preferences_column = if &headings[5]=="Preferences" {Some(5)} else {None};
        let records = reader.into_records();
        Ok(ParsedRawVoteIterator {
            electorate_column,
            collection_column,
            preferences_column,
            records,
        })
    }
}

pub struct ParsedRawVote {
    pub markings : Vec<RawBallotMarking>,
    electorate_column : usize,
    collection_column : usize,
    record : StringRecord,
}

impl ParsedRawVote {
    pub fn metadata(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("Electorate".to_string(),self.record[self.electorate_column].to_string());
        map.insert("Collection Point".to_string(),self.record[self.collection_column].to_string());
        map
    }
}

impl <'a> Iterator for ParsedRawVoteIterator<'a> {
    type Item = Result<ParsedRawVote,csv::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.records.next() {
            Some(Ok(record)) => {
                if record[0].starts_with("---") { return self.next(); } // skip dummy heading "underlines" if there.
                let mut markings : Vec<RawBallotMarking> = Vec::with_capacity(100); // TODO num_atl+num_btl
                match self.preferences_column {
                    Some(preferences_column) => { // preferences are all in 1 column, comma separated
                        for s in record[preferences_column].split(',') {
                            markings.push(parse_marking(s));
                        }
                    }
                    None => {
                        for i in 6..record.len() {
                            markings.push(parse_marking(&record[i]));
                        }
                    }
                }
                Some(Ok(ParsedRawVote{
                    markings,
                    electorate_column: self.electorate_column,
                    collection_column: self.collection_column,
                    record
                }))
            }
            None => None,
            Some(Err(e)) => Some(Err(e)),
        }
    }
}
