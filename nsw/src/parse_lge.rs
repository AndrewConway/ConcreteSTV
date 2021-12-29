// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Functions to parse NSW election data

use std::collections::{HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};
use stv::ballot_metadata::{Candidate, CandidateIndex, ElectionMetadata, ElectionName, NumberOfCandidates, Party, PartyIndex};
use stv::election_data::ElectionData;
use anyhow::anyhow;
use scraper::{ElementRef, Html, Selector};
use stv::ballot_paper::PreferencesComingOutOfOrderHelper;
use stv::parse_util::{file_to_string, FileFinder, MissingFile, RawDataSource};
use stv::tie_resolution::TieResolutionsMadeByEC;
use serde::{Serialize,Deserialize};
use stv::ballot_pile::BallotPaperCount;
use stv::distribution_of_preferences_transcript::{PerCandidate, QuotaInfo};
use stv::official_dop_transcript::{OfficialDistributionOfPreferencesTranscript, OfficialDOPForOneCount};
use stv::parse_util::parse_xlsx_by_converting_to_csv_using_openoffice;


pub fn get_nsw_lge_data_loader_2021(finder:&FileFinder) -> anyhow::Result<NSWLGEDataLoader> {
    NSWLGEDataLoader::new(finder,"2021","https://vtr.elections.nsw.gov.au/LG2101/results")
}

pub struct NSWLGEDataLoader {
    finder : FileFinder,
    archive_location : String,
    year : String,
    page_url : String,
    /// contests by electorate. Preloaded for speed.
    contests : Vec<NSWLGEContest>,
}

/// Preloaded data on the contests. This can be extracted from multiple files on the web, but for simplicity for the user and speed of starting up this is precomputed and included.
#[derive(Serialize,Deserialize)]
pub struct NSWLGEContest {
    /// human readable name of the contest, e.g. `Ballina A Ward`
    pub name : String,
    /// name of the contest used in a url e.g. `ballina/a-ward`
    pub url : String,
    /// number of councillors to elect.
    pub vacancies : NumberOfCandidates,
}

impl RawDataSource for NSWLGEDataLoader {
    fn name(&self,electorate:&str) -> ElectionName {
        ElectionName {
            year: self.year.clone(),
            authority: "NSW Electoral Commission".to_string(),
            name: "Local Government".to_string(),
            electorate: electorate.to_string(),
            modifications: vec![],
            comment: None,
        }
    }

    fn candidates_to_be_elected(&self,region:&str) -> NumberOfCandidates {
        self.find_contest(region).unwrap().vacancies
    }

    /// These are deduced by looking at the actual transcript of results.
    /// I have not included anything if all decisions are handled by the fallback "earlier on the ballot paper candidates are listed in worse positions.
    fn ec_decisions(&self,_electorate:&str) -> TieResolutionsMadeByEC {
            Default::default()
    }

    /// These are due to a variety of events.
    fn excluded_candidates(&self,_electorate:&str) -> Vec<CandidateIndex> {
        Default::default()
    }

    fn find_raw_data_file(&self,filename:&str) -> Result<PathBuf,MissingFile> {
        self.finder.find_raw_data_file(filename,&self.archive_location,&self.page_url)
    }
    fn all_electorates(&self) -> Vec<String> {
        self.contests.iter().map(|c|c.name.clone()).collect()
    }

    fn read_raw_data(&self, electorate: &str) -> anyhow::Result<ElectionData> {
        let contest = &self.find_contest(electorate)?.url;
        let mayoral = electorate.ends_with(" Mayoral");
        let metadata_data_file = self.find_raw_data_file_relative(contest,
                                                                  if mayoral {"mayoral/report"} else {"councillor/report"},
                                                                  if mayoral {"mayoral-fp-by-candidate.html"} else {"fp-by-grp-and-candidate-by-vote-type.html"},
                                                                  if mayoral {"mayoral/report/fp-by-grp-and-candidate-by-vote-type"} else {"councillor/report/fp-by-grp-and-candidate-by-vote-type"})?;
        let mut metadata = self.parse_candidate_list(File::open(metadata_data_file)?,mayoral,electorate)?;
        let winner_info_file = self.find_raw_data_file_relative(contest,"",
                                                                  if mayoral {"mayoral.html"} else {"councillor.html"},
                                                                  if mayoral {"mayoral"} else {"councillor"})?;
        let winner_info = NSWLGESingleContestMainPageInfo::extract_file(winner_info_file)?;
        if !winner_info.elected_candidates.is_empty() {
            let candidates = metadata.get_candidate_name_lookup_with_capital_letters_afterwards();
            fn remove_excess_whitespace(s:&str) -> String {
                s.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
            }
            let winners: anyhow::Result<Vec<_>> = winner_info.elected_candidates.iter().map(|c|candidates.get(&remove_excess_whitespace(c)).cloned().ok_or_else(||anyhow!("Could not find winning candidate {} in candidate list after converting to {}",c,remove_excess_whitespace(c)))).collect();
            metadata.results=Some(winners?);
        }
        // get excluded candidates
        if !mayoral {
            let ineligible_info_file = self.find_raw_data_file_relative(contest,"councillor/report","grp-and-candidates-result.html","councillor/report/grp-and-candidates-result")?;
            let ineligible = get_ineligible_candidates(&Html::parse_document(&std::fs::read_to_string(ineligible_info_file)?))?;
            metadata.excluded=ineligible;
        }
        let zip_name = if mayoral {"mayoral-finalpreferencedatafile.zip"} else {"finalpreferencedatafile.zip"};
        let zip_preferences_list = self.find_raw_data_file_relative(contest,"download",zip_name,&("download/".to_string()+zip_name))?;
        parse_zip_election_file(File::open(zip_preferences_list)?,metadata,None,mayoral)
    }
}

impl NSWLGEDataLoader {

    pub fn new(finder:&FileFinder,year:&'static str,page_url:&'static str) -> anyhow::Result<Self> {
        let archive_location = "NSW/LGE".to_string()+year+"/vtr.elections.nsw.gov.au/LG2101"; // The 2101 should not be hardcoded.
        Ok(NSWLGEDataLoader {
            finder : finder.clone(),
            archive_location,
            year: year.to_string(),
            page_url: page_url.to_string(),
            contests : serde_json::from_str(include_str!("NSWLGE2021_contest_list.json"))?,
        })
    }

    fn find_contest(&self,electorate:&str) -> anyhow::Result<&NSWLGEContest> {
        self.contests.iter().find(|c|c.name==electorate).ok_or_else(||anyhow!("Could not find electorate {}",electorate))
    }

    pub fn find_raw_data_file_relative(&self,contest:&str,relfolder:&str,filename:&str,url_relative:&str) -> Result<PathBuf,MissingFile> {
        let archive_location = self.archive_location.clone()+"/"+contest+(if relfolder.is_empty() {""} else {"/"})+relfolder;
        // println!("Archive location {}",archive_location);
        self.finder.find_raw_data_file_with_extra_url_info(filename,&archive_location,&self.page_url,&(contest.to_string()+"/"+url_relative))
    }
    /// parse a file like https://vtr.elections.nsw.gov.au/LG2101/albury/councillor/report/fp-by-grp-and-candidate-by-vote-type or https://vtr.elections.nsw.gov.au/LG2101/ballina/a-ward/councillor/report/fp-by-grp-and-candidate-by-vote-type
    /// or, for mayoral, like https://vtr.elections.nsw.gov.au/LG2101/ballina/mayoral/report/mayoral-fp-by-candidate
    fn parse_candidate_list(&self,mut file:File,mayoral:bool,electorate:&str) -> anyhow::Result<ElectionMetadata> {
        let html = scraper::Html::parse_document(&file_to_string(&mut file)?);
        // the title has hypens missing - see Ballina - A Ward
        // let electorate = html.select(&Selector::parse("head > title").unwrap()).flat_map(|e|e.text()).collect::<Vec<_>>().join("")+if mayoral {" Mayoral"} else {""};
        let electorate = electorate.to_string(); // +if mayoral {" Mayoral"} else {""};
        if electorate.is_empty() { return Err(anyhow!("Empty page title")); }
        let table = html.select(&Selector::parse("table").unwrap()).next().ok_or_else(||anyhow!("No table element"))?;
        let thead_first = table.select(&Selector::parse("thead > tr > th").unwrap()).next().ok_or_else(||anyhow!("No table headings"))?;
        let has_groups = thead_first.inner_html()=="Group";
        let selector_td = Selector::parse("td").unwrap();
        let mut candidates = vec![];
        let mut parties = vec![];
        let mut current_position = 0;
        // parse <div><strong>A</strong>B</div> into (A,B) if possible.
        fn parse_note(e:ElementRef) -> Option<(String,String)> {
            let text = e.text().map(|s|s.trim()).filter(|s|!s.is_empty()).collect::<Vec<_>>();
            if text.len()==2 { Some((text[0].to_string(),text[1].to_string()))} else { None }
        }
        let notes : Vec<(String,String)> = html.select(&Selector::parse("div.note").unwrap()).map(|e|parse_note(e)).flatten().collect::<Vec<_>>();
        let get_note = |note_name:&str| notes.iter().find(|(name,_)|name.starts_with(note_name)).map(|(_,v)|v.clone()).ok_or_else(||anyhow!("Could not find note {}",note_name));
        let vacancies = NumberOfCandidates(if mayoral {1} else {get_note("Candidates to be Elected:")?.chars().filter(|&c|c!=',').collect::<String>().parse()?});
        let enrolment = NumberOfCandidates(get_note(if mayoral {"Electors Enrolled as on"} else {"Enrolment:"})?.chars().filter(|&c|c!=',').collect::<String>().parse()?);
        for row in table.select(&Selector::parse("tbody tr").unwrap()) {
            let mut cols = row.select(&selector_td);
            let col1 = cols.next().ok_or_else(||anyhow!("No first column"))?;
            let col2 = cols.next().ok_or_else(||anyhow!("No second column"))?;
            let col1_text = col1.text().map(|s|s.trim()).collect::<Vec<_>>().join("");
            let col2_text = col2.text().map(|s|s.trim()).collect::<Vec<_>>().join("");
            if let Some(row_class) = row.value().attr("class") {
                if row_class == "tr-total" && has_groups && (col2_text == "UNGROUPED CANDIDATES" || !col1_text.is_empty()) { // a party!
                    parties.push(Party {
                        column_id: (if col1_text.is_empty() { "UG" } else { &col1_text }).to_string(),
                        name: col2_text,
                        abbreviation: None,
                        atl_allowed: !col1_text.is_empty(),
                        candidates: vec![],
                        tickets: vec![]
                    });
                    current_position = 0;
                }
            } else if mayoral && col1.value().attr("class").is_some() {
                // this is a footnote. Do nothing.
            } else { // it is a candidate
                let name = if has_groups { col2_text } else { col1_text };
                if name.is_empty() { return Err(anyhow!("Empty candidate name")); }
                // could at this point get the second column of the mayoral table to get a party, although it is not so important for a Mayoral contest.
                current_position+=1;
                if let Some(current_party)=parties.last_mut() { current_party.candidates.push(CandidateIndex(candidates.len()))}
                candidates.push(Candidate{
                    name,
                    party: if parties.is_empty() { None } else { Some(PartyIndex(parties.len()-1))},
                    position: Some(current_position),
                    ec_id: None
                })
            }
        }
        Ok(ElectionMetadata{
            name: self.name(&electorate),
            candidates,
            parties,
            source: vec![],
            results: None,
            vacancies: Some(vacancies),
            enrolment: Some(enrolment),
            secondary_vacancies: None,
            excluded: vec![],
            tie_resolutions: Default::default()
        })
    }

    pub fn read_official_dop_transcript(&self,metadata:&ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        let contest = &self.find_contest(&metadata.name.electorate)?.url;
        let dop_file = self.find_raw_data_file_relative(contest,"download","dopfulldetails.xlsx","download/dopfulldetails.xlsx")?;
        let table = parse_xlsx_by_converting_to_csv_using_openoffice(&dop_file)?; // do it this way as calamine does not read this file properly.
        if table.len() < 7 { return Err(anyhow!("DoP spreadsheet is too short"))}
        let expected_number_columns = 11+4*metadata.candidates.len()+14;
        if table[0].len() != expected_number_columns { return Err(anyhow!("DoP spreadsheet has {} columns expecting {}",table[0].len(),expected_number_columns)) }
        println!("Read into a table with {} rows and {} columns",table.len(),table[0].len());
        let candidate_lookup = metadata.get_candidate_name_lookup();
        let col_candidate_papers = |c:CandidateIndex| 12+4*c.0;
        let col_count = 1;
        let col_description = 2;
        let col_type = 3;
        //let col_papers_start = 4;
        //let col_tv_start = 5;
        //let col_surplus = 7;
        //let col_surplus_fraction = 8;
        let col_ctv = 9;
        let col_exhausted_bps = col_candidate_papers(CandidateIndex(metadata.candidates.len()));
        //let col_aev = col_exhausted_bps+2;
        let col_votes_set_aside = col_exhausted_bps+3;
        let col_lost_rounding = col_exhausted_bps+7;
        let col_votes_lost = col_lost_rounding+4;
        let col_result = col_votes_lost+1;
        let quota = Some(QuotaInfo{
            papers: BallotPaperCount(table[1][2].parse::<usize>()?-table[7][col_exhausted_bps].parse::<usize>()?),
            vacancies: NumberOfCandidates(table[2][2].parse::<usize>()?),
            quota: table[3][2].parse::<f64>()?
        });
        assert_eq!(col_result+1,expected_number_columns);
        let mut counts = vec![];
        let mut about_to_be_excluded : Option<CandidateIndex> = None;
        for line_upto in 6..table.len() {
            let line = &table[line_upto];
            if line.len()!=expected_number_columns { return Err(anyhow!("DoP spreadsheet has {} columns on line {} expecting {}",line.len(),line_upto+1,expected_number_columns)) }
            let excluded_candidate_0_ballots = line[col_description].trim()=="Total" && about_to_be_excluded.is_some();
            if (line[col_description].is_empty() && !line[col_count].is_empty() && line[col_count].chars().all(|c|c=='.'||c.is_ascii_digit())) || excluded_candidate_0_ballots { // line is an actual count!
                let transfer_value = if excluded_candidate_0_ballots { None} else { Some(if line[col_ctv].is_empty() {1.0} else {line[col_ctv].parse::<f64>()?})};
                let mut vote_delta = PerCandidate::default();
                let mut paper_delta = PerCandidate::default();
                for c in metadata.candidate_indices() {
                    vote_delta.candidate.push(parse_number_blank_as_nan(&line[col_candidate_papers(c)+1]));
                    paper_delta.candidate.push(parse_number_blank_as_zero(&line[col_candidate_papers(c)]));
                }
                vote_delta.exhausted=parse_number_blank_as_nan(&line[col_votes_set_aside]);
                paper_delta.exhausted=parse_number_blank_as_zero(&line[col_exhausted_bps]);
                //vote_delta.rounding=parse_number_blank_as_NaN(&line[col_votes_set_aside]).into();// they track a different way. I compute the number of votes represented as a delta for the appropriate candidate, and round down. They don't round down (good, but messy), and don't display (bad, but avoids mess). Their rounding thus takes this into account.  TODO be able to track this.
                vote_delta.rounding=f64::NAN.into();
                counts.push(OfficialDOPForOneCount{
                    transfer_value,
                    elected: vec![], // will be set later.
                    excluded: about_to_be_excluded.take().into_iter().collect(),
                    vote_total: None,
                    paper_total: None,
                    vote_delta: Some(vote_delta),
                    paper_delta: Some(paper_delta),
                    count_name: Some(line[col_count].clone())
                });
            }
            if (line_upto==table.len()-1 || table[line_upto+1][col_count].is_empty() || table[line_upto+1][col_count]=="Candidate(s) marked with an asterisk were elected without reaching quota.") && !counts.is_empty() { // last line for a major count. People can get elected here, and cumulative tallies are available.
                if !line[col_result].is_empty() { // someone may have gotten elected.
                    let last_count = counts.last_mut().unwrap();
                    for what in line[col_result].split(',') {
                        let what = what.trim();
                        if let Some(candidate_name) = what.trim_end().trim_end_matches('*').trim_end().strip_suffix(" Elected") {
                            let who = *candidate_lookup.get(candidate_name).ok_or_else(||anyhow!("Could not find elected candidate {}",candidate_name))?;
                            last_count.elected.push(who);
                        }
                    }
                    let mut vote_total = PerCandidate::default();
                    let mut paper_total = PerCandidate::default();
                    for c in metadata.candidate_indices() {
                        vote_total.candidate.push(parse_number_blank_as_nan(&line[col_candidate_papers(c)+1]));
                        paper_total.candidate.push(parse_number_blank_as_zero(&line[col_candidate_papers(c)]) as usize);
                    }
                    vote_total.exhausted=0.0; // parse_number_blank_as_nan(&line[col_votes_set_aside]);
                    paper_total.exhausted=parse_number_blank_as_zero(&line[col_exhausted_bps]) as usize;
                    vote_total.rounding=parse_number_blank_as_nan(&line[col_votes_lost]).into();
                    last_count.vote_total=Some(vote_total);
                    last_count.paper_total=Some(paper_total);
                }
            } else if line[col_type]=="E" {
                let candidate_name = &line[col_description];
                let who = *candidate_lookup.get(candidate_name).ok_or_else(||anyhow!("Could not find excluded candidate {}",candidate_name))?;
                about_to_be_excluded=Some(who);
            }
        }
        Ok(OfficialDistributionOfPreferencesTranscript{
            quota,
            counts,
            missing_negatives_in_papers_delta: true,
            elected_candidates_are_in_order: false,
            all_exhausted_go_to_rounding : true,
        })
    }
}

fn parse_number_blank_as_nan(s:&str) -> f64 {
    let s = s.trim();
    if s.is_empty() { f64::NAN} else { s.parse().unwrap() }
}
fn parse_number_blank_as_zero(s:&str) -> isize {
    let s = s.trim();
    if s.is_empty() { 0 } else { s.parse().unwrap() }
}


/// Parse the zipped election file.
/// The function currently ignores non-formal markings.
/// Optionally, some particular vote types can be suppressed.
fn parse_zip_election_file(zipfile : File, metadata:ElectionMetadata, reject_vote_type : Option<HashSet<String>>, mayoral: bool) -> anyhow::Result<ElectionData> {
    let mut zipfile = zip::ZipArchive::new(zipfile)?;
    let zip_contents = zipfile.by_index(0)?;
    let mut reader = csv::ReaderBuilder::new().delimiter(b'\t').from_reader(zip_contents);
    let headings = reader.headers()?;
    let find_col = |heading_name:&str|headings.iter().position(|s|s==heading_name).ok_or_else(||anyhow!("Could not find {} column",heading_name));
    let vote_type_column = find_col("VoteType")?;
    let paper_id_column = find_col("VCBallotPaperID")?;
    let preference_number_column = find_col("PreferenceNumber")?; // if formal, the number written
    let candidate_name_column = find_col("CandidateName")?; // if BTL, the candidate name
    let group_column = if mayoral {0} else {find_col("GroupCode")?}; // Not applicable for mayoral.
    let mut helper = PreferencesComingOutOfOrderHelper::default();
    let candidate_name_lookup = metadata.get_candidate_name_lookup();
    let group_id_lookup = metadata.get_party_id_lookup();
    for record in reader.records() {
        let record = record?;
        helper.set_current_paper(&record[paper_id_column]);
        let preference = &record[preference_number_column];
        if !preference.is_empty() { // part of a formal vote
            if let Some(restrictions) = reject_vote_type.as_ref() {
                let vote_type = &record[vote_type_column];
                if restrictions.contains(vote_type) { continue }
            }
            let preference : usize = preference.parse()?;
            let candidate_name = &record[candidate_name_column];
            if candidate_name.is_empty() { // ATL vote
                let group_id = &record[group_column];
                let party = *group_id_lookup.get(group_id).ok_or_else(||anyhow!("Unknown group id {}",group_id))?;
                helper.add_atl_pref(preference,party)?;
            } else { // BTL vote
                let candidate = *candidate_name_lookup.get(candidate_name).ok_or_else(||anyhow!("Unknown candidate name {}",candidate_name))?;
                helper.add_btl_pref(preference,candidate)?;
            }
        }
    }
    Ok(helper.done(metadata))
}


/// Information from a file like https://vtr.elections.nsw.gov.au/LG2101/albury/councillor
pub struct NSWLGESingleContestMainPageInfo {
    /// The number of candidates to be elected. If it is None, then the contest is uncontested.
    pub to_be_elected : Option<usize>,
    /// The elected candidates, if known.
    pub elected_candidates : Vec<String>,
    /// The pretty name of the electorate
    pub name : String,
    /// The relative url to a the preferences zip file, if present.
    pub preferences : Option<String>,
    /// The relative url to a distribution of preferences zip file, if present.
    pub dop : Option<String>,
    /// The relative url to the table of votes by candidate, which can be used to get the names and groups of candidates.
    pub candidates_page : Option<String>,
    /// The relative url to the mayoral dop page, which can be used to find ineligible candidates.
    pub mayoral_dop : Option<String>,
    /// The relative url to the candidate results page, which can be used to find ineligible candidates.
    pub candidate_results : Option<String>,
}

impl NSWLGESingleContestMainPageInfo {
    fn extract_file<P:AsRef<Path>>(path:P) -> anyhow::Result<Self> {
        Self::extract_html(&Html::parse_document(&std::fs::read_to_string(path)?))
    }
    /// Extract required information from a file like https://vtr.elections.nsw.gov.au/LG2101/albury/councillor
    pub fn extract_html(html: &Html) -> anyhow::Result<Self> {
        let name = html.select(&Selector::parse("h1").unwrap()).next().ok_or_else(||anyhow!("No h1 element"))?.text().collect::<Vec<_>>().join("");
        let mut res = NSWLGESingleContestMainPageInfo{
            to_be_elected: Some(usize::MAX),
            elected_candidates: vec![],
            name,
            preferences: None,
            dop: None,
            candidates_page: None,
            mayoral_dop: None,
            candidate_results: None
        };
        // get relative files.
        for e in html.select(&Selector::parse("a").unwrap()) {
            if let Some(href) = e.value().attr("href") {
                if href.ends_with("dopfulldetails.xlsx") { res.dop=Some(href.to_string()); }
                if href.ends_with("finalpreferencedatafile.zip") { res.preferences=Some(href.to_string()); }
                if href.ends_with("fp-by-grp-and-candidate-by-vote-type") { res.candidates_page=Some(href.to_string()); }
                if href.ends_with("mayoral-fp-by-candidate") { res.candidates_page=Some(href.to_string()); }
                if href.ends_with("mayoral-dop") { res.mayoral_dop=Some(href.to_string()); }
                if href.ends_with("grp-and-candidates-result") { res.candidate_results=Some(href.to_string()); }
            }
        }
        // get number of councillors. Look for string like  `There are 9 Councillors to be elected from 51 candidates.` or `This election was UNCONTESTED, and there was no need to vote.`
        for e in html.root_element().text() {
            let e = e.trim();
            //println!("    {}",e);
            if e.starts_with("There are ") || e.starts_with("There is ") /* && e.contains("Councillors to be elected from") */ {
                let num : String = e.trim_start_matches("There are").trim_start_matches("There is").trim_start().chars().take_while(|&c|c.is_ascii_digit()).collect();
                res.to_be_elected=Some(num.parse()?);
            }
            if e.contains("This election was UNCONTESTED") { res.to_be_elected=None; }
        }
        if res.to_be_elected==Some(usize::MAX) { return Err(anyhow!("Could not find the number of people to be elected."))}
        // find elected councillors
        for e in html.select(&Selector::parse("div.declared-elected > span.candidate-name").unwrap()) {
            res.elected_candidates.push(e.inner_html());
        }
        Ok(res)
    }
}

fn inner_text(element:&ElementRef) -> String {
    element.text().collect::<Vec<_>>().join(" ").trim().to_string()
}
/// parse page like https://vtr.elections.nsw.gov.au/LG2101/ballina/b-ward/councillor/report/grp-and-candidates-result to get ineligible candidates
fn get_ineligible_candidates(html:&Html) -> anyhow::Result<Vec<CandidateIndex>> {
    let mut res = vec![];
    let table = html.select(&Selector::parse("table").unwrap()).next().ok_or_else(||anyhow!("No table element"))?;
    let thead_first = table.select(&Selector::parse("thead > tr > th").unwrap()).next().ok_or_else(||anyhow!("No table headings"))?;
    let has_groups = thead_first.inner_html()=="Group";
    let selector_td = Selector::parse("td").unwrap();
    let mut candidate_index = 0;
    for row in table.select(&Selector::parse("tbody tr").unwrap()) {
        let col = row.select(&selector_td).map(|e|inner_text(&e)).collect::<Vec<_>>();
        if col.len()<3 {return Err(anyhow!("Fewer than 3 columns")); }
        let is_group_row = has_groups && (col[0].len()>0 || col[1]=="UNGROUPED CANDIDATES");
        if !is_group_row {
            if col[if has_groups {2} else {1}]=="INELIGIBLE" {
                res.push(CandidateIndex(candidate_index));
            }
            candidate_index+=1;
        }
    }
    Ok(res)
}