// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.



use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{anyhow, Context};
use stv::ballot_metadata::{Candidate, CandidateIndex, DataSource, ElectionMetadata, ElectionName, NumberOfCandidates, Party, PartyIndex};
use stv::datasource_description::{AssociatedRules, Copyright, ElectionDataSource};
use stv::election_data::ElectionData;
use stv::official_dop_transcript::{OfficialDistributionOfPreferencesTranscript, OfficialDOPForOneCount};
use stv::parse_util::{FileFinder, KnowsAboutRawMarkings, MissingFile, RawDataSource};
use stv::tie_resolution::TieResolutionsMadeByEC;
//use crate::WALegislativeCouncil;
use calamine::{DataType, open_workbook_auto};
use stv::ballot_pile::BallotPaperCount;
use stv::distribution_of_preferences_transcript::{CountIndex, PerCandidate, QuotaInfo};
use stv::download::{CacheDir, DownloadWithReqwest};
use stv::signed_version::SignedVersion;
use scraper::{ElementRef, Html, Selector};

pub fn get_wa_data_loader_2005(finder:&FileFinder) -> anyhow::Result<WADataLoader> {
    WADataLoader::new(finder, "2005") // Note - not all needed files are public. Ask the WAEC nicely if you want them, and maybe they will oblige. "https://www.elections.wa.gov.au/elections/state/sgelection#/sg2005"
}


/// Do not use on website as votes are not published.
pub struct WADataSource {}

impl ElectionDataSource for WADataSource {
    fn name(&self) -> Cow<'static, str> { "WA Legislative Council".into() }
    fn ec_name(&self) -> Cow<'static, str> { "Western Australian Electoral Commission".into() }
    fn ec_url(&self) -> Cow<'static, str> { "https://www.elections.wa.gov.au/".into() }
    fn years(&self) -> Vec<String> { vec!["2005".to_string(),"2008".to_string(),"2013".to_string(),"2017".to_string(),"2021".to_string()] }
    fn get_loader_for_year(&self,year: &str,finder:&FileFinder) -> anyhow::Result<Box<dyn RawDataSource+Send+Sync>> {
        Ok(Box::new(WADataLoader::new(finder,year)?))
    }
}

pub struct WADataLoader {
    finder : FileFinder,
    archive_location : String,
    year : String,
    page_url : String,
    cache : CacheDir,
}

impl KnowsAboutRawMarkings for WADataLoader {}

impl RawDataSource for WADataLoader {
    fn name(&self, electorate: &str) -> ElectionName {
        ElectionName {
            year: self.year.clone(),
            authority: "Western Australian Electoral Commission".to_string(),
            name: "WA Legislative Council".to_string(),
            electorate: electorate.to_string(),
            modifications: vec![],
            comment: None,
        }
    }

    fn candidates_to_be_elected(&self, _region: &str) -> NumberOfCandidates {
        match self.year.as_str() {
            "2005" => NumberOfCandidates(5),
            _ => NumberOfCandidates(6),
        }
    }

    /// These are deduced by looking at the actual transcript of results.
    /// I have not included anything if all decisions are handled by the fallback "earlier on the ballot paper candidates are listed in worse positions.
    fn ec_decisions(&self, _electorate: &str) -> TieResolutionsMadeByEC {
        Default::default()
    }

    /// These are due to a variety of events.
    fn excluded_candidates(&self, _electorate: &str) -> Vec<CandidateIndex> {
        Default::default()
    }

    fn find_raw_data_file(&self, filename: &str) -> Result<PathBuf, MissingFile> {
        self.finder.find_raw_data_file(filename, &self.archive_location, &self.page_url)
    }
    fn all_electorates(&self) -> Vec<String> {
//        let year_as_num : u32 = self.year.chars().filter(|c|c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0);
        let mut res = vec![
            "Agricultural".to_string(),
            "East Metropolitan".to_string(),
            "Mining and Pastoral".to_string(),
            "North Metropolitan".to_string(),
            "South Metropolitan".to_string(),
            "South West".to_string(),
        ];
        res.sort();
        res
    }
    fn read_raw_data(&self, _electorate: &str) -> anyhow::Result<ElectionData> {
        Err(anyhow!("Raw data not available"))
    }

    fn read_raw_data_best_quality(&self, _electorate: &str) -> anyhow::Result<ElectionData> {
        Err(anyhow!("Raw data not available"))
        // read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<WALegislativeCouncil,Self>(self, electorate)
    }

    /// Get the metadata from the file like south-easternmetropolitanregionvotesreceived.xls
    fn read_raw_metadata(&self,electorate:&str) -> anyhow::Result<ElectionMetadata> {
        let url_candidates_and_parties = format!("https://api.elections.wa.gov.au/sgElections/sg{}/LCCandidateParty", self.year);
        let path_candidates_and_parties = self.find_raw_data_file_from_cache(&url_candidates_and_parties)?;
        let (parties,candidates) = parse_json_lc_candidate_party(&path_candidates_and_parties,electorate)?;
        let url_winners = format!("https://api.elections.wa.gov.au/sgElections/sg{}/LCElectedMembers", self.year);
        let path_winners = self.find_raw_data_file_from_cache(&url_winners)?;
        let mut metadata = ElectionMetadata{
            name: self.name(electorate),
            candidates,
            parties,
            source: vec![DataSource::new(&url_candidates_and_parties,&path_candidates_and_parties),
                         DataSource::new(&url_winners,&path_winners),],
            results: None,
            vacancies: Some(self.candidates_to_be_elected(electorate)),
            enrolment: None,
            secondary_vacancies: None,
            excluded: vec![],
            tie_resolutions: Default::default()
        };
        if let Some((region,winners)) = parse_json_lc_elected_members(&path_winners)?.into_iter().find(|(region,_)|region.name==electorate) {
            let candidates_to_index = metadata.get_candidate_name_lookup();
            let mut winner_indices = vec![];
            for winner in winners {
                if let Some(candidate) = candidates_to_index.get(&winner) {
                    winner_indices.push(*candidate);
                } else {
                    return Err(anyhow!("Could not find winning candidate {} in candidate list {:?}",winner,metadata.candidates))
                }
            }
            metadata.results=Some(winner_indices);
            // See https://www.elections.wa.gov.au/elections/state/sgelection#/sg2021/region/4/results
            let url_overview = format!("https://api.elections.wa.gov.au/sgElections/sg{}/region/{}/overview",self.year,region.code);
            let path_overview = self.find_raw_data_file_from_cache(&url_overview)?;
            let overview = JsonRegionOverview::parse(&path_overview)?;
            metadata.enrolment = Some(NumberOfCandidates(overview.enrolment));
            metadata.source.push(DataSource::new(&url_overview,&path_overview));
            if overview.tickets_available { // only seems like 2021
                let url_tickets = format!("https://api.elections.wa.gov.au/sgElections/sg{}/region/{}/ticketVotes",self.year,region.code);
                let path_tickets = self.find_raw_data_file_from_cache(&url_tickets)?;
                parse_json_tickets(&mut metadata,&path_tickets)?;
                metadata.source.push(DataSource::new(&url_tickets,&path_tickets));
            }
            let url_results = format!("https://api.elections.wa.gov.au/sgElections/sg{}/region/{}/results",self.year,region.code);
            let _path_results = self.find_raw_data_file_from_cache(&url_results)?;
            // TODO use path_results to get the number of first preference votes. Or maybe put it in data.
            // See https://www.elections.wa.gov.au/elections/state/sgelection#/sg2021/region/4/results for a button to https://www.elections.wa.gov.au/elections/state/sgelection#/sg2021/region/4/ticketvotes
            // which really calls https://api.elections.wa.gov.au/sgElections/sg2021/region/4/ticketVotes
            // enrollment and whether tickets are available is from https://api.elections.wa.gov.au/sgElections/sg2021/region/4/overview and number of ticket votes from https://api.elections.wa.gov.au/sgElections/sg2021/region/4/results
        }
        Ok(metadata)
    }

    fn copyright(&self) -> Copyright {
        Copyright {
            statement: Some("© State of Western Australia".into()),
            url: Some("https://www.elections.wa.gov.au/copyright".into()),
            license_name: None,
            license_url: None,
        }
    }

    fn rules(&self, _electorate: &str) -> AssociatedRules {
        match self.year.as_str() {
            "2008" => AssociatedRules {
                rules_used: Some("WA2008".into()),
                rules_recommended: Some("WA2008".into()),
                comment: Some("The legislation is slightly ambiguous. This is my interpretation of what the WAEC did in 2008 which seems to me to be a plausible interpretation of the legislation.".into()),
                reports: vec![],
            },
            _ => AssociatedRules { rules_used: None, rules_recommended: None, comment: Some("The WAEC does not publish a full DoP allowing me to work out what rules they used for years other than 2008.".into()), reports: vec![] },
        }
    }

    fn read_official_dop_transcript(&self, metadata: &ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        let pages_url = format!("https://api.elections.wa.gov.au/sgElections/sg{}/LCPages",self.year);
        let pages_path = self.find_raw_data_file_from_cache(&pages_url)?;
        let detailed_relative_url = parse_json_list_of_detailed_results(&pages_path,&metadata.name.electorate)?;
        let detailed_url : String = url::Url::parse("https://www.elections.wa.gov.au/")?.join(&detailed_relative_url)?.to_string();
        let detailed_path = self.find_raw_data_file_from_cache(&detailed_url)?;
        use calamine::Reader;
        let mut workbook1 = open_workbook_auto(&detailed_path)?;
        let sheet1 = workbook1.worksheet_range_at(0).ok_or_else(||anyhow!("No sheets in {}",detailed_path.to_string_lossy()))??;
        parse_dop_excel_file(&sheet1,metadata,self.year=="2008").context(detailed_path.to_string_lossy().to_string())
    }
}

fn merge_whitespace_to_space(s:&str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}


/// Parse the  DoP summary file in excel format.
///
/// For years 2005/2013/17/21 it is just a summary of the DoP.
/// Note that there is not much useful information in this DoP summary
/// * It does not contain each count, but merges together minor counts unless someone is elected
/// * It doesn't mention the number of papers involved
/// These combine to make the "DoP" pretty useless, especially given that raw votes are not provided.
///
/// full_dop is false for these years.
///
/// For year 2008 they produced a pretty full DoP.
fn parse_dop_excel_file(sheet:&calamine::Range<DataType>,metadata:&ElectionMetadata,full_dop:bool) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript>{
    let string_value = |row:u32,col: u32| { sheet.get_value((row, col)).and_then(|v|v.get_string()) };
    let is_first_preference_row = |row:&usize| match string_value(*row as u32,0) {
        Some("First Preferences") | Some("1st Preferences") => true,
        _ => false,
    };
    let first_preference_row = (1..sheet.height()).into_iter().find(is_first_preference_row).ok_or_else(||anyhow!("Could not find first preferences"))? as u32;
    let has_quota = first_preference_row==13;
    let quota : Option<QuotaInfo<f64>> = if has_quota {
        let quota_string = sheet.get_value((11,0)).and_then(|v|v.get_string()).ok_or_else(||anyhow!("Could not get quota string from cell A12"))?.trim();
        let quota_string_without_commas = quota_string.replace(',',"");
        let regex_find_quota = regex::Regex::new(r"^Quota\s*=\s*(\d+)/\((\d+)\+1\)\+1\s*=\s*(\d+)$").unwrap();
        let captures = regex_find_quota.captures(&quota_string_without_commas).ok_or_else(||anyhow!("Could not interpret quota string {} from cell A12",quota_string_without_commas))?;
        Some(QuotaInfo{
            papers: BallotPaperCount(captures[1].parse::<usize>()?),
            vacancies: NumberOfCandidates(captures[2].parse::<usize>()?),
            quota: captures[3].parse::<f64>()?,
        })
    } else { None };
    let col_who_caused_count : u32 = 0;
    let col_why_count : u32 = 1;
    let col_count_no : Option<u32> = if full_dop { Some(2) } else { None };
    let col_from_count_no : Option<u32> = if full_dop { Some(3) } else { None };
    let col_transfer_value : Option<u32> = if full_dop { Some(6) } else { None };
    let col_start_candidates : u32 = if full_dop {7} else if string_value(first_preference_row,2)==Some("Votes") {3} else {2};
    let col_candidate = |candidate_index:usize|col_start_candidates+candidate_index as u32;
    let col_lost_fraction = col_candidate(metadata.candidates.len());
    let col_totals = col_lost_fraction+1;
    let col_elected = col_totals+1;
    let row_headings : u32 = match first_preference_row { 13=>5, 7=>6, _ => 2};
    let assert_heading_string = |col:u32,expected:&str| {
        match string_value(row_headings,col) {
            None => Err(anyhow!("Row {} col {} expected heading {} found no string",row_headings+1,col+1,expected)),
            Some(found) if expected==&merge_whitespace_to_space(found) => Ok(()),
            Some(found) => Err(anyhow!("Row {} col {} expected heading {} found {}",row_headings+1,col+1,expected,found))
        }
    };
    // sanity check headings
    if let Some(col_count_no) = col_count_no { assert_heading_string(col_count_no,"Count No.")?; }
    if let Some(col_from_count_no) = col_from_count_no { assert_heading_string(col_from_count_no,"From Count No.")?; }
    if let Some(col_transfer_value) = col_transfer_value { assert_heading_string(col_transfer_value,"Transfer Value")?; }
    for i in 0..metadata.candidates.len() {
        assert_heading_string(col_candidate(i),&metadata.candidates[i].name)?;
    }
    assert_heading_string(col_lost_fraction,"Lost Fractions")?;
    if first_preference_row==13 {
        assert_heading_string(col_totals,"TOTALS")?;
        assert_heading_string(col_elected,if full_dop {"Outcome"} else {"Elected"})?;
    }
    // parse counts
    let mut counts = vec![];
    let candidate_of_name = metadata.get_candidate_name_lookup();
    let mut row :u32 = first_preference_row; // the first of a pair (usually) of rows describing a count.
    let mut num_elected : usize = 0;
    let mut count_names : HashMap<String,CountIndex> = Default::default();
    let all_on_one_row = first_preference_row==3;
    let mut last_int_count_no : u32 = 0;
    let mut last_subcount_no : u32 = 0;
    while num_elected<metadata.vacancies.unwrap().0 {
        if row as usize>sheet.height() {
            if metadata.name.year=="2005" { break; } // 2005 doesn't say who the last person elected is.
            else { return Err(anyhow!("Reached end of spreadsheet with only {} elected",num_elected)) }
        }
        let is_first_pref = row==first_preference_row;
        let candidate_causing_count : Option<CandidateIndex> = match string_value(row,col_who_caused_count) {
            Some("First Preferences") => None,
            Some("1st Preferences") => None,
            Some(candidate_name) => {
                if let Some(candidate_index) = candidate_of_name.get(candidate_name.trim_start_matches("*")) { Some(*candidate_index)}
                else if let Some(shorted_candidate_name) = metadata.candidates.iter().position(|c|c.name.replace(',',"").replace(' ',"").starts_with(&candidate_name.replace(',',"").replace(' ',""))) { Some(CandidateIndex(shorted_candidate_name))} // 2005 shortens the names of candidates in this position, and often leaves out spaces or commas.
                else {return Err(anyhow!("Did not understand candidate {}",candidate_name))}
            },
            None => None,
        };
        let why_count = string_value(row,col_why_count);
        let excluded : Vec<CandidateIndex> = if why_count==Some("Exclusion") { candidate_causing_count.into_iter().collect() } else { vec![] };
        let min_possible_rows_in_this_count : u32 = if is_first_pref {1} else {2};
        let num_rows_in_this_count : u32 =
            if all_on_one_row {1}
            else if full_dop {(min_possible_rows_in_this_count..).into_iter().find(|ahead|row+*ahead>sheet.height() as u32 || sheet.get_value((row+*ahead,col_count_no.unwrap())).and_then(|s|s.get_float()).is_some() ).unwrap()} // usually 4, sometimes additional for elected, total, lost fractions
            else {(min_possible_rows_in_this_count..).into_iter().find(|ahead|string_value(row+*ahead,col_why_count).is_some() || string_value(row+*ahead,col_elected).is_none() ).unwrap()}; // usually 2
        let mut elected : Vec<CandidateIndex> = vec![];
        for elected_row in 0..num_rows_in_this_count {
            if let Some(elected_description) = string_value(row+elected_row,col_elected) {
                if let Some((_,elected_name)) = elected_description.split_once('.') { // trim off "1st." or "2nd." or "1." etc.
                    if let Some(candidate) = candidate_of_name.get(elected_name.trim()) {
                        elected.push(*candidate);
                    } else { return Err(anyhow!("Could not understand elected description {} - unknown candidate name",elected_description))}
                } else { return Err(anyhow!("Could not understand elected description {} - could not find period",elected_description))}
            }
        }
        if why_count.is_some() && why_count.unwrap().starts_with("Elected") { elected.push(candidate_causing_count.unwrap()); } // this is a travesty! 2005,2013 It is not stated when the candidate is actually elected, this is a surplus distribution
        let f64_value_or_0  = |deltarow:u32,col:u32| -> f64 {
            if all_on_one_row { // one cell contains both numbers as text with a newline, or a single line for first prefs.
                if let Some(text) = sheet.get_value((row+deltarow,col)).and_then(|v|v.get_string()) {
                    let lines : Vec<&str> = text.trim().split_whitespace().collect();
                    match lines.len() {
                        0 => 0.0,
                        1 => lines[0].trim_start_matches("'").parse::<f64>().unwrap_or(0.0),
                        2 => lines[deltarow as usize].parse::<f64>().unwrap_or(0.0),
                        _ => 0.0, // could give an error?
                    }
                } else {0.0}
            } else { sheet.get_value((row+deltarow,col)).and_then(|v|v.get_float()).unwrap_or(0.0) }
        };
        let count_name = if let Some(col_count_no) = col_count_no { // the count no is stored as a floating point number! This means that 6.1 and 6.10 are stored identically. This can be undone... but it is a problem for the papers_came_from.
            let count_no = if is_first_pref {1.1} else {sheet.get_value((row,col_count_no)).and_then(|v|v.get_float()).ok_or_else(||anyhow!("No count no. at row {}",row+1))?}; // something about the formatting of count 1.1 doesn't play well.
            if count_no.floor() > last_int_count_no as f64 { last_int_count_no=count_no.floor() as u32; last_subcount_no=1; } else { last_subcount_no+=1; }
            let count_name = format!("{}.{}",last_int_count_no,last_subcount_no);
            count_names.insert(count_name.clone(),CountIndex(counts.len()));
            Some(count_name)
        } else {None};
        let papers_came_from_counts : Option<Vec<CountIndex>> = if let Some(col_from_count_no) = col_from_count_no {
            if let Some(from_count_no) = sheet.get_value((row,col_from_count_no)).and_then(|v|v.get_float()) {
                let from_count_name = from_count_no.to_string();
                let from_count_id = *count_names.get(&from_count_name).ok_or_else(||anyhow!("Could not find from count no. {}",from_count_name))?;
                // this may be wrong. 6.10 and 6.1 are stored the same. If ambiguous, state the answer is unknown.
                let is_ambiguous = count_names.contains_key(&(from_count_name+"0"));
                if is_ambiguous { None } else { Some(vec![from_count_id]) }
            } else {Some(vec![])}
        } else {None};
        let transfer_value = if let Some(col_transfer_value) = col_transfer_value { Some(f64_value_or_0(0,col_transfer_value)) } else { None };
        let get_row = |rowdelta:u32|-> PerCandidate<f64> {
            let candidate : Vec<f64> = (0..metadata.candidates.len()).into_iter().map(|candidate_index|f64_value_or_0(rowdelta,col_candidate(candidate_index))).collect();
            let rounding = f64_value_or_0(rowdelta,col_lost_fraction);
            PerCandidate {
                candidate,
                exhausted : 0.0,
                rounding : SignedVersion::from(rounding),
                set_aside: None
            }
        };
        let row0 = get_row(0);
        num_elected+=elected.len();
        counts.push(OfficialDOPForOneCount{
            transfer_value,
            elected,
            excluded,
            vote_total: Some(if is_first_pref {row0.clone()} else {get_row(if full_dop {3} else {1})}),
            paper_total: if is_first_pref {Some(row0.clone().try_into()?)} else if full_dop {Some(get_row(1).try_into()?)} else {None},
            vote_delta: Some(if full_dop&&!is_first_pref {get_row(2)} else {row0.clone()}),
            paper_delta: if is_first_pref||full_dop {Some(row0.clone().try_into()?)} else {None},
            paper_set_aside_for_quota: None,
            count_name,
            papers_came_from_counts,
        });
        row+=num_rows_in_this_count;
    }
    Ok(OfficialDistributionOfPreferencesTranscript{ quota, counts ,missing_negatives_in_papers_delta:false, elected_candidates_are_in_order: true, all_exhausted_go_to_rounding: false, negative_values_in_surplus_distributions_and_rounding_may_be_off: true })
}



impl WADataLoader {
    fn new(finder:&FileFinder,year:&str) -> anyhow::Result<Self> {
        let archive_location = format!("WA/State{}",year);
        let cache = CacheDir::new(finder.path.join(&archive_location));
        Ok(WADataLoader {
            finder : finder.clone(),
            archive_location,
            year: year.to_string(),
            page_url: format!("https://www.elections.wa.gov.au/elections/state/sgelection#/sg{}",year),
            cache,
        })
    }

    // Find the path to an existing file, or useful error if it doesn't exist.
    /// Don't try to download.
    fn find_raw_data_file_from_cache(&self,url:&str) -> anyhow::Result<PathBuf> {
        self.cache.find_raw_data_file_from_cache_or_download::<DownloadWithReqwest>(url)
        //Ok(self.cache.find_raw_data_file_from_cache(url)?)
    }

}

struct WARegion {
    name : String,
    code : String,
}

impl WARegion {
    /// parse a json array of elements that look like
    /// ```json
    /// 	{
    /// 		"ElectorateElectionEventId": "089EBB6C-C325-4AEB-A148-0FE99D438324",
    /// 		"ElectorateName": "Agricultural",
    /// 		"ElectorateCode": "4",
    /// 		"ElelctorateType": "Region",
    /// 		"Lat": "-31.42514754",
    /// 		"Long": "116.4111326",
    /// 		"Zoom": "6"
    /// 	}
    /// ```
    /// Extract the ElectorateName and ElectorateCode for those with "ElelctorateType": "Region" (sic).
    fn get_regions(json_array:&serde_json::Value) -> anyhow::Result<Vec<WARegion>> {
        let mut res = vec![];
        for elem in json_array.as_array().ok_or_else(||anyhow!("json given to WARegion::get_regions is not an array"))? {
            if let Some(map) = elem.as_object() {
                let lookup = |key:&str| { map.get(key).and_then(|v|v.as_str()) };
                match lookup("ElelctorateType") {
                    Some("Region") => {
                        if let Some(name) = lookup("ElectorateName") {
                            if let Some(code) = lookup("ElectorateCode") {
                                res.push(WARegion{name:name.to_string(),code:code.to_string()});
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(res)
    }
}
/*
/// The information that is in the main page of an election, such as
/// [https://www.elections.wa.gov.au/elections/state/sgelection#/sg2017]
/// This actually comes from a JSON file at
/// [https://api.elections.wa.gov.au/sgElections/sg2017]
struct WAElectionMainPageData {
    regions : Vec<WARegion>,

}
*/
/// Parse the JSON file containing info on who was elected.
/// [https://api.elections.wa.gov.au/sgElections/sg2017/LCElectedMembers]
fn parse_json_lc_elected_members(path:&PathBuf) -> anyhow::Result<Vec<(WARegion,Vec<String>)>> {
    let json_text = std::fs::read_to_string(path)?;
    let json : serde_json::Value = serde_json::from_str(&json_text)?;
    let json = json.as_object().ok_or_else(||anyhow!("Parsed file {} is JSON but does not contain a JSON object",path.to_string_lossy()))?;
    let require = |key:&str| json.get(key).ok_or_else(||anyhow!("Parsed file {} is JSON but does not contain a JSON object with a field {}",path.to_string_lossy(),key));
    let regions = WARegion::get_regions(require("electorates")?)?;
    let mut elected : Vec<(WARegion,Vec<String>)> = regions.into_iter().map(|r|(r,vec![])).collect();
    let members = require("LCElectedMembers")?.as_array().ok_or_else(||anyhow!("Parsed file {} is JSON with an LCElectedMembers field that is not an array.",path.to_string_lossy()))?;
    for member in members {
        let member_map = member.as_object().ok_or_else(||anyhow!("Parsed file {} is JSON with an LCElectedMembers containing a member {} that is not an object.",path.to_string_lossy(),member))?;
        let lookup = |key:&str| member_map.get(key).and_then(|v|v.as_str()).ok_or_else(||anyhow!("Parsed file {} contains a member {} without a {} string field .",path.to_string_lossy(),member,key));
        let region = lookup("ELECTORATE_FULLY_QUALIFIED_NAME")?;
        let name = lookup("BALLOT_PAPER_NAME")?;
        let (_,elected_list) = elected.iter_mut().find(|(v,_)|&v.name==region).ok_or_else(||anyhow!("Parsed file {} contains a region {} that don't match the electorate regions.",path.to_string_lossy(),region))?;
        elected_list.push(name.to_string());
    }
    Ok(elected)
}


/// Parse the JSON file containing info on who is in what party for some region
/// [https://api.elections.wa.gov.au/sgElections/sg2017/LCCandidateParty]
fn parse_json_lc_candidate_party(path:&PathBuf,region:&str) -> anyhow::Result<(Vec<Party>,Vec<Candidate>)> {
    let json_text = std::fs::read_to_string(path)?;
    let json : serde_json::Value = serde_json::from_str(&json_text)?;
    let json = json.as_object().ok_or_else(||anyhow!("Parsed file {} is JSON but does not contain a JSON object",path.to_string_lossy()))?;
    let require = |key:&str| json.get(key).ok_or_else(||anyhow!("Parsed file {} is JSON but does not contain a JSON object with a field {}",path.to_string_lossy(),key));
    let members = require("LCCandidateParty")?.as_array().ok_or_else(||anyhow!("Parsed file {} is JSON with an LCCandidateParty field that is not an array.",path.to_string_lossy()))?;
    let mut last_column : Option<&str> = None;
    let mut parties : Vec<Party> = vec![];
    let mut candidates : Vec<Candidate> = vec![];
    for member in members {
        let member_map = member.as_object().ok_or_else(||anyhow!("Parsed file {} is JSON with an LCCandidateParty containing a member {} that is not an object.",path.to_string_lossy(),member))?;
        let lookup = |key:&str| member_map.get(key).and_then(|v|v.as_str()).ok_or_else(||anyhow!("Parsed file {} contains a member {} without a {} string field .",path.to_string_lossy(),member,key));
        if region==lookup("ELECTORATE_NAME")? {
            let name = lookup("BALLOT_PAPER_NAME")?;
            let column = lookup("BALLOT_PAPER_ORDER")?;
            if last_column!=Some(column) {
                let party = lookup("Party")?;
                parties.push(Party{
                    column_id: column.to_string(),
                    name: party.to_string(),
                    abbreviation: None,
                    atl_allowed: true,
                    candidates: vec![],
                    tickets: vec![],
                });
                last_column=Some(column);
            }
            parties.last_mut().unwrap().candidates.push(CandidateIndex(candidates.len()));
            candidates.push(Candidate{
                name: name.to_string(),
                party: Some(PartyIndex(parties.len()-1)),
                position: Some(parties.last().unwrap().candidates.len()),
                ec_id: None,
            });
        }
    }
    Ok((parties,candidates))
}

/// Parse the list of detailed results reports.
/// [https://www.elections.wa.gov.au/elections/state/sgelection#/sg2017/LCDetailResults]
/// which is actually parsing
/// [https://api.elections.wa.gov.au/sgElections/sg2017/LCPages]
/// which is a JSON file containing an object containing a field `variables` which contains a field `ElectionLCResultsStatistics` which is a string representation of HTML looking like
/// ```html
/// <p>Detailed results and first preference analysis reports for the six Legislative Council regions are available for download.</p><h2>    Detailed results reports</h2><p><a href="/sites/default/files/waec/sg_elections/LCDetailedResults/sg2017/Agricultural_LCDetailedResults2017.xlsx" target="_blank">Agricultural Region</a>&nbsp;(XLS, 175 kB)<br />    <a href="/sites/default/files/waec/sg_elections/LCDetailedResults/sg2017/EastMetro_LCDetailedResults2017.xlsx" target="_blank">East Metropolitan Region</a>&nbsp;(XLS, 287 kB)<br />    <a href="/sites/default/files/waec/sg_elections/LCDetailedResults/sg2017/MiningPastoral_LCDetailedResults2017.xlsx" target="_blank">Mining and Pastoral Region</a> (XLS, 117 kB)<br />    <a href="/sites/default/files/waec/sg_elections/LCDetailedResults/sg2017/NorthMetro_LCDetailedResults2017.xlsx" target="_blank">North Metropolitan Region</a> (XLS, 165 kB)<br />    <a href="/sites/default/files/waec/sg_elections/LCDetailedResults/sg2017/SouthMetro_LCDetailedResults2017.xlsx" target="_blank">South Metropolitan Region</a> (XLS, 351 kB)<br />    <a href="/sites/default/files/waec/sg_elections/LCDetailedResults/sg2017/SouthWest_LCDetailedResults2017.xlsx" target="_blank">South West Region</a> (XLS, 208 kB)</p><h2>    First preference analysis reports</h2><p>Reports include a region summary calculation.</p><p><a href="/sites/default/files/waec/sg_elections/LCDetailedResults/sg2017/Agricultural_CalculationResult.xlsx" target="_blank">Agricultural Region</a>&nbsp;(XLS, 42 kB)<br />    <a href="/sites/default/files/waec/sg_elections/LCDetailedResults/sg2017/EastMetro_CalculationResult.xlsx" target="_blank">East Metropolitan Region</a> (XLS, 38 kB)<br />    <a href="/sites/default/files/waec/sg_elections/LCDetailedResults/sg2017/MiningPastoral_CalculationResult.xlsx" target="_blank">Mining and Pastoral Region</a> (XLS, 43 kB)<br />    <a href="/sites/default/files/waec/sg_elections/LCDetailedResults/sg2017/NorthMetro_CalculationResult.xlsx" target="_blank">North Metropolitan Region</a> (XLS, 37 kB)<br />    <a href="/sites/default/files/waec/sg_elections/LCDetailedResults/sg2017/SouthMetro_CalculationResult.xlsx" target="_blank">South Metropolitan Region</a> (XLS, 50 kB)<br />    <a href="/sites/default/files/waec/sg_elections/LCDetailedResults/sg2017/SouthWest_CalculationResult.xlsx" target="_blank">South West Region</a> (XLS, 39 kB)</p>
/// ```
///
/// Returns the relative url for the distribution of preferences for the provided region.
pub fn parse_json_list_of_detailed_results(path:&PathBuf,region:&str) -> anyhow::Result<String> {
    let json_text = std::fs::read_to_string(path)?;
    let json : serde_json::Value = serde_json::from_str(&json_text)?;
    let json = json.as_object().ok_or_else(||anyhow!("Parsed file {} is JSON but does not contain a JSON object",path.to_string_lossy()))?;
    let require = |key:&str| json.get(key).ok_or_else(||anyhow!("Parsed file {} is JSON but does not contain a JSON object with a field {}",path.to_string_lossy(),key));
    let variables = require("variables")?.as_object().ok_or_else(||anyhow!("Parsed file {} is JSON with an variables field that is not an object.",path.to_string_lossy()))?;
    let html_str = variables.get("ElectionLCResultsStatistics").and_then(|v|v.as_str()).ok_or_else(||anyhow!("Parsed file {} is JSON with an variables object without a string ElectionLCResultsStatistics.",path.to_string_lossy()))?;
    let html = Html::parse_document(html_str);
    for a in html.select(&Selector::parse("a").unwrap()) {
        let name = text_content(&a);
        let name = name.trim().trim_end_matches("Region").trim();
        if name==region {
            if let Some(url) = a.value().attr("href") {
                return Ok(url.to_string());
            }
        }
    }
    for a in html.select(&Selector::parse("a").unwrap()) { // look for summary - 2005
        let name = text_content(&a);
        let name = name.trim().trim_end_matches("count summary").trim().trim_end_matches("Region").trim();
        if name==region {
            if let Some(url) = a.value().attr("href") {
                return Ok(url.to_string());
            }
        }
    }
    Err(anyhow!("Could not find region {} in {}",region,html_str))
}

/// Get all the text in an element as a string
fn text_content(e:&ElementRef<'_>) -> String {
    e.text().collect::<Vec<_>>().join("")
}

struct JsonRegionOverview {
    enrolment: usize,
    tickets_available : bool,
}

impl JsonRegionOverview {
    /// Parse the json file like [https://api.elections.wa.gov.au/sgElections/sg2021/region/4/overview]
    /// which contains variables.ElectionLCVoteTicketsShow which is `"1"` if tickets are available, `"0"` otherwise
    /// and toptotals.NUMBER_ELECTORS which is a string.
    fn parse(path:&PathBuf) -> anyhow::Result<Self> {
        let json_text = std::fs::read_to_string(path)?;
        let json : serde_json::Value = serde_json::from_str(&json_text)?;
        let tickets_available = match json["variables"]["ElectionLCVoteTicketsShow"].as_str() {
            Some("0") => false,
            Some("1") => true,
            _ => return Err(anyhow!("No field variables.ElectionLCVoteTicketsShow containing 0 or 1")),
        };
        let enrollment : usize = match json["toptotals"]["NUMBER_ELECTORS"].as_str() {
            Some(num) => num.parse()?,
            _ => return Err(anyhow!("No field toptotals.NUMBER_ELECTORS containing a string")),
        };
        Ok(JsonRegionOverview{ enrolment: enrollment,tickets_available})
    }
}

/// Get tickets and party abbreviations from a json file like [https://api.elections.wa.gov.au/sgElections/sg2021/region/4/ticketVotes]
fn parse_json_tickets(metadata:&mut ElectionMetadata,path:&PathBuf) -> anyhow::Result<()> {
    let json_text = std::fs::read_to_string(path)?;
    let json : serde_json::Value = serde_json::from_str(&json_text)?;
//    let parties = metadata.get_party_name_lookup();
    let candidates = metadata.get_candidate_name_lookup();
    let num_candidates = metadata.candidates.len();
    for group in json["Group"].as_array().ok_or_else(||anyhow!("Parsed file {} does not have a group array.",path.to_string_lossy()))? {
        let require = |key:&str| group[key].as_str().ok_or_else(||anyhow!("Parsed file {} has a group {} missing a field {}",path.to_string_lossy(),group,key));
        let party_index = PartyIndex(require("BallotPaperOrder")?.parse::<usize>()?-1);
        let party = &mut metadata.parties[party_index.0];
        if let Some(party_name) = group["BallotPaperName"].as_str() {
            if party_name!=&party.name { return Err(anyhow!("Parsed file {} has a group {} with an unexpected party name {} expecting {}",path.to_string_lossy(),group,party_name,party.name)); }
        }
        if let Some(abbreviation) = group["WAECAbbreviation"].as_str() { party.abbreviation=Some(abbreviation.to_string()) }
        if let Some(ticket_array) = group["TicketPreference"].as_array() {
            let mut ticket : Vec<CandidateIndex> = vec![CandidateIndex(usize::MAX);num_candidates];
            for candidate in ticket_array {
                let candidate_name = candidate["CandidateBallotPaperName"].as_str().ok_or_else(||anyhow!("Parsed file {} has a ticket element {} missing a field CandidateBallotPaperName",path.to_string_lossy(),candidate))?;
                let candidate_index = *candidates.get(candidate_name).ok_or_else(||anyhow!("Parsed file {} has a CandidateBallotPaperName {} I don't recognise",path.to_string_lossy(),candidate_name))?;
                let preference = candidate["Preference"].as_str().ok_or_else(||anyhow!("Parsed file {} has a ticket element {} missing a field Preference",path.to_string_lossy(),candidate))?;
                let preference : usize = preference.parse()?;
                ticket[preference-1]=candidate_index;
            }
            for preference in 0..num_candidates {
                if ticket[preference]==CandidateIndex(usize::MAX) {
                    return Err(anyhow!("Party {} ticket does not have a preference {}",party.name,preference+1));
                }
            }
            party.tickets=vec![ticket];
        }
    };
    Ok(())
}