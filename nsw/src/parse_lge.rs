// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Functions to parse NSW election data

use std::borrow::Cow;
use std::collections::{HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};
use stv::ballot_metadata::{Candidate, CandidateIndex, ElectionMetadata, ElectionName, NumberOfCandidates, Party, PartyIndex};
use stv::election_data::ElectionData;
use anyhow::{anyhow, Context};
use scraper::{ElementRef, Html, Selector};
use stv::ballot_paper::PreferencesComingOutOfOrderHelper;
use stv::parse_util::{file_to_string, FileFinder, KnowsAboutRawMarkings, MissingAlternateNamedFiles, MissingFile, RawDataSource, read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions};
use stv::tie_resolution::{TieResolutionAtom, TieResolutionExplicitDecision, TieResolutionsMadeByEC};
use serde::{Serialize,Deserialize};
use url::Url;
use stv::ballot_pile::BallotPaperCount;
use stv::datasource_description::{AssociatedRules, Copyright, ElectionDataSource};
use stv::distribution_of_preferences_transcript::{PerCandidate, QuotaInfo};
use stv::download::CacheDir;
use stv::official_dop_transcript::{OfficialDistributionOfPreferencesTranscript, OfficialDOPForOneCount};
use stv::parse_util::parse_xlsx_by_converting_to_csv_using_openoffice;
use crate::{NSWECLocalGov2021, SimpleIRVAnyDifferenceBreaksTies};
use crate::parse_lc::read_official_dop_transcript_html_index_page_then_one_html_page_per_count;


pub fn get_nsw_lge_data_loader_2021(finder:&FileFinder) -> anyhow::Result<NSWLGEDataLoader> {
    NSWLGEDataLoader::new(finder,"2021","https://pastvtr.elections.nsw.gov.au/LG2101/results","/LG2101")
}
pub fn get_nsw_lge_data_loader_2017(finder:&FileFinder) -> anyhow::Result<NSWLGEDataLoader> {
    NSWLGEDataLoader::new(finder,"2017","https://pastvtr.elections.nsw.gov.au/LGE2017/index.htm","/LGE2017")
}
pub fn get_nsw_lge_data_loader_2016(finder:&FileFinder) -> anyhow::Result<NSWLGEDataLoader> {
    NSWLGEDataLoader::new(finder,"2016","https://pastvtr.elections.nsw.gov.au/LGE2016/lge-index.htm","/LGE2016")
}
pub struct NSWLGEDataSource {}

impl ElectionDataSource for NSWLGEDataSource {
    fn name(&self) -> Cow<'static, str> { "NSW Local Government".into() }
    fn ec_name(&self) -> Cow<'static, str> { "NSW Electoral Commission".into() }
    fn ec_url(&self) -> Cow<'static, str> { "https://www.elections.nsw.gov.au/".into() }
    fn years(&self) -> Vec<String> { vec!["2016".to_string(),"2017".to_string(),"2021".to_string()] }
    fn get_loader_for_year(&self,year: &str,finder:&FileFinder) -> anyhow::Result<Box<dyn RawDataSource+Send+Sync>> {
        match year {
            "2021" => Ok(Box::new(get_nsw_lge_data_loader_2021(finder)?)),
            "2017" => Ok(Box::new(get_nsw_lge_data_loader_2017(finder)?)),
            "2016" => Ok(Box::new(get_nsw_lge_data_loader_2016(finder)?)),
            _ => Err(anyhow!("Not a valid year")),
        }
    }
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

impl KnowsAboutRawMarkings for NSWLGEDataLoader {}

impl RawDataSource for NSWLGEDataLoader {
    fn name(&self, electorate: &str) -> ElectionName {
        ElectionName {
            year: self.year.clone(),
            authority: "NSW Electoral Commission".to_string(),
            name: "NSW Local Government".to_string(),
            electorate: electorate.to_string(),
            modifications: vec![],
            comment: None,
        }
    }

    fn candidates_to_be_elected(&self, region: &str) -> NumberOfCandidates {
        self.find_contest(region).unwrap().vacancies
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
        self.contests.iter().map(|c| c.name.clone()).collect()
    }
    fn read_raw_data(&self, electorate: &str) -> anyhow::Result<ElectionData> { self.read_raw_data_possibly_rejecting_some_types(electorate, None) }

    fn read_raw_data_best_quality(&self, electorate: &str) -> anyhow::Result<ElectionData> {
        if electorate.ends_with("Mayoral") { read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<SimpleIRVAnyDifferenceBreaksTies,Self>(self, electorate) }
        else { read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<NSWECLocalGov2021,Self>(self,electorate) }
    }

    fn read_raw_metadata(&self, electorate: &str) -> anyhow::Result<ElectionMetadata> {
        let contest = &self.find_contest(electorate)?.url;
        let mayoral = electorate.ends_with(" Mayoral");
        let metadata_data_file = match self.year.as_str() {
            "2017" | "2016" => {
                self.find_raw_data_file_relative(contest,"","fp_by_grp_and_candidate_by_vote_type.htm","fp_by_grp_and_candidate_by_vote_type.htm")?
            }
            "2021" => {
                self.find_raw_data_file_relative(contest,
                                                 if mayoral { "mayoral/report" } else { "councillor/report" },
                                                 if mayoral { "mayoral-fp-by-candidate.html" } else { "fp-by-grp-and-candidate-by-vote-type.html" },
                                                 if mayoral { "mayoral/report/fp-by-grp-and-candidate-by-vote-type" } else { "councillor/report/fp-by-grp-and-candidate-by-vote-type" })?
            }
            _ => { return Err(anyhow!("Invalid year")); }
        };
        let mut metadata = self.parse_candidate_list(File::open(metadata_data_file)?, mayoral, electorate)?;
        // get winners
        let winner_info_file = match self.year.as_str() {
            "2016" => {
                self.find_raw_data_file_relative_may_have_alt_names(contest,"",&vec!["summary.htm","index.htm"])? // TODO could be index.htm
            }
            "2017" => {
                self.find_raw_data_file_relative(contest,"","summary.htm","summary.htm")?
            }
            "2021" => {
                self.find_raw_data_file_relative(contest,"",
                                                 if mayoral {"mayoral.html"} else {"councillor.html"},
                                                 if mayoral {"mayoral"} else {"councillor"})?
            }
            _ => { return Err(anyhow!("Invalid year")); }
        };
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
            let ineligible_info_file = match self.year.as_str() {
                "2017" | "2016" => {
                    self.find_raw_data_file_relative(contest,"","grp_and_candidates_result.htm","grp_and_candidates_result.htm")?
                }
                "2021" => {
                    self.find_raw_data_file_relative(contest,"councillor/report","grp-and-candidates-result.html","councillor/report/grp-and-candidates-result")?
                }
                _ => { return Err(anyhow!("Invalid year")); }
            };
            let ineligible = get_ineligible_candidates(&Html::parse_document(&std::fs::read_to_string(ineligible_info_file)?))?;
            metadata.excluded=ineligible;
        }
        if electorate=="Port Stephens - Central Ward" && self.year.as_str()=="2017" {
            metadata.tie_resolutions.tie_resolutions.push(TieResolutionAtom::ExplicitDecision(TieResolutionExplicitDecision{
                favoured: vec![CandidateIndex(1)],
                disfavoured: vec![CandidateIndex(2),CandidateIndex(13),CandidateIndex(14),CandidateIndex(15)],
                came_up_in: Some("2".to_string()),
            }))
        }
        Ok(metadata)
    }

    fn copyright(&self) -> Copyright {
        Copyright {
            statement: Some("© State of New South Wales through the NSW Electoral Commission".into()),
            url: Some("https://www.elections.nsw.gov.au/Copyright".into()),
            license_name: Some("Creative Commons Attribution 4.0 License".into()),
            license_url: Some("https://creativecommons.org/licenses/by/4.0/".into())
        }
    }

    fn rules(&self, electorate: &str) -> AssociatedRules {
        match self.year.as_str() {
            "2021" if electorate.ends_with("Mayoral") => AssociatedRules {
                rules_used: Some("IRV".into()),
                rules_recommended: Some("IRV".into()),
                comment: Some("This is not actually a STV election, having only one candidate. It is an IRV election. The legislation is ambiguous about tie resolution. The NSWEC transcripts frequently continue far beyond when the mayor is elected; this is harmless if bizarre and confusing and I do not bother emulating this bug.".into()),
                reports: vec!["https://github.com/AndrewConway/ConcreteSTV/blob/main/reports/NSWLGE2021Report.pdf".into()],
            },
            "2021" => AssociatedRules {
                rules_used: Some("NSWECLocalGov2021".into()),
                rules_recommended: None,
                comment: Some("The legislation is very ambiguous. My interpretation of the rules is NSWLocalGov2021 but NSWECLocalGov2021 seems a plausible interpretation.".into()),
                reports: vec!["https://github.com/AndrewConway/ConcreteSTV/blob/main/reports/NSWLGE2021Report.pdf".into()],
            },
            _ => AssociatedRules { rules_used: None, rules_recommended: None, comment: None, reports: vec![] },
        }
    }

    fn read_official_dop_transcript(&self, metadata: &ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        match self.year.as_str() {
            "2017" | "2016" => {
                let cache = CacheDir::new(self.finder.path.join("NSW/LGE".to_string()+&self.year));
                let contest = &self.find_contest(&metadata.name.electorate)?.url;
                let dop_url = Url::parse(&self.page_url)?.join(&(contest.to_string()+"/dop_index.htm"))?;
                read_official_dop_transcript_html_index_page_then_one_html_page_per_count(&cache,dop_url.as_str(),metadata)
            }
            "2021" => {
                if metadata.name.electorate.ends_with("Mayoral") { self.read_official_dop_transcript_mayoral(metadata) }
                else { self.read_official_dop_transcript_councillor(metadata).context("read_official_dop_transcript_councillor") }
            }
            _ => { return Err(anyhow!("Invalid year")); }
        }
    }
}

impl NSWLGEDataLoader {

    pub fn read_raw_data_possibly_rejecting_some_types(&self, electorate: &str, reject_vote_type : Option<HashSet<String>>) -> anyhow::Result<ElectionData> {
        let contest = &self.find_contest(electorate)?.url;
        let mayoral = electorate.ends_with(" Mayoral");
        let metadata = self.read_raw_metadata(electorate)?;
        let zip_name = if mayoral {"mayoral-finalpreferencedatafile.zip"} else {"finalpreferencedatafile.zip"};
        let zip_preferences_list = self.find_raw_data_file_relative(contest,if self.year=="2021"{"download"} else {""},zip_name,&((if self.year=="2021"{"download/"} else {""}).to_string()+zip_name))?;
        parse_zip_election_file(File::open(zip_preferences_list)?,metadata,reject_vote_type,mayoral)
    }

    pub fn new(finder:&FileFinder,year:&'static str,page_url:&'static str,archive_location_prefix:&str) -> anyhow::Result<Self> {

        let archive_location = "NSW/LGE".to_string()+year+"/pastvtr.elections.nsw.gov.au"+archive_location_prefix;
        let contest_list = match year {
            "2016" => include_str!("NSWLGE2016_contest_list.json"),
            "2017" => include_str!("NSWLGE2017_contest_list.json"),
            "2021" => include_str!("NSWLGE2021_contest_list.json"),
            _ => return Err(anyhow!("Illegal year {}",year)),
        };
        Ok(NSWLGEDataLoader {
            finder : finder.clone(),
            archive_location,
            year: year.to_string(),
            page_url: page_url.to_string(),
            contests : serde_json::from_str(contest_list)?,
        })
    }

    fn find_contest(&self,electorate:&str) -> anyhow::Result<&NSWLGEContest> {
        self.contests.iter().find(|c|c.name==electorate).ok_or_else(||anyhow!("Could not find electorate {}",electorate))
    }

    pub fn find_raw_data_file_relative(&self,contest:&str,relfolder:&str,filename:&str,url_relative:&str) -> Result<PathBuf,MissingFile> {
        let archive_location = self.archive_location.clone()+"/"+contest+(if relfolder.is_empty() {""} else {"/"})+relfolder;
        //println!("Archive location {}",archive_location);
        self.finder.find_raw_data_file_with_extra_url_info(filename,&archive_location,&self.page_url,&(contest.to_string()+"/"+url_relative))
    }
    pub fn find_raw_data_file_relative_may_have_alt_names(&self,contest:&str,relfolder:&str,alternate_names:&[&str]) -> Result<PathBuf,MissingAlternateNamedFiles> {
        let mut alternates = vec![];
        for name in alternate_names {
            match self.find_raw_data_file_relative(contest,relfolder,name,name) {
                Ok(path) => { return Ok(path); }
                Err(e) => { alternates.push(e); }
            }
        }
        Err(MissingAlternateNamedFiles{alternates})
    }
    /// parse a file like https://vtr.elections.nsw.gov.au/LG2101/albury/councillor/report/fp-by-grp-and-candidate-by-vote-type or https://vtr.elections.nsw.gov.au/LG2101/ballina/a-ward/councillor/report/fp-by-grp-and-candidate-by-vote-type
    /// or, for mayoral, like https://vtr.elections.nsw.gov.au/LG2101/ballina/mayoral/report/mayoral-fp-by-candidate
    fn parse_candidate_list(&self,mut file:File,mayoral:bool,electorate:&str) -> anyhow::Result<ElectionMetadata> {
        let html = Html::parse_document(&file_to_string(&mut file)?);
        // the title has hypens missing - see Ballina - A Ward
        // let electorate = html.select(&Selector::parse("head > title").unwrap()).flat_map(|e|e.text()).collect::<Vec<_>>().join("")+if mayoral {" Mayoral"} else {""};
        let electorate = electorate.to_string(); // +if mayoral {" Mayoral"} else {""};
        if electorate.is_empty() { return Err(anyhow!("Empty page title")); }
        let table = html.select(&Selector::parse("table").unwrap()).next().ok_or_else(||anyhow!("No table element"))?;
        let headings_in_tbody = match self.year.as_str() {
            "2017" | "2016" => true,
            "2021" => false,
            _ => return Err(anyhow!("Illegal Year"))
        };
        let thead_first = table.select(&Selector::parse(if headings_in_tbody {"tbody > tr > th"} else {"thead > tr > th"}).unwrap()).next().ok_or_else(||anyhow!("No table headings"))?;
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
        let notes : Vec<(String,String)> = html.select(&Selector::parse("div.note, p.note_p").unwrap()).map(|e|parse_note(e)).flatten().collect::<Vec<_>>();
        let get_note = |note_name:&str| notes.iter().find(|(name,_)|name.starts_with(note_name)).map(|(_,v)|v.clone()).ok_or_else(||anyhow!("Could not find note {}",note_name));
        let vacancies = NumberOfCandidates(if mayoral {1} else {get_note("Candidates to be Elected:")?.chars().filter(|&c|c!=',').collect::<String>().parse()?});
        let enrolment = NumberOfCandidates(get_note(if mayoral {"Electors Enrolled as on"} else {"Enrolment:"})?.chars().filter(|&c|c!=',').collect::<String>().parse()?);
        let select_rows = Selector::parse("tbody tr").unwrap();
        let mut rows = table.select(&select_rows);
        let rows = if headings_in_tbody { rows.next(); rows } else { rows }; // skip first row.
        for row in rows {
            let mut cols = row.select(&selector_td);
            let col1 = cols.next().ok_or_else(||anyhow!("No first column"))?;
            let col2 = cols.next().ok_or_else(||anyhow!("No second column"))?;
            let col1_text = col1.text().map(|s|s.trim()).collect::<Vec<_>>().join("");
            let col2_text = col2.text().map(|s|s.trim()).collect::<Vec<_>>().join("");
            if let Some(_row_class) = row.value().attr("class") {
                if /* row_class == "tr-total" &&*/ has_groups && (col2_text == "UNGROUPED CANDIDATES" || !col1_text.is_empty()) { // a party!
                    parties.push(Party {
                        column_id: (if col1_text.is_empty() { "UG" } else { &col1_text }).to_string(),
                        name: col2_text,
                        abbreviation: None,
                        atl_allowed: !col1_text.is_empty(),
                        candidates: vec![],
                        tickets: vec![],
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

    pub fn read_official_dop_transcript_councillor(&self,metadata:&ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        let contest = &self.find_contest(&metadata.name.electorate)?.url;
        let dop_file = self.find_raw_data_file_relative(contest,"download","dopfulldetails.xlsx","download/dopfulldetails.xlsx")?;
        let table = parse_xlsx_by_converting_to_csv_using_openoffice(&dop_file).context("parsing dopfulldetails.xlsx")?; // do it this way as calamine does not read this file properly.
        if table.len() < 7 { return Err(anyhow!("DoP spreadsheet is too short"))}
        let expected_number_columns = 11+4*metadata.candidates.len()+14;
        if table[0].len() != expected_number_columns { return Err(anyhow!("DoP spreadsheet has {} columns expecting {}",table[0].len(),expected_number_columns)) }
        //println!("Read into a table with {} rows and {} columns",table.len(),table[0].len());
        //for line in &table { println!("{:?}",line); }
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
            papers: BallotPaperCount(remove_comma(&table[1][2]).parse::<usize>()?-remove_comma(&table[7][col_exhausted_bps]).parse::<usize>()?),
            vacancies: NumberOfCandidates(remove_comma(&table[2][2]).parse::<usize>()?),
            quota: remove_comma(&table[3][2]).parse::<f64>()?
        });
        assert_eq!(col_result+1,expected_number_columns);
        let mut counts = vec![];
        let mut about_to_be_excluded : Option<CandidateIndex> = None;
        for line_upto in 6..table.len() {
            let line = &table[line_upto];
            if line.len()!=expected_number_columns { return Err(anyhow!("DoP spreadsheet has {} columns on line {} expecting {}",line.len(),line_upto+1,expected_number_columns)) }
            let excluded_candidate_0_ballots = line[col_description].trim()=="Total" && about_to_be_excluded.is_some();
            if (line[col_description].is_empty() && !line[col_count].is_empty() && line[col_count].chars().all(|c|c=='.'||c.is_ascii_digit())) || excluded_candidate_0_ballots { // line is an actual count!
                let transfer_value = if excluded_candidate_0_ballots { None} else { Some(if line[col_ctv].is_empty() {1.0} else {remove_comma(&line[col_ctv]).parse::<f64>()?})};
                let mut vote_delta = PerCandidate::default();
                let mut paper_delta = PerCandidate::default();
                for c in metadata.candidate_indices() {
                    vote_delta.candidate.push(parse_number_blank_as_nan(&line[col_candidate_papers(c)+1]));
                    paper_delta.candidate.push(parse_number_blank_as_zero(&line[col_candidate_papers(c)]));
                }
                vote_delta.exhausted=parse_number_blank_as_nan(&line[col_votes_set_aside]);
                paper_delta.exhausted=parse_number_blank_as_zero(&line[col_exhausted_bps]);
                //vote_delta.rounding=parse_number_blank_as_NaN(&line[col_votes_set_aside]).into();// they track a different way. I compute the number of votes represented as a delta for the appropriate candidate, and round down. They don't round down (good, but messy), and don't display (bad, but avoids mess). Their rounding thus takes this into account.  It would be nice to be able to track this.
                vote_delta.rounding=f64::NAN.into();
                counts.push(OfficialDOPForOneCount{
                    transfer_value,
                    elected: vec![], // will be set later.
                    excluded: about_to_be_excluded.take().into_iter().collect(),
                    vote_total: None,
                    paper_total: None,
                    vote_delta: Some(vote_delta),
                    paper_delta: Some(paper_delta),
                    paper_set_aside: None,
                    count_name: Some(line[col_count].clone()),
                    papers_came_from_counts : None,
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

    const FIX_NSWEC_MAYORAL_TRANSCRIPT_BUG_AT_TRANSCRIPT: bool = true;

    pub fn read_official_dop_transcript_mayoral(&self,metadata:&ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        let contest = &self.find_contest(&metadata.name.electorate)?.url;
        let dop_file = self.find_raw_data_file_relative(contest,"mayoral/report","mayoral-dop","mayoral/report/mayoral-dop")?;
        let html = Html::parse_document(&std::fs::read_to_string(dop_file)?);
        let selector_td = Selector::parse("td").unwrap();
        let mut candidates_in_order_of_exclusion : Vec<CandidateIndex> = vec![];
        let mut candidates_tallys : Vec<Vec<usize>> = vec![];
        let mut exhausted = vec![];
        let mut elected_candidate : Option<CandidateIndex> = None;
        //let mut elected_round : Option<usize> = None;
        fn get_candidate_from_name_and_suffix(name:&str,metadata:&ElectionMetadata) -> CandidateIndex {
            for i in 0..metadata.candidates.len() {
                if name.starts_with(&metadata.candidates[i].name) { return CandidateIndex(i); }
            }
            panic!("Could not find candidate {}",name);
        }
        fn parse_number_with_comma(s:&str) -> usize {
            if s.is_empty() || s=="EXCLUDED" || s=="-" {0}
            else {s.replace(',',"").parse().unwrap()}
        }
        for row in html.select(&Selector::parse("table tbody tr").unwrap()) {
            let cols = row.select(&selector_td);
            let mut cols : Vec<String> = cols.map(|c|c.text().map(|s|s.trim()).collect::<Vec<_>>().join("")).collect();
            if cols[0]=="Candidates" {
            } else if cols[0].is_empty() { // list of excluded candidates
                candidates_in_order_of_exclusion = cols[2..cols.len()-1].iter().map(|s|get_candidate_from_name_and_suffix(s,metadata)).collect::<Vec<_>>();
            } else { // everything else is mostly a set of numbers, possibly with "ELECTED" added. Extract it.
                let elected_position = cols.len()-2;
                let elected = if cols[elected_position].starts_with("ELECTED") {
                    cols[elected_position] = cols[elected_position].trim_start_matches("ELECTED").trim().to_string();
                    // deal with a bug in NSWEC count, don't stop counting just because a candidate was elected on first round, see https://pastvtr.elections.nsw.gov.au/LG2101/bellingen/mayoral/mayoral-dop
                    // sometimes to be really special they continue for more than just one extra round : https://pastvtr.elections.nsw.gov.au/LG2101/burwood/mayoral/mayoral-dop
                    // Or Rina Mercuri shows off a bug again, the first time it occurs when not elected on first round, https://pastvtr.elections.nsw.gov.au/LG2101/griffith/mayoral/mayoral-dop
                    for i in 1..cols.len() {
                        if cols[i].starts_with("ELECTED") {
                            cols[i] = cols[i].trim_start_matches("ELECTED").trim().to_string();
                            // elected_round=Some((i-1)/2);
                            if Self::FIX_NSWEC_MAYORAL_TRANSCRIPT_BUG_AT_TRANSCRIPT {
                                // In this case we will make sure all future rounds are ignored. So we will fix the bug at the transcript rather than emulating the bug.
                                candidates_in_order_of_exclusion.truncate((i-1)/2);
                            }
                        }
                    }
                    true
                } else { false };

                let counts = cols[1..cols.len()-1].iter().map(|s|parse_number_with_comma(s)).collect::<Vec<_>>();
                if cols[0]=="Total Votes in Count" { // do nothing
                } else if cols[0]=="Exhausted Votes" {
                    exhausted=counts;
                } else if cols[0]=="Informal" {// do nothing
                } else if cols[0]=="Total Votes / Ballot Papers" {// do nothing
                } else if cols[0]=="Absolute Majority" { // do nothing
                } else {
                    let candidate = get_candidate_from_name_and_suffix(&cols[0],metadata);
                    if candidate.0 == candidates_tallys.len() {
                        candidates_tallys.push(counts);
                        if elected { elected_candidate=Some(candidate); }
                    } else { panic!("Candidates out of order")}
                }
            }
        }
        let mut counts:Vec<OfficialDOPForOneCount> = vec![];
        let column = |col:usize|Some(PerCandidate{
            candidate: candidates_tallys.iter().map(|v|v[col]).collect(),
            exhausted: exhausted[col],
            rounding: 0.into(),
            set_aside: None
        });
        let columnf64 = |col:usize|Some(PerCandidate{
            candidate: candidates_tallys.iter().map(|v|v[col] as f64).collect(),
            exhausted: exhausted[col] as f64,
            rounding: 0.0.into(),
            set_aside: None
        });
        let columnisize = |col:usize|Some(PerCandidate{
            candidate: candidates_tallys.iter().map(|v|v[col] as isize).collect(),
            exhausted: exhausted[col] as isize,
            rounding: 0.into(),
            set_aside: None
        });
        // do first prefs.
        let mut add_col= |col_delta:usize,col_total:usize,excluded:Option<CandidateIndex>| {
            let mut count = OfficialDOPForOneCount{
                transfer_value: None,
                elected: vec![],
                excluded: excluded.into_iter().collect(),
                vote_total: columnf64(col_total),
                paper_total: column(col_total),
                vote_delta: columnf64(col_delta),
                paper_delta: columnisize(col_delta),
                paper_set_aside: None,
                count_name: None,
                papers_came_from_counts : None,
            };
            if let Some(candidate) = excluded {
                let old_total = counts.last().unwrap().paper_total.as_ref().unwrap().candidate[candidate.0];
                count.vote_delta.as_mut().unwrap().candidate[candidate.0] = - (old_total as f64);
                count.paper_delta.as_mut().unwrap().candidate[candidate.0] = - (old_total as isize);
            }
            counts.push(count);
        };
        add_col(0,0,None);
        for i in 0..candidates_in_order_of_exclusion.len() {
            add_col(i*2+1,i*2+2,Some(candidates_in_order_of_exclusion[i]));
        }
        counts.last_mut().unwrap().elected.push(elected_candidate.unwrap());
        Ok(OfficialDistributionOfPreferencesTranscript{
            quota : None,
            counts,
            missing_negatives_in_papers_delta: false,
            elected_candidates_are_in_order: true,
            all_exhausted_go_to_rounding : false,
        })
    }

}

fn parse_number_blank_as_nan(s:&str) -> f64 {
    let s = s.trim();
    if s.is_empty() { f64::NAN} else { remove_comma(s).parse().unwrap() }
}
fn parse_number_blank_as_zero(s:&str) -> isize {
    let s = s.trim();
    if s.is_empty() { 0 } else { remove_comma(s).parse().unwrap() }
}
fn remove_comma(s:&str) -> String { s.chars().filter(|c|*c!=',').collect() }

/// Parse the zipped election file.
/// The function currently ignores non-formal markings.
/// Optionally, some particular vote types can be suppressed.
pub(crate) fn parse_zip_election_file(zipfile : File, metadata:ElectionMetadata, reject_vote_type : Option<HashSet<String>>, mayoral: bool) -> anyhow::Result<ElectionData> {
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
            let vote_type = &record[vote_type_column];
            helper.set_vote_type(vote_type);
            if let Some(restrictions) = reject_vote_type.as_ref() {
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
    /// The relative url to a distribution of preferences zip file, if present. Used in 2021.
    pub dop : Option<String>,
    /// The relative url to a DoP index file, if present. Generally used prior to 2021.
    pub dop_index : Option<String>,
    /// The relative url to the table of votes by candidate, which can be used to get the names and groups of candidates.
    pub candidates_page : Option<String>,
    /// The relative url to the mayoral dop page, which can be used to find ineligible candidates.
    pub mayoral_dop : Option<String>,
    /// The relative url to the candidate results page, which can be used to find ineligible candidates.
    pub candidate_results : Option<String>,
    /// Sometimes the links are on a different page called "final results".
    pub final_results_page : Option<String>,
}

impl NSWLGESingleContestMainPageInfo {
    fn extract_file<P:AsRef<Path>>(path:P) -> anyhow::Result<Self> {
        Self::extract_html(&Html::parse_document(&std::fs::read_to_string(path)?))
    }
    /// Sometimes this is in the main page (2021), sometimes in the final_results_page file (2017)
    pub fn add_relative_files(&mut self,html:&Html) {
        for e in html.select(&Selector::parse("a").unwrap()) {
            if let Some(href) = e.value().attr("href") {
                if href.ends_with("dopfulldetails.xlsx") { self.dop=Some(href.to_string()); }
                if href.contains("dop_index") { self.dop_index=Some(href.to_string()); }
                if href.ends_with("finalpreferencedatafile.zip") { self.preferences=Some(href.to_string()); }
                if href.ends_with("11 - Details Preference for Count.zip") { self.preferences=Some(href.to_string()); }
                if href.ends_with("fp-by-grp-and-candidate-by-vote-type") || href.ends_with("fp_by_grp_and_candidate_by_vote_type.htm") { self.candidates_page=Some(href.to_string()); }
                if href.ends_with("mayoral-fp-by-candidate") { self.candidates_page=Some(href.to_string()); }
                if href.ends_with("mayoral-dop") { self.mayoral_dop=Some(href.to_string()); }
                if href.ends_with("grp-and-candidates-result") || href.ends_with("grp_and_candidates_result.htm") { self.candidate_results=Some(href.to_string()); }
                if href.contains("final-results") { self.final_results_page=Some(href.to_string()); }
            }
        }
    }
    /// Extract required information from a file like https://vtr.elections.nsw.gov.au/LG2101/albury/councillor
    pub fn extract_html(html: &Html) -> anyhow::Result<Self> {
        let name = html.select(&Selector::parse("h1").unwrap()).last().ok_or_else(||anyhow!("No h1 element"))?.text().collect::<Vec<_>>().join("");
        let mut res = NSWLGESingleContestMainPageInfo{
            to_be_elected: Some(usize::MAX),
            elected_candidates: vec![],
            name,
            preferences: None,
            dop: None,
            dop_index: None,
            candidates_page: None,
            mayoral_dop: None,
            candidate_results: None,
            final_results_page: None,
        };
        // get relative files.
        res.add_relative_files(html);
        // get number of councillors. Look for string like  `There are 9 Councillors to be elected from 51 candidates.` or `This election was UNCONTESTED, and there was no need to vote.`
        for e in html.root_element().text() {
            let e = e.trim();
            if let Some(rest) = e.strip_prefix("There") {
                let rest=rest.trim();
                if rest.starts_with("is") || rest.starts_with("are") || rest.starts_with("was") || rest.starts_with("were") {
                    let num : String = rest.trim_start_matches("is").trim_start_matches("are").trim_start_matches("was").trim_start_matches("were").trim_start().chars().take_while(|&c|c.is_ascii_digit()).collect();
                    res.to_be_elected=Some(num.parse()?);
                }
            }
            //println!("    {}",e);
            if e.contains("This election was UNCONTESTED")||e.contains("The election was uncontested") { res.to_be_elected=None; }
        }
        if res.to_be_elected==Some(usize::MAX) { return Err(anyhow!("Could not find the number of people to be elected."))}
        // find elected councillors
        for e in html.select(&Selector::parse("span.candidate-name, span.candidate_name").unwrap()) {
            let name = e.inner_html();
            let name = if name.ends_with(')') {
                if let Some(ind) = name.find('(') { name[..ind].to_string() }
                else { name }
            } else { name };
            res.elected_candidates.push(name);
        }
        if let Some(to_be_elected) = res.to_be_elected {
            if res.elected_candidates.len()!=to_be_elected {
                // there is an error in the 2016 shire of Bourke page, "There were 0 Councillors to be elected from 13 candidates." A similar error in Campbelltown City Council 5 instead of 15 means they probably just enter the last digit.
                if res.elected_candidates.len()%10==to_be_elected { res.to_be_elected=Some(res.elected_candidates.len()); }
                else if res.name=="Leeton Shire Council" && to_be_elected==9 {} // There are only 8 candidates
                else {
                    return Err(anyhow!("Wrong number of candidates elected."));
                }
            }
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
    let thead_first = table.select(&Selector::parse("thead > tr > th, tbody > tr > th").unwrap()).next().ok_or_else(||anyhow!("No table headings"))?;
    let has_groups = thead_first.inner_html()=="Group";
    let selector_td = Selector::parse("td").unwrap();
    let mut candidate_index = 0;
    for row in table.select(&Selector::parse("tbody tr").unwrap()) {
        let col = row.select(&selector_td).map(|e|inner_text(&e)).collect::<Vec<_>>();
        if col.is_empty() { continue; } // first row of headings
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