use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{BufReader, BufRead, Seek, SeekFrom};
use stv::ballot_metadata::{ElectionName, Candidate, CandidateIndex, PartyIndex, Party, ElectionMetadata, DataSource};
use stv::ballot_paper::{RawBallotMarking, parse_marking, RawBallotMarkings, FormalVote, ATL, BTL};
use std::collections::HashMap;
use csv::{StringRecord, StringRecordsIntoIter};
use zip::ZipArchive;
use zip::read::ZipFile;
use anyhow::anyhow;
use stv::election_data::ElectionData;

pub fn get_federal_data_loader_2016() -> FederalDataLoader {
    FederalDataLoader::new("2016",true,"https://results.aec.gov.au/20499/Website/SenateDownloadsMenu-20499-Csv.htm",20499)
}

pub fn get_federal_data_loader_2019() -> FederalDataLoader {
    FederalDataLoader::new("2019",false,"https://results.aec.gov.au/24310/Website/SenateDownloadsMenu-24310-Csv.htm",24310)
}


pub struct FederalDataLoader {
    year : String,
    double_dissolution : bool,
    page_url : String,
    election_number : usize,
    base_path : PathBuf,
}

impl FederalDataLoader {
    pub fn new(year:&'static str,double_dissolution:bool,page_url:&'static str,election_number:usize) -> Self {
        let base_path : PathBuf = PathBuf::from("../votecounting/CountPreferentialVotes/Elections/Federal/".to_string()+year);
        FederalDataLoader {
            year: year.to_string(),
            double_dissolution,
            page_url: page_url.to_string(),
            election_number,
            base_path
        }
    }
    pub fn name(&self,state:&str) -> ElectionName {
        ElectionName{
            year: self.year.clone(),
            authority: "AEC".to_string(),
            name: "Federal Senate".to_string(),
            electorate: state.to_string(),
            modifications: vec![]
        }
    }

    fn name_of_candidate_source_post_election(&self) -> String {
        format!("SenateFirstPrefsByStateByVoteTypeDownload-{}.csv",self.election_number)
    }
    fn name_of_vote_source(&self,state:&str) -> String {
        format!("aec-senate-formalpreferences-{}-{}.zip",self.election_number,state)
    }
    pub fn read_raw_metadata(&self,state:&str) -> anyhow::Result<ElectionMetadata> {
        let mut builder = CandidateAndGroupInformationBuilder::default();
        builder.read_from_senate_first_prefs_by_state_by_vote_typ_download_file(self.base_path.join(self.name_of_candidate_source_post_election()).as_path(),state)?;
        Ok(ElectionMetadata{
            name: self.name(state),
            candidates: builder.candidates.clone(),
            parties: builder.extract_parties(),
            source: vec![DataSource{
                url: self.page_url.clone(),
                files: vec![self.name_of_candidate_source_post_election()],
                comments: None
            }],
            results: None
        })
    }

    pub fn load_cached_data(&self,state:&str) -> anyhow::Result<ElectionData> {
        match self.name(state).load_cached_data() {
            Ok(data) => Ok(data),
            Err(_) => {
                let data = self.read_raw_data(state)?;
                data.save_to_cache()?;
                Ok(data)
            }
        }
    }

    // This below should be made more general and most of it factored out into a separate function.
    pub fn read_raw_data(&self,state:&str) -> anyhow::Result<ElectionData> {
        let mut metadata = self.read_raw_metadata(state)?;
        let filename = self.name_of_vote_source(state);
        let preferences_zip_file = self.base_path.join(&filename);
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
}

#[derive(Default)]
struct CandidateAndGroupInformationBuilder {
    candidates : Vec<Candidate>,
    //candidate_by_id : HashMap<String,CandidateIndex>,
    parties : Vec<GroupBuilder>,
}

struct GroupBuilder {
    name : String,
    group_id : String, // e.g. "A" or "UG"
    ticket_id : Option<String>, // the dummy candidate id for the ticket vote.
}

fn skip_first_line_of_file(path:&Path) -> anyhow::Result<File> {
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
    // the candidate information file doesn't list the place on the ticket.
    // the SenateFirstPrefsByStateByVoteTypeDownload file does, but it isn't available until after the election.
    // the file that is available before the election is not available well after the election :-)
    // so need to be able to parse both.
    fn read_from_senate_first_prefs_by_state_by_vote_typ_download_file(&mut self,path:&Path,state:&str) -> anyhow::Result<()> {
        let mut rdr = csv::Reader::from_reader(skip_first_line_of_file(path)?);
        for result in rdr.records() {
            let record = result?;
            if state==&record[0] { // right state
                let group_id = &record[1]; // something like A, B, or UG
                let candidate_id = &record[2]; // something like 32847
                if candidate_id!="0" {
                    let position_in_ticket = record[3].parse::<usize>()?; // 0, 1, .. 0 means a dummy id for the group ticket.
                    if self.parties.len()==0 || &self.parties[self.parties.len()-1].group_id != group_id {
                        self.parties.push(GroupBuilder{name:record[5].to_string(),group_id:group_id.to_string(),ticket_id:if position_in_ticket==0 {Some(candidate_id.to_string())} else {None}});
                    }
                    if position_in_ticket!=0 { // real candidate.
                        // self.candidate_by_id.insert(candidate_id.to_string(),CandidateIndex(self.candidates.len()));
                        self.candidates.push(Candidate{
                            name: record[4].to_string(),
                            party: PartyIndex(self.parties.len()-1),
                            position: position_in_ticket
                        })
                    }
                }
            }
        }
        Ok(())
    }

    fn extract_parties(&self) -> Vec<Party> {
        let mut res : Vec<Party> = self.parties.iter().map(|g|Party{
            column_id: g.group_id.clone(),
            name: g.name.clone(),
            abbreviation: None,
            atl_allowed: g.ticket_id.is_some(),
            candidates: vec![]
        }).collect();
        for candidate_index in 0..self.candidates.len() {
            let candidate = & self.candidates[candidate_index];
            res[candidate.party.0].candidates.push(CandidateIndex(candidate_index));
            assert_eq!(res[candidate.party.0].candidates.len(),candidate.position);
        }
        res
    }
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
