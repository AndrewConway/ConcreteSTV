// Copyright 2022-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.



use std::borrow::Cow;
use std::collections::HashSet;
use std::path::PathBuf;
use anyhow::{anyhow, Context};
use stv::ballot_metadata::{Candidate, CandidateIndex, DataSource, ElectionMetadata, ElectionName, NumberOfCandidates, Party, PartyIndex};
use stv::datasource_description::{AssociatedRules, Copyright, ElectionDataSource};
use stv::election_data::ElectionData;
use stv::official_dop_transcript::{OfficialDistributionOfPreferencesTranscript, OfficialDOPForOneCount};
use stv::parse_util::{CalamineLikeWrapper, FileFinder, KnowsAboutRawMarkings, MissingFile, RawDataSource, read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions};
use stv::tie_resolution::TieResolutionsMadeByEC;
use crate::Vic2018LegislativeCouncil;
use calamine::{DataType, open_workbook_auto};
use stv::ballot_paper::{ATL, BTL, parse_marking, RawBallotMarking, RawBallotMarkings, UniqueBTLBuilder};
use stv::ballot_pile::BallotPaperCount;
use stv::distribution_of_preferences_transcript::{CountIndex, PerCandidate, QuotaInfo};
use stv::signed_version::SignedVersion;

pub fn get_vic_data_loader_2014(finder:&FileFinder) -> anyhow::Result<VicDataLoader> {
    VicDataLoader::new(finder,"2014","https://www.vec.vic.gov.au/results/state-election-results/2014-state-election") // Note - not all needed files are public. Ask the VEC nicely if you want them, and maybe they will oblige.
}

pub fn get_vic_data_loader_2018(finder:&FileFinder) -> anyhow::Result<VicDataLoader> {
    VicDataLoader::new(finder,"2018","https://www.vec.vic.gov.au/results/state-election-results/2018-state-election") // Note - not all needed files are public. Ask the VEC nicely if you want them, and maybe they will oblige.
}

pub fn get_vic_data_loader_2022(finder:&FileFinder) -> anyhow::Result<VicDataLoader> {
    VicDataLoader::new(finder,"2022","https://www.vec.vic.gov.au/results/state-election-results/2022-state-election") // Note - not all needed files are public. Ask the VEC nicely if you want them, and maybe they will oblige.
}


/// Do not use on website as votes are not published.
pub struct VicDataSource {}

impl ElectionDataSource for VicDataSource {
    fn name(&self) -> Cow<'static, str> { "Victorian Upper House".into() }
    fn ec_name(&self) -> Cow<'static, str> { "Victorian Electoral Commission".into() }
    fn ec_url(&self) -> Cow<'static, str> { "https://www.vec.vic.gov.au/".into() }
    fn years(&self) -> Vec<String> { vec!["2014".to_string(),"2018".to_string(),"2022".to_string()] }
    fn get_loader_for_year(&self,year: &str,finder:&FileFinder) -> anyhow::Result<Box<dyn RawDataSource+Send+Sync>> {
        match year {
            "2014" => Ok(Box::new(get_vic_data_loader_2014(finder)?)),
            "2018" => Ok(Box::new(get_vic_data_loader_2018(finder)?)),
            "2022" => Ok(Box::new(get_vic_data_loader_2022(finder)?)),
            _ => Err(anyhow!("Not a valid year")),
        }
    }
}

pub struct VicDataLoader {
    finder : FileFinder,
    archive_location : String,
    year : String,
    page_url : String,
}

impl KnowsAboutRawMarkings for VicDataLoader {}

impl RawDataSource for VicDataLoader {
    fn name(&self, electorate: &str) -> ElectionName {
        ElectionName {
            year: self.year.clone(),
            authority: "Victorian Electoral Commission".to_string(),
            name: "Victorian Upper House".to_string(),
            electorate: electorate.to_string(),
            modifications: vec![],
            comment: None,
        }
    }

    fn candidates_to_be_elected(&self, _region: &str) -> NumberOfCandidates {
        NumberOfCandidates(5)
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
        let year_as_num : u32 = self.year.chars().filter(|c|c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0);
        let east_met_name = if year_as_num<2022 { "Eastern Metropolitan Region" } else { "North-Eastern Metropolitan Region" };
        let mut res = vec![
            east_met_name.to_string(),
            "Eastern Victoria Region".to_string(),
            "Northern Metropolitan Region".to_string(),
            "Northern Victoria Region".to_string(),
            "South-Eastern Metropolitan Region".to_string(),
            "Southern Metropolitan Region".to_string(),
            "Western Metropolitan Region".to_string(),
            "Western Victoria Region".to_string(),
        ];
        res.sort();
        res
    }
    fn read_raw_data(&self, electorate: &str) -> anyhow::Result<ElectionData> {
        let (mut metadata,atl_votes) = self.read_raw_metadata_and_atl_votes(electorate)?;
        // println!("{:?}",metadata);
        let filename = match self.year.as_str() {
            "2014" => format!("received_from_ec/Ballot Paper Details - {}.csv",electorate),
            "2022" => format!("received_from_ec/BallotPaperDetails-{}.csv",electorate.trim_end_matches(" Region").trim_end_matches("politan")),
            _ => {return Err(anyhow!("Do not know the file naming convention for votes received in {}",self.year))}
        };
        let path = self.find_raw_data_file(&filename)?;
        let mut reader = csv::ReaderBuilder::new().flexible(true).has_headers(false).from_path(&path)?;
        let mut builder = UniqueBTLBuilder::default();
        let mut informal : usize = 0;
        let min_btl_prefs_needed = 1; // Not really used - only formal votes provided.
        for result in reader.records() {
            let record = result?;
            if let Some(ballot_index_in_batch) = record.get(0) {
                if ballot_index_in_batch.len()>0 && ballot_index_in_batch.chars().all(|c|c.is_ascii_digit()) && record.len()==1+metadata.candidates.len() { // check it is not metadata.
                    let mut btl_markings = vec![RawBallotMarking::Blank;metadata.candidates.len()];
                    for i in 0..metadata.candidates.len() {
                        btl_markings[i]=parse_marking(&record[1+i]);
                    }
                    if let Some(btl) = (RawBallotMarkings{ atl: &[], btl: &btl_markings, atl_parties: &[] }).interpret_vote_as_btl(min_btl_prefs_needed) {
                        /* Code below used to debug an error in the VEC transcript for North Eastern Region, 2022, where on count 169 a paper was strangely transferred to DOLAN, Hugh.
                        if let Some(p1) = btl.candidates.iter().position(|c|c.0==32) {
                            if let Some(p2) = btl.candidates.iter().position(|c|c.0==25) {
                                if p1<p2 {
                                    let s = format!("{:?}",btl.candidates);
                                    if s=="[#31, #32, #33, #34, #25]" || s=="[#38, #31, #32, #25, #11]" || s=="[#32, #25, #38, #44, #51]" || s=="[#32, #31, #35, #14, #25]" || s=="[#32, #25, #33, #30, #45]"
                                        || s=="[#32, #34, #35, #31, #33, #25, #26, #50, #51, #54, #55]" || s=="[#32, #30, #53, #55, #25]" || s=="[#34, #32, #33, #31, #35, #25, #26]" || s=="[#35, #31, #32, #33, #34, #25, #26]"
                                        || s=="[#35, #31, #32, #33, #34, #25, #3, #13]" || s=="[#35, #31, #32, #34, #33, #25, #26, #14, #13]" || s=="[#32, #31, #35, #33, #34, #36, #37, #25, #26]" || s=="[#31, #32, #33, #34, #35, #19, #20, #25, #26, #50, #51]" || s=="[#31, #33, #32, #34, #35, #13, #14, #25, #26]" {
                                        println!("Record {:?}, result {:?}",record,btl.candidates);
                                    }
                                }
                            }
                        }*/
                        builder.add_vote(btl);
                    } else { informal+=1;}
                }
            }
        }
        metadata.source.push(DataSource{
            url: "VEC does not publish unfortunately".to_string(),
            files: vec![filename],
            comments: None
        });
        let mut result = ElectionData{
            metadata,
            atl : vec![],
            atl_types: vec![],
            btl : builder.to_btls(),
            btl_types: vec![],
            informal
        };
        if atl_votes.len()>0 { // the ATL votes are converted to BTL already. Deduce them (and tickets) from the BTL via the (ugly, unreliable) method of assuming the largest number of full length BTL votes starting with a given candidate is it.
            find_atl_votes_in_btl(&mut result,&atl_votes)?;
        }
        if self.election_count_not_published_yet() { // in this case the file above is only partial results. Add an estimate of the remaining results from the count by first preference.
            let early_preference_votes = self.parse_results_by_region_intermediate_webpage(&mut result.metadata)?;
            let mut votes_by_candidate_first_pref = vec![0;result.metadata.candidates.len()]; // the number of votes we have BTL candidates for.
            for b in &result.btl {
                votes_by_candidate_first_pref[b.candidates[0].0]+=b.n;
            }
            // Deal with all votes for parties as if they are ATL votes for all votes not already in btl.
            for party_index in 0..result.metadata.parties.len().min(early_preference_votes.by_party.len()) {
                let (num_atl,num_btl) = early_preference_votes.by_party[party_index];
                let mut unaccounted = num_atl+num_btl;
                for &c in &result.metadata.parties[party_index].candidates {
                    unaccounted-=votes_by_candidate_first_pref[c.0];
                    votes_by_candidate_first_pref[c.0]=0; // accounted for
                }
                result.atl.push(ATL{
                    parties: vec![PartyIndex(party_index)],
                    n: unaccounted,
                    ticket_index: Some(0)
                });
            }
            // Deal with votes for ungrouped candidates, as just a first pref vote for them.
            for candidate_index in 0..result.metadata.candidates.len() {
                let n = early_preference_votes.by_candidate[candidate_index];
                if n>votes_by_candidate_first_pref[candidate_index] {
                    result.btl.push(BTL{ candidates: vec![CandidateIndex(candidate_index)], n:n-votes_by_candidate_first_pref[candidate_index] })
                }
            }
        }
        Ok(result)
    }

    fn read_raw_data_best_quality(&self, electorate: &str) -> anyhow::Result<ElectionData> {
        read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<Vic2018LegislativeCouncil,Self>(self, electorate)
    }

    /// Get the metadata from the file like south-easternmetropolitanregionvotesreceived.xls
    fn read_raw_metadata(&self,electorate:&str) -> anyhow::Result<ElectionMetadata> {
        let (metadata,_) = self.read_raw_metadata_and_atl_votes(electorate)?;
        Ok(metadata)
    }

    fn copyright(&self) -> Copyright {
        Copyright {
            statement: Some("© Victorian Electoral Commission".into()),
            url: Some("https://www.vec.vic.gov.au/legal".into()),
            license_name: Some("Creative Commons Attribution 4.0 international license".into()),
            license_url: Some("https://creativecommons.org/licenses/by/4.0/".into())
        }
    }

    fn rules(&self, _electorate: &str) -> AssociatedRules {
        match self.year.as_str() {
            "2014" => AssociatedRules {
                rules_used: Some("VEC2018".into()),
                rules_recommended: None,
                comment: Some("Untested. 2018 rules used rather than literal legislation which included what could reasonably be considered a typo fixed in 2018.".into()),
                reports: vec![],
            },
            "2022" => AssociatedRules {
                rules_used: Some("VEC2018".into()),
                rules_recommended: None,
                comment: Some("Untested.".into()),
                reports: vec![],
            },
            _ => AssociatedRules { rules_used: None, rules_recommended: None, comment: None, reports: vec![] },
        }
    }

    fn read_official_dop_transcript(&self, metadata: &ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        let filename = self.distribution_of_preferences_filename(&metadata.name.electorate)?;
        let path = self.find_raw_data_file(&filename)?;
        use calamine::Reader;
        let mut workbook1 = open_workbook_auto(&path)?;
        let sheet1 = workbook1.worksheet_range_at(0).ok_or_else(||anyhow!("No sheets in {}",path.to_string_lossy()))??;
        let format = DOPFileFormat::new(&sheet1,metadata).context(path.to_string_lossy().to_string())?;
        format.parse_spreadsheet(&sheet1,metadata).context(path.to_string_lossy().to_string())
    }
}

/// Deduce ATL votes and tickets from the BTL votes given the number of ATL votes, given that the ATL votes are already turned into BTL votes.
///
/// This is in general not solvable, of course, but since most people vote ATL it will work almost always by taking
/// the largest number of full ticket votes. It is unfortunate that we have to use this messy unreliable algorithm.
///
/// If there are multiple tickets, it is even less reliably, particularly with working out the rounding on the tickets... it is impossible
/// to distinguish this if multiple people vote BTL according to the ticket. However the effect on statistics is tiny and it will not modify the
/// election transcript other than slight changes to the ATL/BTL number assignments.
///
/// We don't actually require the first candidate to have the ticket first preference, example Palmer United Party, 2014, South Eastern Metropolitan Region.
fn find_atl_votes_in_btl(result:&mut ElectionData,atl_votes:&[BallotPaperCount]) -> anyhow::Result<()> {
    let mut highest_by_candidate : Vec<(usize,Option<usize>)> = vec![(0,None);result.metadata.candidates.len()]; // first value is the largest number of identical votes starting with the given candidate and full length, second is the index into btl that has it
    for btl_index in 0..result.btl.len() {
        let btl = &result.btl[btl_index];
        if let Some(&c) = btl.candidates.first() {
            if btl.n>highest_by_candidate[c.0].0 && btl.candidates.len()==result.metadata.candidates.len() {
                highest_by_candidate[c.0] = (btl.n,Some(btl_index));
            }
        }
    }
    for party_index in 0..atl_votes.len() {
        let wanted = atl_votes[party_index];
        if wanted.0>0 {
            if result.metadata.parties[party_index].candidates.is_empty() { return Err(anyhow!("No candidates for party {}",party_index)) }
            // find the candidate in the party with the most first pref votes.
            let (available,index) = result.metadata.parties[party_index].candidates.iter().map(|c|highest_by_candidate[c.0]).max_by_key(|(n,_)|*n).unwrap();
            // let (available,index) = highest_by_candidate[result.metadata.parties[party_index].candidates.first().ok_or_else(||anyhow!("No candidates for party {}",party_index))?.0];
            if available<wanted.0 { // Not enough. This is not necessarily a show stopper - there may be multiple tickets!
                if available>1 {
                    let tickets_needed = ((wanted.0 as f64)/(available as f64)).round() as usize;
                    if tickets_needed<5 { // sanity check
                        println!("Trying {} tickets for party {} ({}), needing {} but best that could be found was {}",tickets_needed,party_index,result.metadata.parties[party_index].column_id,wanted,available);
                        let mut btl_index = 0;
                        let needed_given_perfect_rounding = wanted.0/tickets_needed;
                        let mut rounding_needed = wanted.0-tickets_needed*needed_given_perfect_rounding;
                        'next_ticket : for ticket_number in 0..tickets_needed {
                            let needed = needed_given_perfect_rounding+if rounding_needed==tickets_needed-ticket_number {1} else {0};
                            while btl_index<result.btl.len() {
                                let btl = &result.btl[btl_index];
                                if btl.candidates.len()==result.metadata.candidates.len() && btl.candidates[0]==result.metadata.parties[party_index].candidates[0] && btl.n>=needed {
                                    let used_rounding = if rounding_needed>0 && btl.n>needed_given_perfect_rounding {1} else {0};
                                    rounding_needed-=used_rounding;
                                    let used = needed_given_perfect_rounding+used_rounding;
                                    result.btl[btl_index].n-=used;
                                    result.atl.push(ATL{
                                        parties: vec![PartyIndex(party_index)],
                                        n: used,
                                        ticket_index: Some(ticket_number)
                                    });
                                    let ticket = result.btl[btl_index].candidates.clone();
                                    result.metadata.parties[party_index].tickets.push(ticket);
                                    btl_index+=1;
                                    continue 'next_ticket;
                                }
                                btl_index+=1;
                            }
                            return Err(anyhow!("Could only find {} of {} tickets for party {} ({}), needing {} but best that could be found was {}",ticket_number,tickets_needed,party_index,result.metadata.parties[party_index].column_id,wanted,available));
                        }
                        continue; // found multiple tickets!
                    }
                }/*
                println!("Metadata : {:#?}",result.metadata);
                for c in &result.metadata.parties[party_index].candidates {
                    println!("Candidate {} largest number of votes {:?}",result.metadata.candidates[c.0].name,highest_by_candidate[c.0]);
                }*/
                return Err(anyhow!("Could not find enough ATL votes using heuristics for party {} ({}), needing {} but best that could be found starting with {} was {}",party_index,result.metadata.parties[party_index].column_id,wanted,result.metadata.candidates[result.metadata.parties[party_index].candidates[0].0].name,available));
            }
            let index = index.ok_or_else(||anyhow!("Internal error - got votes>0 with no source"))?;
            result.btl[index].n-=wanted.0;
            result.atl.push(ATL{
                parties: vec![PartyIndex(party_index)],
                n: wanted.0,
                ticket_index: Some(0)
            });
            let ticket = result.btl[index].candidates.clone();
            result.metadata.parties[party_index].tickets.push(ticket);
        }
    }
    result.btl.retain(|v|v.n>0); // get rid of BTL entries if all converted to ATL.
    Ok(())
}
impl VicDataLoader {
    /// Get the metadata from the file like south-easternmetropolitanregionvotesreceived.xls
    fn read_raw_metadata_and_atl_votes(&self,electorate:&str) -> anyhow::Result<(ElectionMetadata,Vec<BallotPaperCount>)> {
        if self.election_count_not_published_yet() {
            let mut metadata = self.read_raw_metadata_from_candidate_list_on_main_website(electorate)?;
            self.parse_pdf_tickets_file(&mut metadata)?;
            return Ok((metadata,vec![])) // could be more sophisticated here and actually produce meaningful results.
        }
        let filename = match self.year.as_str() {
            "2014" => format!("{}votesreceived.xls",Self::region_human_name_to_computer_name(electorate,false)),
            "2018" => format!("{}votesreceived.xls",Self::region_human_name_to_computer_name(electorate,false)),
            "2022" => format!("{}-Votes Received.xls",electorate),
            _ => {return Err(anyhow!("Do not know the file naming convention for votes received in {}",self.year))}
        };
        let path = self.find_raw_data_file(&filename)?;
        // calamine seems not to work for this file
        let sheet1 = CalamineLikeWrapper::open(&path)?;
        //use calamine::Reader;
        //let mut workbook1 = open_workbook_auto(&path)?;
        //let sheet1 = workbook1.worksheet_range_at(0).ok_or_else(||anyhow!("No sheets in {}",path.to_string_lossy()))??;
        // find the row of headings. Probably 7...
        let is_headings_row = |row:usize|{
            if let Some(v) = sheet1.get_value((row as u32,0)) {
                if let Some(s) = v.get_string() {
                    s.ends_with(" District")
                } else {false}
            } else {false}
        };
        let headings_row = (0..sheet1.height()).into_iter().find(|&row|is_headings_row(row)).ok_or_else(||anyhow!("Could not find headlines in {}",path.to_string_lossy()))?;
        let is_region_total_row = |row:usize|{
            if let Some(v) = sheet1.get_value((row as u32,0)) {
                if let Some(s) = v.get_string() {
                    s.trim().to_uppercase()=="REGION TOTAL"
                } else {false}
            } else {false}
        };
        let region_total_row = (0..sheet1.height()).into_iter().find(|&row|is_region_total_row(row)).ok_or_else(||anyhow!("Could not find region total in {}",path.to_string_lossy()))?;
        let mut candidates = vec![];
        let mut parties = vec![];
        let mut current_party_name = "".to_string();
        let mut position_in_party = 0;
        let mut atl_votes : Vec<BallotPaperCount> = vec![];
        for col in 1..sheet1.width() {
            if let Some(v) = sheet1.get_value((headings_row as u32,col as u32)) {
                if let Some(s) = v.get_string() {
                    let lines : Vec<&str> = s.split('\n').collect();
                    if lines.len()==3 || (lines.len()==2 && lines[0].starts_with("Group ")) {
                        let last_line = lines[lines.len()-1];
                        if last_line=="ATL" {
                            if let Some(atl_value) = sheet1.get_value((region_total_row as u32,col as u32)) {
                                if let Some(s) = atl_value.get_string() {
                                    atl_votes.push(BallotPaperCount(s.parse()?));
                                }
                            }
                        }
                        if last_line!="ATL" && last_line!="TOTAL" {
                            let party_line = lines[lines.len()-2];
                            let had_atl = atl_votes.len()==parties.len()+1;
                            if parties.is_empty() || current_party_name!=party_line {
                                current_party_name=party_line.to_string();
                                position_in_party=1;
                                parties.push(Party{
                                    column_id: lines[0].to_string(),
                                    name: if lines.len()==3 { party_line.to_string() } else {"".to_string()},
                                    abbreviation: None,
                                    atl_allowed: had_atl,
                                    candidates: vec![],
                                    tickets: vec![]
                                });
                                if !had_atl {
                                    if atl_votes.len()+1!=parties.len() {
                                        return Err(anyhow!("Could not find ATL votes for party {} in {}",lines[0],filename));
                                    }
                                    atl_votes.push(BallotPaperCount(0));
                                }
                            }
                            parties.last_mut().unwrap().candidates.push(CandidateIndex(candidates.len()));
                            candidates.push(Candidate{
                                name: last_line.to_string(),
                                party: Some(PartyIndex(parties.len()-1)),
                                position: Some(position_in_party),
                                ec_id: None
                            });
                            position_in_party+=1;
                        }
                    } else if lines.len()==2 && lines[0]=="Ungrouped" { // Just to make life exciting, the ungrouped candidates appear to be listed in random order, not ballot paper order! Wheee!
                        candidates.push(Candidate{
                            name: lines[1].to_string(),
                            party: None,
                            position: None,
                            ec_id: None
                        });
                    }
                }
            }
        }
        let mut metadata = ElectionMetadata{
            name: self.name(electorate),
            candidates,
            parties,
            source: vec![DataSource{
                url: self.page_url.to_string(),
                files: vec![path.file_name().unwrap().to_string_lossy().to_string()],
                comments: None
            }],
            results: None,
            vacancies: Some(self.candidates_to_be_elected(electorate)),
            enrolment: None,
            secondary_vacancies: None,
            excluded: vec![],
            tie_resolutions: Default::default()
        };
        self.reorder_candidates_in_metadata_by_official_dop_transcript(&mut metadata)?;
        Ok((metadata,atl_votes))
    }

    fn election_count_not_published_yet(&self) -> bool { false }

    /// Get the metadata from a url like https://www.vec.vic.gov.au/electoral-boundaries/state-regions/north-eastern-metropolitan-region/nominations
    /// First will need to get file with something like
    /// `wget -S -O northern-metropolitan-region.nominations.html https://www.vec.vic.gov.au/electoral-boundaries/state-regions/northern-metropolitan-region/nominations`
    /// except the VEC blocks wget so you have to manually download it with a more conventional browser.
    ///
    fn read_raw_metadata_from_candidate_list_on_main_website(&self, electorate: &str) -> anyhow::Result<ElectionMetadata> {
        let filename = format!("{}.nominations.html", Self::region_human_name_to_computer_name(electorate,true));
        let url = format!("https://www.vec.vic.gov.au/electoral-boundaries/state-regions/{}/nominations",Self::region_human_name_to_computer_name(electorate,true));
        let path = self.finder.find_raw_data_file_with_extra_url_info(&filename, &self.archive_location, &url,"")?;
        let html = scraper::Html::parse_document(&std::fs::read_to_string(&path)?);
        let table = html.select(&scraper::Selector::parse("main table > tbody").unwrap()).next().ok_or_else(||anyhow!("Could not find main table in candidate list html page"))?;
        let mut candidates = vec![];
        let mut parties = vec![];
        let mut position_in_party = 0;
        for tr in table.select(&scraper::Selector::parse("tr").unwrap()) {
            if let Some(party_name) = tr.select(&scraper::Selector::parse("td.party-name a").unwrap()).next() {
                let party_name = party_name.text().collect::<Vec<_>>().join("").trim().to_string();
                //println!("Party name : {}",party_name);
                let (column_id,name) = party_name.split_once(' ').unwrap_or(("",&party_name));
                parties.push(Party{
                    column_id: column_id.trim_end_matches(".").to_string(),
                    name: name.to_string(),
                    abbreviation: None,
                    atl_allowed: name!="Ungrouped",
                    candidates: vec![],
                    tickets: vec![]
                });
                position_in_party=1;
            } else if let Some(candidate_name) = tr.select(&scraper::Selector::parse("tr.candidate-row td").unwrap()).next() {
                let candidate_name = candidate_name.text().next().ok_or_else(||anyhow!("No candidate name found in html file"))?.trim();
                //println!("Candidate name : {:?}",candidate_name);
                parties.last_mut().unwrap().candidates.push(CandidateIndex(candidates.len()));
                candidates.push(Candidate{
                    name: candidate_name.to_string(),
                    party: Some(PartyIndex(parties.len()-1)),
                    position: Some(position_in_party),
                    ec_id: None
                });
                position_in_party+=1;
            }
        }
        let metadata = ElectionMetadata{
            name: self.name(electorate),
            candidates,
            parties,
            source: vec![DataSource{
                url,
                files: vec![path.file_name().as_ref().unwrap().to_string_lossy().to_string()],
                comments: None
            }],
            results: None,
            vacancies: Some(self.candidates_to_be_elected(electorate)),
            enrolment: None,
            secondary_vacancies: None,
            excluded: vec![],
            tie_resolutions: Default::default()
        };
        Ok(metadata)
    }

    /// Parse the PDF file (!) containing party tickets, adding the tickets to the metadata reference.
    /// This is not very reliable
    fn parse_pdf_tickets_file(&self,metadata:&mut ElectionMetadata) -> anyhow::Result<()> {
        let filename = format!("State Election {}-{}-Group Voting Tickets.pdf",metadata.name.year,metadata.name.electorate);
        let path = self.finder.find_raw_data_file(&filename, &self.archive_location, "https://www.vec.vic.gov.au/candidates-and-parties/become-a-state-election-candidate/groups-and-voting-tickets")?;
        let pdf = pdf::file::File::open(path)?;
        let mut current_font : Option<String> = None; // the font currently active
        let font_of_preference_numbers = Some("T1_1".to_string());
        let font_of_address1 = Some("T1_2".to_string());
        let font_of_address2 = Some("T1_3".to_string());
        let font_of_surname = Some("T1_0".to_string());
        let font_of_firstname = None;
        let mut current_preference_number : Option<usize> = None;
        let mut current_candidate_name : Option<String> = None;
        let mut current_group_ticket : Option<PartyIndex> = None;
        let mut current_ticket_contents : Vec<Option<CandidateIndex>> = vec![None;metadata.candidates.len()];
        let candidate_name_lookup = metadata.get_candidate_name_lookup();
        let candidate_from_name = |name:&str| {
            candidate_name_lookup.get(name).cloned().or_else(||{
                // Sometimes the PDF file contains a different anglicalization of the name to the web page. Check for unique surnames in this case.
                if let Some((surname,firstname)) = name.split_once(',') {
                    let just_match_surname : Vec<CandidateIndex> = metadata.candidates.iter().enumerate().filter(|(_,c)|c.name.starts_with(surname)).map(|(i,_)|CandidateIndex(i)).collect::<Vec<_>>();
                    if just_match_surname.len()==1 { Some(just_match_surname[0]) } else {
                        let just_match_firstname : Vec<CandidateIndex> = metadata.candidates.iter().enumerate().filter(|(_,c)|c.name.ends_with(firstname)).map(|(i,_)|CandidateIndex(i)).collect::<Vec<_>>();
                        if just_match_firstname.len()==1 { Some(just_match_firstname[0]) } else { None }
                    }
                } else { None }
            })
        };
        let party_id_lookup = metadata.get_party_id_lookup();
        let mut found_candidate_party_name = true;
        for page in pdf.pages() {
            let page = page?;
            if let Some(content) = &page.contents {
                for op in &content.operations {
                    match op.operator.to_uppercase().as_str() {
                        "BT" => {  current_font=None; }
                        "TF" if op.operands.len()==2 => {  current_font=Some(op.operands[0].as_name()?.to_string()); }
                        "TJ" => {
                            let text = extract_string(op);
                            //println!("TJ : {} (font {:?})",text,current_font);
                            if text.starts_with("Group ") && text.ends_with(" Voting") {
                                let group = text[5..text.len()-6].trim();
                                if let Some(&party) = party_id_lookup.get(group) {
                                    //println!("Group voting ticket for {}",party);
                                    if !current_ticket_contents.iter().all(|c|c.is_none()) {
                                        if let Some(prior_ticket) = current_group_ticket.take() { // This is duplicated below! Ugh!
                                            if !current_ticket_contents.iter().all(|c|c.is_some()) { return Err(anyhow!("Partial ticket found"))}
                                            metadata.parties[prior_ticket.0].tickets.push(current_ticket_contents.iter_mut().map(|c|c.take().unwrap()).collect())
                                        }
                                    }
                                    current_group_ticket=Some(party);
                                }
                            } else if current_font==font_of_preference_numbers {
                                if let Ok(preference_number) = text.parse::<usize>() {
                                    //println!("Preference number {}",preference_number);
                                    current_preference_number=Some(preference_number);
                                } else {
                                    found_candidate_party_name = true;
                                }
                            } else if current_font==font_of_address1 || current_font==font_of_address2 {
                                if let Some(preference_number) = current_preference_number.take() {
                                    if let Some(name) = current_candidate_name.take() {
                                        if let Some(candidate_index) = candidate_from_name(&name) {
                                            //println!("Found address. Preference number {} for {}",preference_number,candidate_index);
                                            if preference_number==0 || preference_number>current_ticket_contents.len() { return Err(anyhow!("Unexpected preference {}",preference_number))}
                                            if current_ticket_contents[preference_number-1].is_some() { return Err(anyhow!("Duplicated preference {}",preference_number))}
                                            current_ticket_contents[preference_number-1]=Some(candidate_index);
                                        } else { return Err(anyhow!("Did not understand candidate name {}",name))}
                                    } else { return Err(anyhow!("Found address without candidate name"))}
                                }
                            } else if current_font==font_of_surname && current_preference_number.is_some() {
                               // println!("Found surname {}",text);
                                current_candidate_name=Some(text);
                                found_candidate_party_name=false;
                            } else if current_font==font_of_firstname && current_preference_number.is_some() && !found_candidate_party_name {
                                if let Some(name) = current_candidate_name.as_mut() {
                                    //println!("Found first name {}",text);
                                    name.push_str(&text);
                                }
                            }
                            //if res.len()>0 && current_font==font_of_last_text { res.last_mut().unwrap().push_str(&text) }
                            //else { res.push(text); font_of_last_text=current_font.clone(); }
                        }
                        _ => {}
                    }
                }
            }
        }
        if let Some(prior_ticket) = current_group_ticket.take() { // This is duplicated above! Ugh!
            if !current_ticket_contents.iter().all(|c|c.is_some()) { return Err(anyhow!("Partial ticket found"))}
            metadata.parties[prior_ticket.0].tickets.push(current_ticket_contents.iter_mut().map(|c|c.take().unwrap()).collect())
        }
        metadata.source.push(DataSource{
            url: "https://www.vec.vic.gov.au/candidates-and-parties/become-a-state-election-candidate/groups-and-voting-tickets".to_string(),
            files: vec![filename],
            comments: None
        });
        Ok(())
    }

    /// Produce a list of (atl,btl) votes from the table of ATL and BTL votes at e.g. https://www.vec.vic.gov.au/results/2022-state-election-results/results-by-region/northern-metropolitan-region-results
    fn parse_results_by_region_intermediate_webpage(&self,metadata:&mut ElectionMetadata) -> anyhow::Result<EarlyFirstPreferenceVotes> {
        let filename = format!("{}-results.html", Self::region_human_name_to_computer_name(&metadata.name.electorate,true));
        let url = format!("https://www.vec.vic.gov.au/results/2022-state-election-results/results-by-region/{}-results",Self::region_human_name_to_computer_name(&metadata.name.electorate,true));
        let path = self.finder.find_raw_data_file_with_extra_url_info(&filename, &self.archive_location, &url,"")?;
        let html = scraper::Html::parse_document(&std::fs::read_to_string(&path)?);
        let table = html.select(&scraper::Selector::parse("main table > tbody").unwrap()).next().ok_or_else(||anyhow!("Could not find main table in region results html page {}",filename))?;
        let mut by_party: Vec<(usize, usize)> = Vec::default();
        let mut by_candidate : Vec<usize> = vec![0;metadata.candidates.len()];
        let candidate_of_name = metadata.get_candidate_name_lookup();
        for tr in table.select(&scraper::Selector::parse("tr").unwrap()) {
            let tds = tr.select(&scraper::Selector::parse("td").unwrap()).collect::<Vec<_>>();
            if tds.len()>3 { // first tr just has headings
                let id = tds[0].text().collect::<Vec<_>>().join("").trim().to_string();
                let name = tds[1].text().collect::<Vec<_>>().join("").trim().to_string();
                let atl : usize = tds[2].text().collect::<Vec<_>>().join("").trim().parse()?;
                let btl : usize = tds[3].text().collect::<Vec<_>>().join("").trim().parse()?;
                if id.is_empty() {
                    let candidate = *candidate_of_name.get(&name).ok_or_else(||anyhow!("Unexpected name {}",name))?;
                    by_candidate[candidate.0] = atl+btl;
                } else {
                    if by_party.len()>=metadata.parties.len() { return Err(anyhow!("More rows in table than there are parties")); }
                    if id!=metadata.parties[by_party.len()].column_id { return Err(anyhow!("Found unexpected party id {}",id)); }
                    by_party.push((atl, btl))
                }
            }
        }
        metadata.source.push(DataSource{
            url,
            files: vec![filename],
            comments: None
        });
        Ok(EarlyFirstPreferenceVotes{by_party,by_candidate})
    }
}

/// First preferences are counted before everything else. Used when they are available but others are not.
#[derive(Debug)]
struct EarlyFirstPreferenceVotes {
    /// for each party, the number of (atl,btl) first pref votes
    by_party : Vec<(usize,usize)>,
    /// for each candidate, the number of btl first pref votes if not counted above (e.g. for ungrouped candidates).
    by_candidate : Vec<usize>,
}

/// A PDF TJ operation takes a string, or rather an array of strings and other stuff. Extract just the string. Also works for Tj
pub(crate) fn extract_string(op:&pdf::content::Operation) -> String {
    let mut res = String::new();
    // println!("{:?}",op);
    for o in &op.operands {
        if let Ok(a) = o.as_array() {
            for p in a {
                if let Ok(s) = p.as_string() {
                    if let Ok(s) = s.as_str() {
                        res.push_str(&s);
                    }
                }
            }
        } else if let Ok(s) = o.as_string() {
            if let Ok(s) = s.as_str() {
                res.push_str(&s);
            }
        }
    }
    res
}



#[cfg(test)]
mod tests {
    use stv::parse_util::FileFinder;
    use crate::parse_vic::get_vic_data_loader_2022;
    #[test]
    fn test_pdf_parse() -> anyhow::Result<()>{
        let loader = get_vic_data_loader_2022(&FileFinder::find_ec_data_repository())?;
        let mut metadata = loader.read_raw_metadata_from_candidate_list_on_main_website("Northern Metropolitan Region")?;
        loader.parse_pdf_tickets_file(&mut metadata)?;
        let first_pref_votes = loader.parse_results_by_region_intermediate_webpage(&mut metadata)?;
        println!("{:?}",metadata);
        println!("{:?}",first_pref_votes);
        Ok(())
    }
}

/// The headings for the distribution of preferences (DOP) file (excel spreadsheet).
/// The parsing of this is split into two sections
///  * Read the quota and headings (making this structure)
///  * Read everything else.
///
/// This separation is done partly because it is conceptually what is going on and limits the span
/// of mutable variables, but mostly because it is needed to reorder the candidates into ballot paper
/// order as the file used to get the candidates and parties is not in ballot paper order, and the
/// file of votes does not specify the ballot paper order, and this file does not give parties.
struct DOPFileFormat {
    quota:QuotaInfo<f64>,
    count_details_columm:u32,
    tv_column:u32,
    rounding_column:u32,
    exhausted_column:u32,
    elected_column :u32,
    // We need to map the candidates in the headings to the metadata as some columns (e.g. K and L in easternvictoriaregiondistribution.xls 2014) are missing!
    column_for_candidate : Vec<u32>, // column_for_candidate[candidate_index] = column for candidate i.
    table_headings_row : u32,
}

impl DOPFileFormat {
    // Read the spreadsheet to get the metadata (quota), and columns for candidates and other things.
    fn new(sheet1:&calamine::Range<DataType>,metadata:&ElectionMetadata) -> anyhow::Result<Self> {
        let mut formal_ballots : Option<BallotPaperCount> = None;
        let mut quota_size : Option<f64> = None;
        let mut count_details_columm = 1;
        let mut tv_column = 2;
        let mut rounding_column = u32::MAX;
        let mut exhausted_column = u32::MAX;
        let mut elected_column = u32::MAX;
        // We need to map the candidates in the headings to the metadata as some columns (e.g. K and L in easternvictoriaregiondistribution.xls 2014) are missing!
        let mut column_for_candidate : Vec<u32> = vec![u32::MAX;metadata.candidates.len()]; // column_for_candidate[candidate_index] = column for candidate i.
        for row in 0..(sheet1.height() as u32) {
            let string_value = |col: u32| { sheet1.get_value((row, col)).and_then(|v|v.get_string()) };
            if let Some(s) = string_value(0) {
                if &(s.chars().filter(|c|!c.is_whitespace()).collect::<String>())=="CountNo." {
                    // set up columns based on headings.
                    let candidate_of_name = metadata.get_candidate_name_lookup();
                    for col in 1..(sheet1.width() as u32) {
                        if let Some(heading) = string_value(col) {
                            let heading : String = heading.trim().chars().filter(|&c|c!='\n').collect();
                            if let Some(candidate) = candidate_of_name.get(&heading) {
                                column_for_candidate[candidate.0] = col;
                            } else {
                                match heading.as_str() {
                                    "Transfer Value" => tv_column=col,
                                    "Count Details" => count_details_columm=col,
                                    "Gain/Loss" => rounding_column=col,
                                    "Exhausted" => exhausted_column=col,
                                    "TOTAL" => { } // boring.
                                    "Candidates provisionally elected at this count" => elected_column=col,
                                    "Candidates elected at this count" => elected_column=col,
                                    "" => {}
                                    _ => {return Err(anyhow!("Don't understand heading {}",heading));}
                                }
                            }
                        }
                    }
                    // finished with metadata
                    let quota = QuotaInfo{
                        papers: formal_ballots.ok_or_else(||anyhow!("No formal ballots heading found"))?,
                        vacancies: NumberOfCandidates(5),
                        quota: quota_size.ok_or_else(||anyhow!("No quota heading found"))?
                    };
                    return Ok(DOPFileFormat{
                        quota,
                        count_details_columm,
                        tv_column,
                        rounding_column,
                        exhausted_column,
                        elected_column,
                        column_for_candidate,
                        table_headings_row: row,
                    })
                } else if let Some((desc,num)) = s.split_once(':') {
                    if desc.trim()=="Formal Ballot Papers included in count" { formal_ballots = Some(BallotPaperCount(num.trim().parse()?))}
                    else if desc.trim()=="Quota" { quota_size = Some(num.trim().parse()?)}
                } else {
                    // println!("Found {}",s);
                }
            }
        }
        Err(anyhow!("Could not find the headings row in DOP spreadsheet"))
    }
    /// Reorders the candidates to match the order in this file.
    /// Unfortunately the ungrouped candidates in the votesreceived file do not specify the order of candidates reliably.
    /// Invalidates this structure.
    fn reorder_candidates_to_match_this(self,metadata:&mut ElectionMetadata) {
        let mut candidates_with_index : Vec<(usize,Candidate)> = metadata.candidates.drain(..).enumerate().collect();
        candidates_with_index.sort_unstable_by_key(|(index,_)|self.column_for_candidate[*index]);
        let mut candidates_without_index : Vec<Candidate> = candidates_with_index.into_iter().map(|(_,c)|c).collect();
        metadata.candidates.append(&mut candidates_without_index);
    }

    /// Reads the rest of the spreadsheet given this format.
    fn parse_spreadsheet(self,sheet1:&calamine::Range<DataType>,metadata:&ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        let mut counts = vec![];
        let mut already_excluded : HashSet<CandidateIndex> = HashSet::new();
        for row in (self.table_headings_row+1)..(sheet1.height() as u32) {
            let value  = |deltarow:u32,col:u32| { sheet1.get_value((row+deltarow,col)) };
            let f64_value = |deltarow:u32,col:u32| { value(deltarow,col).and_then(|v|v.get_float()) };
            let f64_value_or_0  = |deltarow:u32,col:u32| -> f64 { f64_value(deltarow,col).unwrap_or(0.0) };
            if let Some(n) = f64_value(0,0) {
                if n==(counts.len()+1) as f64 {
                    let transfer_value : Option<f64> = sheet1.get_value((row as u32,self.tv_column)).and_then(|v|v.get_float());
                    let mut elected : Vec<CandidateIndex> = vec![];
                    let mut excluded : Vec<CandidateIndex> = vec![];
                    let is_first = counts.is_empty();
                    if let Some(elected_names) = value(if is_first {0} else {2},self.elected_column).and_then(|v|v.get_string()) {
                        // println!("Elected column {}",elected_names);
                        let mut remaining : &str = elected_names.trim(); // there may be multiple names separated by commas. Names contain commas. Argh!
                        while remaining.len()>0 {
                            let mut found = false;
                            for candidate_index in 0..metadata.candidates.len() {
                                if remaining.starts_with(&metadata.candidates[candidate_index].name) {
                                    elected.push(CandidateIndex(candidate_index));
                                    remaining=&remaining[metadata.candidates[candidate_index].name.len()..];
                                    remaining=remaining.trim();
                                    if remaining.starts_with(',') { remaining=remaining[1..].trim(); }
                                    found=true;
                                }
                            }
                            if !found { return Err(anyhow!("Could not interpret names {} problem with {}",elected_names,remaining))}
                        }
                    }
                    let mut papers_came_from_counts: Option<Vec<CountIndex>> = None;
                    if let Some(count_details) = value(0,self.count_details_columm).and_then(|v|v.get_string()) {
                        // println!("Details column {}",count_details);
                        if count_details.starts_with("Exclusion of ") {
                            let remaining = count_details["Exclusion of ".len()..].trim();
                            if let Some(who) = metadata.candidates.iter().position(|c|remaining.starts_with(&c.name)) {
                                let newly_inserted = already_excluded.insert(CandidateIndex(who));
                                if newly_inserted {
                                    excluded.push(CandidateIndex(who));
                                }
                            } else { return Err(anyhow!("Could not work out who was excluded in {}",count_details))}
                            papers_came_from_counts=OfficialDOPForOneCount::extract_counts_from_comment(remaining,"from count ","")?;
                            if papers_came_from_counts.is_none() { return Err(anyhow!("Could not work out who which counts contributed {}",count_details))}
                        }
                    }
                    let get_row = |rowdelta:u32|-> PerCandidate<f64> {
                        let candidate : Vec<f64> = self.column_for_candidate.iter().map(|&col|f64_value_or_0(rowdelta,col)).collect();
                        let exhausted = f64_value_or_0(rowdelta,self.exhausted_column);
                        let rounding = f64_value_or_0(rowdelta,self.rounding_column);
                        PerCandidate {
                            candidate,
                            exhausted,
                            rounding : SignedVersion::from(rounding),
                            set_aside: None
                        }
                    };
                    counts.push(OfficialDOPForOneCount{
                        transfer_value,
                        elected,
                        excluded,
                        vote_total : Some(get_row(if is_first {0} else {2})),
                        paper_total : None,
                        vote_delta : Some(get_row(if is_first {0} else {1})),
                        paper_delta : Some(get_row(0).try_into()?),
                        paper_set_aside_for_quota: None,
                        count_name: None,
                        papers_came_from_counts,
                    });
                }
            }
        }
        Ok(OfficialDistributionOfPreferencesTranscript{ quota : Some(self.quota), counts ,missing_negatives_in_papers_delta:false, elected_candidates_are_in_order: true, all_exhausted_go_to_rounding: false })
    }
}

impl VicDataLoader {
    fn new(finder:&FileFinder,year:&'static str,page_url:&'static str) -> anyhow::Result<Self> {
        let archive_location = "VIC/State".to_string()+year;
        Ok(VicDataLoader {
            finder : finder.clone(),
            archive_location,
            year: year.to_string(),
            page_url: page_url.to_string(),
        })
    }

    fn reorder_candidates_in_metadata_by_official_dop_transcript(&self, metadata: &mut ElectionMetadata) -> anyhow::Result<()>  {
        let filename = self.distribution_of_preferences_filename(&metadata.name.electorate)?;
        let path = self.find_raw_data_file(&filename)?;
        use calamine::Reader;
        let mut workbook1 = open_workbook_auto(&path)?;
        let sheet1 = workbook1.worksheet_range_at(0).ok_or_else(||anyhow!("No sheets in {}",path.to_string_lossy()))??;
        let format = DOPFileFormat::new(&sheet1,metadata).context(path.to_string_lossy().to_string())?;
        format.reorder_candidates_to_match_this(metadata);
        Ok(())
    }
    /// Convert a name like "South-Eastern Metropolitan Region" to "south-easternmetropolitanregion" as used in file names. or "south-eastern-metropolitan-region" as used in urls
    fn region_human_name_to_computer_name(electorate:&str,convert_spaces_to_hypens:bool) -> String {
        electorate.chars().map(|c|if convert_spaces_to_hypens&&c==' ' {'-'} else {c}).filter(|c|!c.is_whitespace()).map(|c|c.to_ascii_lowercase()).collect()
    }
    fn distribution_of_preferences_filename(&self,electorate:&str) -> anyhow::Result<String> {
        Ok(match self.year.as_str() {
            "2014" => format!("{}distributions.xls",Self::region_human_name_to_computer_name(electorate,false)),
            "2018" => format!("{}distributions.xls",Self::region_human_name_to_computer_name(electorate,false)),
            "2022" => format!("{}-Distribution of Preferences.xls",electorate),
            _ => { return Err(anyhow!("Do not know name for distribution of preferences file for year {}",self.year))}
        })
    }

}