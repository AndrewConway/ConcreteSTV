// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Functions to parse NSW election data for the legislative council.

use std::borrow::Cow;
use std::collections::{HashSet};
use std::fs::File;
use std::path::PathBuf;
use stv::ballot_metadata::{Candidate, CandidateIndex, DataSource, ElectionMetadata, ElectionName, NumberOfCandidates, Party, PartyIndex};
use stv::election_data::ElectionData;
use anyhow::{anyhow, Context};
use scraper::{ElementRef, Html, Selector};
use url::Url;
use stv::parse_util::{FileFinder, KnowsAboutRawMarkings, MissingFile, RawDataSource};
use stv::tie_resolution::{TieResolutionAtom, TieResolutionExplicitDecision, TieResolutionExplicitDecisionInCount, TieResolutionsMadeByEC};
use stv::ballot_pile::BallotPaperCount;
use stv::datasource_description::{AssociatedRules, Copyright, ElectionDataSource};
use stv::distribution_of_preferences_transcript::{CountIndex, PerCandidate, QuotaInfo};
use stv::download::CacheDir;
use stv::official_dop_transcript::{OfficialDistributionOfPreferencesTranscript, OfficialDOPForOneCount};
use crate::parse_lge::parse_zip_election_file;


// 2011 data files are at http://www.pastvtr.elections.nsw.gov.au/SGE2011/lc_prefdata.htm
pub fn get_nsw_lc_data_loader_2015(finder:&FileFinder) -> anyhow::Result<NSWLCDataLoader> {
    NSWLCDataLoader::new(finder,"2015","https://pastvtr.elections.nsw.gov.au/SGE2015/lc-home.htm","https://pastvtr.elections.nsw.gov.au/SGE2015/lc/state/dop/dop_index/index.htm","https://pastvtr.elections.nsw.gov.au/SGE2015/data/lc/") // https://pastvtr.elections.nsw.gov.au/SGE2015/lc/state/preferences/index.htm is a description of the preferences files.
}

pub fn get_nsw_lc_data_loader_2019(finder:&FileFinder) -> anyhow::Result<NSWLCDataLoader> {
    NSWLCDataLoader::new(finder,"2019","https://pastvtr.elections.nsw.gov.au/SG1901/LC/results","https://pastvtr.elections.nsw.gov.au/SG1901/LC/state/dop/index","https://pastvtr.elections.nsw.gov.au/SG1901/LC/state/preferences")
}

pub fn get_nsw_lc_data_loader_2023(finder:&FileFinder) -> anyhow::Result<NSWLCDataLoader> {
    NSWLCDataLoader::new(finder,"2023","https://vtr.elections.nsw.gov.au/SG2301/LC/results","https://vtr.elections.nsw.gov.au/SG2301/LC/state/dop/index","https://vtr.elections.nsw.gov.au/SG2301/LC/state/preferences")
}





pub struct NSWLCDataSource {}

impl ElectionDataSource for NSWLCDataSource {
    fn name(&self) -> Cow<'static, str> { "NSW Legislative Council".into() }
    fn ec_name(&self) -> Cow<'static, str> { "NSW Electoral Commission".into() }
    fn ec_url(&self) -> Cow<'static, str> { "https://www.elections.nsw.gov.au/".into() }
    fn years(&self) -> Vec<String> { vec!["2015".to_string(),"2019".to_string(),"2023".to_string()] }
    fn get_loader_for_year(&self,year: &str,finder:&FileFinder) -> anyhow::Result<Box<dyn RawDataSource+Send+Sync>> {
        match year {
            "2015" => Ok(Box::new(get_nsw_lc_data_loader_2015(finder)?)),
            "2019" => Ok(Box::new(get_nsw_lc_data_loader_2019(finder)?)),
            "2023" => Ok(Box::new(get_nsw_lc_data_loader_2023(finder)?)),
            _ => Err(anyhow!("Not a valid year")),
        }
    }
}

pub struct NSWLCDataLoader {
    finder : FileFinder,
    archive_location : String,
    year : String,
    page_url : String,
    dop_url : String,
    pref_url : Url,
}



impl KnowsAboutRawMarkings for NSWLCDataLoader {}
/// Get all the text in an element as a string
fn text_content(e:&ElementRef<'_>) -> String {
    e.text().collect::<Vec<_>>().join("")
}
/// Get all the text in multiple elements as a vec of strings
fn text_contents<'a>(iter: impl IntoIterator<Item=ElementRef<'a>>) -> Vec<String> {
    iter.into_iter().map(|e|text_content(&e)).collect::<Vec<_>>()
}
fn parse_with_commas(s:&str) -> anyhow::Result<usize> {
    if s=="INELIGIBLE" { Ok(0) }
    else { s.trim().replace(',',"").parse().with_context(||format!("Error trying to parse {}",s)) }
}

impl RawDataSource for NSWLCDataLoader {
    fn name(&self, electorate: &str) -> ElectionName {
        ElectionName {
            year: self.year.clone(),
            authority: "NSW Electoral Commission".to_string(),
            name: "NSW Legislative Council".to_string(),
            electorate: electorate.to_string(),
            modifications: vec![],
            comment: None,
        }
    }

    fn candidates_to_be_elected(&self, _region: &str) -> NumberOfCandidates {
        NumberOfCandidates(21)
    }

    /// There is not much point including these as there is so much other randomness
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
    fn all_electorates(&self) -> Vec<String> { vec!["Whole State".to_string()] }

    fn read_raw_data(&self, electorate: &str) -> anyhow::Result<ElectionData> { self.read_raw_data_possibly_rejecting_some_types(electorate, None) }

    /// Currently don't try to predict EC choices as there is not much point.
    fn read_raw_data_best_quality(&self, electorate: &str) -> anyhow::Result<ElectionData> { self.read_raw_data(electorate) }

    fn read_raw_metadata(&self, electorate: &str) -> anyhow::Result<ElectionMetadata> {
        let mut metadata = self.read_raw_metadata_from_excel_file(electorate)?;
        if self.year=="2023" {
            metadata.tie_resolutions.tie_resolutions.push(TieResolutionAtom::ExplicitDecision(TieResolutionExplicitDecisionInCount {
                decision: TieResolutionExplicitDecision::two_lists(vec![CandidateIndex(195)],vec![CandidateIndex(34)]),
                came_up_in: Some(CountIndex(3)),
            }));
        }
        Ok(metadata)
            /* Alternate method using HTML files
                let cache = self.cache();
                let base_url = url::Url::parse(&self.dop_url)?;
                let fp_by_grp_and_candidate_by_vote_type = base_url.join("fp_by_grp_and_candidate_by_vote_type")?;
                let file = cache.get_file(fp_by_grp_and_candidate_by_vote_type.as_str())?;
                let metadata = parse_fp_by_grp_and_candidate_by_vote_type(file,false,Some(NumberOfCandidates(21)),false,self.name(electorate))?;*/
    }

    fn copyright(&self) -> Copyright {
        Copyright {
            statement: Some("© State of New South Wales through the NSW Electoral Commission".into()),
            url: Some("https://www.elections.nsw.gov.au/Copyright".into()),
            license_name: Some("Creative Commons Attribution 4.0 License".into()),
            license_url: Some("https://creativecommons.org/licenses/by/4.0/".into())
        }
    }

    fn rules(&self, _electorate: &str) -> AssociatedRules {
        AssociatedRules {
            rules_used: match self.year.as_str() {
                "2015" => Some("NSWECRandomLC2015".into()),
                "2019" | "2023" => Some("NSWECRandomLC2019".into()),
                _ => None,
            },
            rules_recommended: Some("NSWECRandomLC2019".into()),
            comment: Some("The legislation is poorly written and involves significant randomness - a repeat count may produce a different outcome.".into()),
            reports: vec!["https://github.com/AndrewConway/ConcreteSTV/blob/main/reports/NSWLGE2012CountErrorTechReport.pdf".into(),"https://github.com/AndrewConway/ConcreteSTV/blob/main/reports/2016%20NSW%20LGE%20Errors.pdf".into()],
        }
    }

    fn read_official_dop_transcript(&self, metadata: &ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        read_official_dop_transcript_html_index_page_then_one_html_page_per_count(&self.cache(),&self.dop_url,metadata)
    }

}

/// Read the "old style" DoP used for the probabilistic algorithm (LC and LGE pre 2021)
pub fn read_official_dop_transcript_html_index_page_then_one_html_page_per_count(cache:&CacheDir,dop_url:&str, metadata: &ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
    // first parse the main DOP.
    let overview_html = Html::parse_document(&cache.get_string(dop_url)?);
    let mut counts: Vec<OfficialDOPForOneCount> = vec![];
    let base_url = url::Url::parse(dop_url)?;
    let select_td = Selector::parse("td").unwrap();
    let select_th = Selector::parse("th").unwrap();
    let select_a = Selector::parse("a").unwrap();
    let select_notep = Selector::parse("p.note_p, div.note").unwrap();
    let select_list_rows = Selector::parse("table tr").unwrap(); // Before 2023, it was "table.list tr". 2023 LC version could be "div#prccReport div.prcc-report div.prcc-data table tr". 2015 LC version could be "div#prcc-report table.list tr".
    let regex_find_transfer_value = regex::Regex::new(r"^\d+ / \( ?\d+ - \d+\) = ([\d\.]+)$").unwrap();
    let candidate_name_lookup = metadata.get_candidate_name_lookup();
    let mut total_formal_votes : Option<usize> = None;
    for overview_tr in overview_html.select(&Selector::parse("div#ReportContent table.list tr, div#prccReport table.list tr, div.prcc-data table tr").unwrap()) {
        let tds : Vec<ElementRef> = overview_tr.select(&select_td).collect();
        if tds.len()==4 {
            if let Some(a) = tds[0].select(&select_a).next() {
                if let Some(href) = a.value().attr("href") {
                    let resolved = base_url.join(href)?;
                    //println!(" {}",resolved);
                    let td_to_candidate_list = |td:&ElementRef| -> anyhow::Result<Vec<CandidateIndex>> {
                        let mut res = vec![];
                        for s in td.text() {
                            let s = s.trim();
                            if s.len()>0 && s!="*" {
                                if let Some(candidate) = candidate_name_lookup.get(s) { res.push(*candidate); }
                                else {return Err(anyhow!("Unknown candidate {}",s))}
                            }
                        }
                        Ok(res)
                    };
                    let elected = td_to_candidate_list(&tds[1])?;
                    let _candidate_distributed = td_to_candidate_list(&tds[2])?;
                    let excluded = td_to_candidate_list(&tds[3])?;
                    let count_html = Html::parse_document(&cache.get_string(&resolved.to_string())?);
                    let mut candidate_col : Option<usize> = None;
                    let mut progressive_total_col : Option<usize> = None;
                    let mut transferred_col : Option<usize> = None;
                    let mut set_aside_col : Option<usize> = None;
                    let mut votes_col : Option<usize> = None;
                    let mut expected_len = 0;
                    let mut paper_total : PerCandidate<usize> = PerCandidate::from_num_candidates(metadata.candidates.len(),usize::MAX);
                    let mut paper_delta : PerCandidate<isize> = PerCandidate::from_num_candidates(metadata.candidates.len(),isize::MAX);
                    let mut paper_set_aside : PerCandidate<usize> = PerCandidate::from_num_candidates(metadata.candidates.len(),0);
                    let mut first_prefs_atl : Option<usize> = None;
                    let mut groups_col : Option<usize> = None;
                    let transfer_value : Option<f64> = {
                        text_contents(count_html.select(&select_notep)).iter().filter_map(|s|{
                            regex_find_transfer_value.captures(s.trim_end_matches(" (Note: the Transfer Value cannot be greater than 1)")).and_then(|c|c[1].parse::<f64>().ok())
                        }).next()
                    };
                    // if let Some(transfer_value) = transfer_value { println!("Found tv {}",transfer_value); }
                    for tr in count_html.select(&select_list_rows) {
                        let is_headings_row = match tr.value().attr("class") {
                            Some("t_head") | Some("tr-title") => true,
                            _ => false,
                        };
                        if is_headings_row {
                            let headings = tr.select(&select_th).map(|e|e.text().next().unwrap_or("")).collect::<Vec<_>>();
                            // println!("Found headings {:?}",headings);
                            if headings.len()>0 && (headings[0]=="Group" || headings[0]=="Candidates in Ballot Order") {
                                expected_len=headings.len();
                                groups_col=headings.iter().position(|s|*s=="Group");
                                candidate_col=headings.iter().position(|s|*s=="Candidates in Ballot Order");
                                progressive_total_col=headings.iter().position(|s|*s=="Progressive Total"); // exclusion or surplus
                                transferred_col=headings.iter().position(|s|*s=="Ballot Papers Transferred"); // exclusion or surplus
                                set_aside_col=headings.iter().position(|s|*s=="Set Aside for Quota"); // surplus
                                votes_col=headings.iter().position(|s|*s=="Votes").or(headings.iter().position(|s|*s=="First Preferences")); // first preferences count. Sometimes there are both columns, when someone is ineligible. In this case we want the votes column.
                                // println!("Found votes_col {:?} candidate_col {:?} groups_col {:?}",votes_col,candidate_col,groups_col);
                            } else { candidate_col=None }
                        } else if let Some(candidate_col) = candidate_col {
                            let tds : Vec<String> = text_contents(tr.select(&select_td));
                            if tds.len()==expected_len && groups_col.map(|c|!tds[c].is_empty()).unwrap_or(false)  { // This is a group line. For first preferences, above the line is included here,
                                if let Some(votes_col) = votes_col {
                                    let s = tds[votes_col].trim();
                                    if !s.is_empty() {
                                        first_prefs_atl = Some(parse_with_commas(s)?);
                                    }
                                }
                            } else if tds.len()==expected_len {
                                match tds[candidate_col].trim() {
                                    "Group Total" => {}
                                    "UNGROUPED CANDIDATES" => {}
                                    "Exhausted" => { // comes up twice! So don't give errors if empty values
                                        if let Some(col) = set_aside_col.or(transferred_col) {
                                            let votes = tds[col].trim();
                                            if !votes.is_empty() {
                                                let votes = parse_with_commas(votes)?;
                                                if set_aside_col.is_some() {
                                                    paper_set_aside.exhausted = votes;
                                                    if transfer_value==Some(1.0) {
                                                        // Note that in 2016, Ballina Shire Council - A Ward, count 3, there is a TV of 208/(213-125) which means 125 were exhausted, of which 5 counted as set aside for quota, and 120 as exhausted. But all 125 were in the "Set Aside for Quota" column.
                                                        // the exhausted set aside values are not accurate if the transfer value is 1.0.
                                                        // It could be better to correct the buggy NSWEC data as below, but this is currently handled by a special case is the compare official to computed code.
                                                        // paper_set_aside.exhausted = usize::MAX;
                                                    }
                                                }
                                                else { paper_delta.exhausted = votes as isize; }
                                            }
                                        }
                                        if let Some(col) = votes_col.or(progressive_total_col) {
                                            let votes = tds[col].trim();
                                            if !votes.is_empty() {
                                                let votes = parse_with_commas(votes)?;
                                                paper_total.exhausted=votes;
                                                if votes_col.is_some() { paper_delta.exhausted = votes as isize; }
                                            }
                                        }
                                    }
                                    "Formal Votes" => {
                                        if let Some(votes_col) = votes_col { // first preferences
                                            total_formal_votes = Some(parse_with_commas(&tds[votes_col])?);
                                        }
                                    }
                                    "Set Aside (previous counts)" => {}
                                    "Informal Ballot Papers" => {}
                                    "Total Votes / Ballot Papers" => {}
                                    "Brought Forward" => {// exhausted
                                        if let Some(progressive_total_col) = progressive_total_col {
                                            paper_total.exhausted+=parse_with_commas(&tds[progressive_total_col])?;
                                        }
                                    }
                                    "Set Aside this Count" => {// exhausted
                                        if let Some(progressive_total_col) = progressive_total_col {
                                            let num = parse_with_commas(&tds[progressive_total_col])?;
                                            paper_total.exhausted+=num;
                                            paper_delta.exhausted=num as isize;
                                            // paper_set_aside.exhausted-=num; // double counted in the "Exhausted" column above here.
                                        }
                                    }
                                    "TOTAL" => {}
                                    name => {
                                        if let Some(&candidate) = candidate_name_lookup.get(name) {
                                            // println!("Found candidate {} tds {:?}",candidate,tds);
                                            if let Some(votes_col) = votes_col { // first preferences
                                                let votes = parse_with_commas(&tds[votes_col])?+first_prefs_atl.take().unwrap_or(0); // add in ATL group votes if present.
                                                paper_total.candidate[candidate.0] = votes;
                                                paper_delta.candidate[candidate.0] = votes as isize;
                                            } else if let Some(progressive_total_col) = progressive_total_col {
                                                paper_total.candidate[candidate.0] = if "EXCLUDED"==tds[progressive_total_col] {0} else { parse_with_commas(&tds[progressive_total_col])?};
                                                if let Some(transferred_col) = transferred_col {
                                                    paper_delta.candidate[candidate.0] = parse_with_commas(&tds[transferred_col]).unwrap_or(isize::MAX as usize) as isize;
                                                    if let Some(set_aside_col) = set_aside_col {
                                                        paper_set_aside.candidate[candidate.0] = parse_with_commas(&tds[set_aside_col]).unwrap_or(0);
                                                    }
                                                } else { return Err(anyhow!("No transferred_col")); }
                                            } else {
                                                println!("I don't know how to get anything from the row for candidate {}",candidate)
                                            }
                                            //println!("Found candidate {}",candidate);
                                        } else { return Err(anyhow!("Could not interpret (in DoP for single count) candidate name {}",name))}
                                    }
                                }
                            }
                        }
                    }
                    // println!("Found ballot papers {:?}",paper_total);
                    let exhausted_round_1 = if counts.is_empty() {0} else if let Some(pt) = &counts[0].paper_total { pt.exhausted} else {0};
                    // exhausted votes in round 1 are not kept in the official tally of cumulative exhausted votes. ConcreteSTV does. It is not clear what the right thing to
                    // do is; at least the ConcreteSTV choice has the property that the sum of ballot papers in each round is the same (formal votes). The NSWEC has the advantage of having
                    // the total sum be the same as the value used in the quota computation for all rounds other than the first.
                    // The line below converts the NSWEC choice to the ConcreteSTV choice.
                    paper_total.exhausted+=exhausted_round_1;
                    let vote_total : PerCandidate<f64> = paper_total.clone().into();
                    let vote_delta : PerCandidate<f64> = paper_delta.clone().into();
                    counts.push(OfficialDOPForOneCount{
                        transfer_value,
                        elected,
                        excluded,
                        vote_total: Some(vote_total),
                        paper_total: Some(paper_total),
                        vote_delta: Some(vote_delta),
                        paper_delta: Some(paper_delta),
                        paper_set_aside_for_quota: Some(paper_set_aside),
                        count_name: None,
                        papers_came_from_counts: None,
                    })
                }
            }
        }
    }
    // println!("Total formal votes = {:?}",total_formal_votes);
    let quota = if let Some(total_formal_votes) = total_formal_votes {
        let mut vacancies : Option<NumberOfCandidates> = None;
        let mut quota : Option<f64> = None;
        for p in overview_html.select(&select_notep) {
            let text = p.text().collect::<Vec<_>>();
            // println!("Found text {:?}",text);
            if text.len()==2 {
                if text[0].to_lowercase()=="candidates to be elected" { vacancies=Some(NumberOfCandidates(text[1].trim_start_matches(':').trim().parse()?))}
                if text[0].to_lowercase()=="quota" { quota=Some(text[1].trim_start_matches(':').trim().replace(',',"").parse()?)}
            }
            //println!("{:?}",text);
        }
        //println!("quota = {:?} vacancies = {:?}",quota,vacancies);
        if let Some(quota) = quota {
            if let Some(vacancies) = vacancies {
                Some(QuotaInfo{
                    papers : BallotPaperCount(total_formal_votes),
                    vacancies,
                    quota,
                })
            } else { None }
        } else { None }
    } else {None} ;
    Ok(OfficialDistributionOfPreferencesTranscript{
        quota,
        counts,
        missing_negatives_in_papers_delta: true,
        elected_candidates_are_in_order: false,
        all_exhausted_go_to_rounding : false,
        negative_values_in_surplus_distributions_and_rounding_may_be_off: false,
    })
}


impl NSWLCDataLoader {

    /// Name of the file containing detailed preferences
    fn pref_data_filename(&self) -> String {
        match self.year.as_str() {
            "2023" => "SG2301 LC Pref Data Statewide.zip".to_string(),
            _ => format!("SGE{} LC Pref Data Statewide.zip",&self.year),
        }
    }
    /// Name of the file containing candidates.
    fn candidate_list_excel_filename(&self) -> String {
        match self.year.as_str() {
            "2015" => "SGE2015 LC Candidates v1.xlsx".to_string(),
            _ => format!("SGE{} LC Candidates.xlsx",&self.year),
        }
    }

    pub fn read_raw_data_possibly_rejecting_some_types(&self, electorate: &str, reject_vote_type : Option<HashSet<String>>) -> anyhow::Result<ElectionData> {
        let mut metadata = self.read_raw_metadata(electorate)?;
        let zip_preferences_list = self.find_raw_data_file_from_cache(self.pref_url.join(&self.pref_data_filename())?.as_str())?;
        // let zip_preferences_list = self.finder.find_raw_data_file(&self.pref_data_filename(),&self.archive_location,&self.pref_url)?; // old location
        metadata.source.push(DataSource{
            url: self.pref_url.to_string(),
            files: vec![self.pref_data_filename()],
            comments: None,
        });
        parse_zip_election_file(File::open(zip_preferences_list)?,metadata,reject_vote_type,false)
    }

    pub fn new(finder:&FileFinder,year:&str,page_url:&str,dop_url:&str,pref_url:&str) -> anyhow::Result<Self> {
        let archive_location = "NSW/State".to_string()+year+"/"; // The 2101 should not be hardcoded.
        Ok(NSWLCDataLoader {
            finder : finder.clone(),
            archive_location,
            year: year.to_string(),
            page_url: page_url.to_string(),
            dop_url: dop_url.to_string(),
            pref_url: Url::parse(pref_url)?,
        })
    }

    /// Get a CacheDir structure suitable for finding a file specified by a url.
    fn cache(&self) -> CacheDir {
        CacheDir::new(self.finder.path.join(&self.archive_location))
    }

    fn find_raw_data_file_from_cache(&self, url:&str) -> Result<PathBuf, MissingFile> {
        self.cache().find_raw_data_file_from_cache(url)
    }
/*
    fn read_cached_url_as_string(&self,url:&str) -> anyhow::Result<String> {
        self.cache().get_string(url)
    }
*/
    fn read_raw_metadata_from_excel_file(&self, electorate: &str) -> anyhow::Result<ElectionMetadata> {
        let metadata_path = self.find_raw_data_file_from_cache(self.pref_url.join(&self.candidate_list_excel_filename())?.as_str())?;
            // self.finder.find_raw_data_file(excel_file_name,&self.archive_location,&self.pref_url)?; // old location
        let metadata = self.parse_candidate_list(&metadata_path,electorate)?;
        Ok(metadata)
    }

    /// parse a xlsx file looking like
    /// ```txt
    /// In Grp Seq.	GVS/Candidate	Group	Group/Candidates in Ballot Order
    /// 1	Candidate	A	JONES Peter
    /// 2	Candidate	A	CARBONE Pat
    /// 3	Candidate	A	MACRI Gus
    /// 1	Candidate	B	WHELAN Peter
    /// 2	Candidate	B	ELLIS Mark
    /// 3	Candidate	B	WHELAN James
    /// ...
    /// 13	Candidate	X	CIURPITA Roman
    /// 14	Candidate	X	HICKSON Barbara
    /// 15	Candidate	X	HARKER-SMITH Angus
    /// 1	Candidate	UG	NUTHALL Ramsay
    /// 2	Candidate	UG	WARD Jane
    /// 3	Candidate	UG	HOOD Alan
    /// ```
    /// to get the metadata for the election. The second tab contrains groups.
    fn parse_candidate_list(&self,path:&PathBuf,electorate:&str) -> anyhow::Result<ElectionMetadata> {
        use calamine::Reader;
        let mut workbook1 = calamine::open_workbook_auto(&path)?;
        // load candidates
        let sheet1 = workbook1.worksheet_range_at(0).ok_or_else(||anyhow!("No sheets in {}",path.to_string_lossy()))??;
        let mut candidates = vec![];
        let mut parties = vec![];
        let mut current_group = "NA".to_string();
        let mut current_position = 0;
        for row in 1..(sheet1.height() as u32) {
            let group = sheet1.get_value((row,2)).ok_or_else(||anyhow!("Missing group id"))?.get_string().ok_or_else(||anyhow!("Group id is not a string"))?;
            let name = sheet1.get_value((row,3)).ok_or_else(||anyhow!("Missing candidate name"))?.get_string().ok_or_else(||anyhow!("Candidate name is not a string"))?;
            if group!=&current_group {
                parties.push(Party {
                    column_id: group.to_string(),
                    name: group.to_string(),
                    abbreviation: None,
                    atl_allowed: group!="UG",
                    candidates: vec![],
                    tickets: vec![],
                });
                current_position = 0;
                current_group = group.to_string();
            }
            current_position+=1;
            if let Some(party) = parties.last_mut() { party.candidates.push(CandidateIndex(candidates.len()))}
            candidates.push(Candidate{
                name : name.to_string(),
                party: if parties.is_empty() { None } else { Some(PartyIndex(parties.len()-1))},
                position: Some(current_position),
                ec_id: None
            });
        }
        // load groups, second tab.
        let sheet2 = workbook1.worksheet_range_at(1).ok_or_else(||anyhow!("No group sheet in {}",path.to_string_lossy()))??;
        for row in 1..(sheet2.height() as u32) {
            let group = sheet2.get_value((row,1)).ok_or_else(||anyhow!("Missing group name"))?.get_string().ok_or_else(||anyhow!("Group name is not a string"))?;
            let group_name = sheet2.get_value((row,2)).and_then(|s|s.get_string()).and_then(|s|if s.is_empty() {None} else {Some(s)});
            let gvs = sheet2.get_value((row,0)).ok_or_else(||anyhow!("Missing gvs"))?.get_string().ok_or_else(||anyhow!("gvs is not a string"))?;
            let group = parties.iter_mut().find(|p|group==&p.column_id).ok_or_else(||anyhow!("Cannot find group {}",group))?;
            if let Some(group_name) = group_name {
                group.name=group_name.to_string();
            }
            group.atl_allowed = gvs=="Yes";
        }
        Ok(ElectionMetadata{
            name: self.name(&electorate),
            candidates,
            parties,
            source: vec![DataSource{
                url: self.pref_url.to_string(),
                files: path.file_name().map(|p|p.to_string_lossy().to_string()).into_iter().collect(),
                comments: None,
            }],
            results: None,
            vacancies: Some(self.candidates_to_be_elected(electorate)),
            enrolment: None,
            secondary_vacancies: None,
            excluded: vec![],
            tie_resolutions: Default::default()
        })
    }


}

