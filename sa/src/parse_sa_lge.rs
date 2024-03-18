// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.



use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use anyhow::anyhow;
use serde::{Deserialize};
use stv::ballot_metadata::{Candidate, CandidateIndex, DataSource, ElectionMetadata, ElectionName, NumberOfCandidates};
use stv::datasource_description::{AssociatedRules, Copyright, ElectionDataSource};
use stv::election_data::ElectionData;
use stv::official_dop_transcript::{OfficialDistributionOfPreferencesTranscript};
use stv::parse_util::{FileFinder, KnowsAboutRawMarkings, MissingFile, RawDataSource};
use stv::tie_resolution::TieResolutionsMadeByEC;
use stv::ballot_pile::BallotPaperCount;
use stv::download::CacheDir;

pub fn get_sa_lge_data_loader_2022(finder:&FileFinder) -> anyhow::Result<SALGEDataLoader> {
    SALGEDataLoader::new(finder, "2022-03-23") }


pub struct SALGEDataSource {
    events : Vec<SALGEEvent>,
}

impl ElectionDataSource for SALGEDataSource {
    fn name(&self) -> Cow<'static, str> { "SA Local Government".into() }
    fn ec_name(&self) -> Cow<'static, str> { "Electoral Commission South Australia".into() }
    fn ec_url(&self) -> Cow<'static, str> { "https://www.ecsa.sa.gov.au/".into() }
    fn years(&self) -> Vec<String> { self.events.iter().map(|e|e.election_date.to_string()).collect() }
    fn get_loader_for_year(&self,year: &str,finder:&FileFinder) -> anyhow::Result<Box<dyn RawDataSource+Send+Sync>> {
        Ok(Box::new(SALGEDataLoader::new(finder, year)?))
    }
}

impl SALGEDataSource {
    /// https://apim-ecsa-production.azure-api.net/results-display/LGEEvents gives a list of elections.
    pub fn new(finder:&FileFinder) -> anyhow::Result<Self> {
        let cache = CacheDir::new(finder.path.join("SA/LGE"));
        let path = cache.find_raw_data_file_from_cache("https://apim-ecsa-production.azure-api.net/results-display/LGEEvents")?;
        Ok(SALGEDataSource{ events: serde_json::from_reader(File::open(&path)?)? })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // some fields may not be used until later.
struct SALGEEvent {
    election_date : String, // like 2023-05-25
    election_name : String,
}

pub struct SALGEDataLoader {
    finder : FileFinder,
    archive_location : String,
    year : String,
    page_url : String,
    cache : CacheDir,
    contests : Vec<ContestInfo>,
    contest_index_of_name : HashMap<String,usize>, // map from name to index of contests above
    source : Vec<DataSource>,
}



impl KnowsAboutRawMarkings for SALGEDataLoader {}

impl RawDataSource for SALGEDataLoader {
    fn name(&self, electorate: &str) -> ElectionName {
        ElectionName {
            year: self.year.clone(),
            authority: "Electoral Commission South Australia".to_string(),
            name: "SA Local Government".to_string(),
            electorate: electorate.to_string(),
            modifications: vec![],
            comment: None,
        }
    }

    fn candidates_to_be_elected(&self, region: &str) -> NumberOfCandidates {
        if let Some(&contest_index) = self.contest_index_of_name.get(region) { self.contests[contest_index].pre_election_info.vacancies } else {NumberOfCandidates(0)}
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
    fn all_electorates(&self) -> Vec<String> { self.contests.iter().map(|c|c.contest_name.clone()).collect() }
    fn read_raw_data(&self, _electorate: &str) -> anyhow::Result<ElectionData> {
        Err(anyhow!("Raw data not available"))
    }

    fn read_raw_data_best_quality(&self, _electorate: &str) -> anyhow::Result<ElectionData> {
        Err(anyhow!("Raw data not available"))
        // read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<WALegislativeCouncil,Self>(self, electorate)
    }

    /// Get the metadata, mostly from info it has already
    fn read_raw_metadata(&self,electorate:&str) -> anyhow::Result<ElectionMetadata> {
        if let Some(&contest_index) = self.contest_index_of_name.get(electorate) {
            let contest = &self.contests[contest_index];
            let name = self.name(electorate);
            let candidates : Vec<Candidate> = contest.pre_election_info.candidates.iter().map(|c|Candidate{
                name: c.ballot_name.clone(),
                party: None,
                position: None,
                ec_id: Some(c.uid.clone()),
            }).collect();
            let results = None; // todo parse
            Ok(ElectionMetadata{
                name,
                candidates,
                parties: vec![],
                source: self.source.clone(),
                results,
                vacancies: Some(contest.pre_election_info.vacancies),
                enrolment: Some(NumberOfCandidates(contest.post_election_info.total_enrollment.0)),
                secondary_vacancies: None,
                excluded: vec![],
                tie_resolutions: Default::default(),
            })
        } else { Err(anyhow!("Invalid electorate {}",electorate))}
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
        if let Some(&contest_index) = self.contest_index_of_name.get(&metadata.name.electorate) {
            let contest = &self.contests[contest_index];
            let url = &contest.post_election_info.scrutiny_sheet_url;
            println!("URL : {}",url);
            let url_path = self.find_raw_data_file_from_cache(&url)?;
            parse_dop(&url_path,metadata)
        } else { Err(anyhow!("Invalid electorate {}",&metadata.name.electorate))}
    }
}


/// Parse the Distribution of Preferences csv file, which is inside a zip file.
/// Unfortunately it does not have ballots or transfer values of lots of things.
fn parse_dop(_path:&PathBuf,_metadata:&ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
    // TODO implement
    Ok(OfficialDistributionOfPreferencesTranscript{
        quota: None,
        counts: vec![],
        missing_negatives_in_papers_delta: false,
        elected_candidates_are_in_order: false,
        all_exhausted_go_to_rounding: false,
        negative_values_in_surplus_distributions_and_rounding_may_be_off: false,
    })
}
/*
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

fn merge_whitespace_to_space(s:&str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

 */


impl SALGEDataLoader {
    fn new(finder:&FileFinder,year:&str) -> anyhow::Result<Self> {
        let archive_location = format!("SA/LGE{}",year);
        let cache = CacheDir::new(finder.path.join(&archive_location));
        let council_list_url = format!("https://apim-ecsa-production.azure-api.net/results-display/LGEStatic/{}/0",year);
        let council_list_path = cache.find_raw_data_file_from_cache(&council_list_url)?;
        let council_list : CouncilList = serde_json::from_reader(File::open(&council_list_path)?)?;
        let council_res_list_url = format!("https://apim-ecsa-production.azure-api.net/results-display/LGEChange/{}/0",year);
        let council_res_list_path = cache.find_raw_data_file_from_cache(&council_res_list_url)?;
        let council_res_list : CouncilResList = serde_json::from_reader(File::open(&council_res_list_path)?)?;
        let mut contests : Vec<ContestInfo> = vec![];
        let mut contest_index_of_name : HashMap<String,usize> = HashMap::default();
        // generate regions
        for (cl,rl) in council_list.councils.into_iter().zip(council_res_list.councils.into_iter()) {
            assert_eq!(cl.uid,rl.uid);
            for (pre_election_info,post_election_info) in cl.elections.into_iter().zip(rl.elections.into_iter()) {
                if let Some(post_election_info) = post_election_info { // avoid non-contests
                    assert_eq!(pre_election_info.uid,post_election_info.uid);
                    let contest_name = format!("{} - {}",cl.council_name,pre_election_info.election_name);
                    contest_index_of_name.insert(contest_name.clone(),contests.len());
                    contests.push(ContestInfo{
                        contest_name,
                        council_name: cl.council_name.clone(),
                        council_full_name: cl.council_full_name.to_string(),
                        pre_election_info,
                        post_election_info,
                    });
                }
            }
        }

        Ok(SALGEDataLoader {
            finder : finder.clone(),
            archive_location,
            year: year.to_string(),
            page_url: format!("https://result.ecsa.sa.gov.au/lgeresults"),
            cache,
            contests,
            contest_index_of_name,
            source: vec![DataSource::new(&council_list_url,&council_list_path),DataSource::new(&council_res_list_url,&council_res_list_path)],
        })
    }

    // Find the path to an existing file, or useful error if it doesn't exist.
    /// Don't try to download.
    fn find_raw_data_file_from_cache(&self,url:&str) -> anyhow::Result<PathBuf> {
        Ok(self.cache.find_raw_data_file_from_cache(url)?)
    }

}

#[allow(dead_code)] // some fields may not be used until later.
struct ContestInfo {
    contest_name : String,
    council_name : String,
    council_full_name : String,
    pre_election_info : CouncilElection,
    post_election_info : CouncilResElection,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
/// something that can parse https://apim-ecsa-production.azure-api.net/results-display/LGEStatic/2023-05-25/0 [note that element 17 is the non-trivial one ]
struct CouncilList {
    councils : Vec<Council>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Council {
    uid : String,
    council_name : String,
    council_full_name : String,
    elections : Vec<CouncilElection>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CouncilElection {
    uid : String,
    election_name : String,
    vacancies : NumberOfCandidates,
    candidates : Vec<CouncilElectionCandidate>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // some fields may not be used until later.
struct CouncilElectionCandidate {
    uid : String,
    candidate_order: usize,
    ballot_name : String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
/// something that can parse https://apim-ecsa-production.azure-api.net/results-display/LGEChange/2023-05-25/0 [note that element 17 is the non-trivial one ]
struct CouncilResList {
    councils : Vec<CouncilRes>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CouncilRes {
    uid : String,
    elections : Vec<Option<CouncilResElection>>, // sometimes is null
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // some fields may not be used until later.
struct CouncilResElection {
    uid : String,
    informal_ballot_count : BallotPaperCount,
    total_enrollment : BallotPaperCount,
    scrutiny_sheet_url : String,
    quota : usize,
    candidates : Vec<CouncilResElectionCandidate>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // some fields may not be used until later.
struct CouncilResElectionCandidate {
    uid : String,
    first_pref_votes : BallotPaperCount,
    is_elected : bool,
    order_of_election : usize, // seems to be present and 0 if not used,
    count_no : usize,
    votes_at_conclusion : BallotPaperCount,
}