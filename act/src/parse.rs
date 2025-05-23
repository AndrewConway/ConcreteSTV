// Copyright 2021-2025 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.



//! Parse files used by the Elections ACT for vote data



use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use stv::parse_util::{FileFinder, KnowsAboutRawMarkings, MissingFile, RawDataSource, read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions};
use std::path::PathBuf;
use stv::ballot_metadata::{Party, CandidateIndex, Candidate, PartyIndex, ElectionMetadata, DataSource, ElectionName, NumberOfCandidates};
use stv::election_data::ElectionData;
use stv::tie_resolution::TieResolutionsMadeByEC;
use stv::ballot_paper::UniqueBTLBuilder;
use anyhow::anyhow;
use serde::Deserialize;
use stv::official_dop_transcript::{OfficialDistributionOfPreferencesTranscript, OfficialDOPForOneCount};
use calamine::{open_workbook_auto, DataType};
use stv::datasource_description::{AssociatedRules, Copyright, ElectionDataSource};
use stv::distribution_of_preferences_transcript::{CountIndex, PerCandidate, QuotaInfo};
use crate::{ACT2020, ACT2021, ACTPre2020};

pub fn get_act_data_loader_2024(finder:&FileFinder) -> anyhow::Result<ACTDataLoader> {
    ACTDataLoader::new(finder,"2024","https://www.elections.act.gov.au/elections/previous-assembly-elections/2024-election/ballot-paper-preference-data")
}

// Get a data loader that loads the original Distribution of Preferences.
pub fn get_act_data_loader_2020_0(finder:&FileFinder) -> anyhow::Result<ACTDataLoader> {
    ACTDataLoader::new(finder,"2020.0","https://www.elections.act.gov.au/elections_and_voting/2020_legislative_assembly_election/ballot-paper-preference-data-2020-election")
}
// Get a data loader that loads the March 2021 updated Distribution of Preferences.
pub fn get_act_data_loader_2020(finder:&FileFinder) -> anyhow::Result<ACTDataLoader> {
    ACTDataLoader::new(finder,"2020","https://www.elections.act.gov.au/elections_and_voting/2020_legislative_assembly_election/ballot-paper-preference-data-2020-election")
}
pub fn get_act_data_loader_2016(finder:&FileFinder) -> anyhow::Result<ACTDataLoader> {
    ACTDataLoader::new(finder,"2016","https://www.elections.act.gov.au/elections_and_voting/past_act_legislative_assembly_elections/2016-election/ballot-paper-preference-data-2016-election")
}
pub fn get_act_data_loader_2012(finder:&FileFinder) -> anyhow::Result<ACTDataLoader> {
    ACTDataLoader::new(finder,"2012","https://www.elections.act.gov.au/elections_and_voting/past_act_legislative_assembly_elections/2012_act_legislative_assembly_election/ballot_paper_preference_data_-_2012_election")
}
pub fn get_act_data_loader_2008(finder:&FileFinder) -> anyhow::Result<ACTDataLoader> {
    ACTDataLoader::new(finder,"2008","https://www.elections.act.gov.au/elections_and_voting/past_act_legislative_assembly_elections/2008_election/ballot_paper_preference_data_2008_election")
}


/// Note: do not use this for a website as data does not have a suitable license.
pub struct ACTDataSource {}

impl ElectionDataSource for ACTDataSource {
    fn name(&self) -> Cow<'static, str> { "ACT Legislative Assembly".into() }
    fn ec_name(&self) -> Cow<'static, str> { "Elections ACT".into() }
    fn ec_url(&self) -> Cow<'static, str> { "https://www.elections.act.gov.au/".into() }
    fn years(&self) -> Vec<String> { vec!["2008".to_string(),"2012".to_string(),"2016".to_string(),"2020".to_string()] } // 2020.0 means the original DoP not the fixed result posted in 2021.
    fn get_loader_for_year(&self,year: &str,finder:&FileFinder) -> anyhow::Result<Box<dyn RawDataSource+Send+Sync>> {
        match year {
            "2008" => Ok(Box::new(get_act_data_loader_2008(finder)?)),
            "2012" => Ok(Box::new(get_act_data_loader_2012(finder)?)),
            "2016" => Ok(Box::new(get_act_data_loader_2016(finder)?)),
            "2020" => Ok(Box::new(get_act_data_loader_2020(finder)?)),
            "2020.0" => Ok(Box::new(get_act_data_loader_2020_0(finder)?)),
            "2024" => Ok(Box::new(get_act_data_loader_2024(finder)?)),
            _ => Err(anyhow!("Not a valid year")),
        }
    }
}

pub struct ACTDataLoader {
    finder : FileFinder,
    archive_location : String,
    year : String,
    page_url : String,
    // data used for multiple electorates/reasons
    electorate_to_ecode : HashMap<String,usize>, // convert a human readable electorate to a ecode used in Elections ACT datafiles. An ecode is a small integer.
}

impl KnowsAboutRawMarkings for ACTDataLoader {}

impl RawDataSource for ACTDataLoader {
    fn name(&self,electorate:&str) -> ElectionName {
        ElectionName{
            year: self.year.clone(),
            authority: "Elections ACT".to_string(),
            name: "ACT Legislative Assembly".to_string(),
            electorate: electorate.to_string(),
            modifications: vec![],
            comment: None,
        }
    }

    fn candidates_to_be_elected(&self,region:&str) -> NumberOfCandidates {
        NumberOfCandidates( if region=="Molonglo" {7} else {5})
    }

    /// These are deduced by looking at the actual transcript of results.
    /// I have not included anything if all decisions are handled by the fallback "earlier on the ballot paper candidates are listed in worse positions.
    fn ec_decisions(&self,_electorate:&str) -> TieResolutionsMadeByEC {
        TieResolutionsMadeByEC::default()
    }

    /// These are due to a variety of events.
    fn excluded_candidates(&self,_electorate:&str) -> Vec<CandidateIndex> {
        Default::default()
    }

    // This below should be made more general and most of it factored out into a separate function.
    fn read_raw_data(&self,electorate:&str) -> anyhow::Result<ElectionData> {
        let prohibit_electronic = false;
        let mut metadata = self.read_raw_metadata(electorate)?;
        let filename = electorate.to_string()+"Total.txt";
        let preferences_text_file = self.find_raw_data_file(&filename)?;
        println!("Parsing {}",&preferences_text_file.to_string_lossy());
        metadata.source[0].files.push(filename);
        let candidate_of_pcode_and_ccode : HashMap<(String,usize),CandidateIndex> = metadata.candidates.iter().enumerate().map(|(i,c)|((metadata.parties[c.party.unwrap().0].column_id.clone(),c.position.unwrap()),CandidateIndex(i))).collect();
        #[derive(Deserialize)]
        struct VoteRecord {
            batch : String,
            pindex : String, // possibly not a unique id, but unique for a batch.
            pref:usize,
            pcode:String,
            ccode:usize,
            // rcand:usize,
        }
        let mut rdr = csv::Reader::from_path(&preferences_text_file)?;
        let mut last_paper : Option<String> = None;
        let mut btl = UniqueBTLBuilder::default();
        let mut prefs : Vec<CandidateIndex> = vec![];
        for result in rdr.deserialize() {
            let record: VoteRecord = result?;
            if prohibit_electronic && record.batch.ends_with("000") {} else {
                if let Some(&candidate) = candidate_of_pcode_and_ccode.get(&(record.pcode,record.ccode)) {
                    let paper = record.batch+"_"+&record.pindex;
                    if last_paper==None || last_paper.as_ref().unwrap()!=&paper {
                        if !prefs.is_empty() { btl.add(prefs.clone()); }
                        prefs.clear();
                        last_paper=Some(paper);
                    }
                    prefs.push(candidate);
                    if prefs.len()!=record.pref { return Err(anyhow!("Preferences not in order. No reason they should be other than they seem to be, but it saves work if we assume they are and this is a safety check"))}
                } else { return Err(anyhow!("Bad candidate"))}
            }
        }
        if !prefs.is_empty() { btl.add(prefs.clone()); }
        Ok(ElectionData{ metadata, atl:vec![], atl_types: vec![], atl_transfer_values: vec![], btl:btl.to_btls(), btl_types: vec![], btl_transfer_values: vec![], informal:0 })
    }

    fn read_raw_data_best_quality(&self, electorate: &str) -> anyhow::Result<ElectionData> {
        match self.year.as_str() {
            "2008"|"2012"|"2016" => read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<ACTPre2020,Self>(self,electorate),
            "2020"|"2024" => read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<ACT2021,Self>(self,electorate),
            "2020.0" => read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<ACT2020,Self>(self,electorate),
            _ => Err(anyhow!("Invalid year {}",self.year)),
        }
    }

    fn find_raw_data_file(&self,filename:&str) -> Result<PathBuf,MissingFile> {
        self.finder.find_raw_data_file(filename,&self.archive_location,&self.page_url)
    }
    fn all_electorates(&self) -> Vec<String> {
        self.electorate_to_ecode.keys().cloned().collect()
    }

    fn read_raw_metadata(&self,electorate:&str) -> anyhow::Result<ElectionMetadata> {
        let ecode = self.electorate_to_ecode.get(electorate).cloned().ok_or_else(||self.bad_electorate(electorate))?;
        let mut parties = self.load_parties(ecode)?;
        let candidates = self.load_candidates(ecode,&mut parties)?;
        let vacancies = self.candidates_to_be_elected(electorate);
        Ok(ElectionMetadata{
            name: self.name(electorate),
            candidates,
            parties,
            source: vec![DataSource{
                url: self.page_url.clone(),
                files: vec![(if self.year=="2024" {"candidates.txt"} else {"Candidates.txt"}).to_string(),"Electorates.txt".to_string(),"Groups.txt".to_string()],
                comments: None
            }],
            results: None,
            vacancies: Some(vacancies),
            enrolment: None,
            secondary_vacancies: None,
            excluded: self.excluded_candidates(electorate),
            tie_resolutions : self.ec_decisions(electorate),
        })
    }

    /* as at current time, requires distribution to be in unaltered form */
    fn copyright(&self) -> Copyright {
        Copyright{
            statement: Some("© Australian Capital Territory.".into()),
            url: Some("https://www.elections.act.gov.au/copyright".into()),
            license_name: None,
            license_url: None,
        }
    }

    fn rules(&self, _electorate: &str) -> AssociatedRules {
        match self.year.as_str() {
            "2008"|"2012"|"2016" => AssociatedRules{
                rules_used: Some("ACTPre2020".into()),
                rules_recommended: Some("ACTPre2020".into()),
                comment: None,
                reports: vec![]
            },
            "2020" | "2020.0" => AssociatedRules{
                rules_used: Some("ACT2020".into()),
                rules_recommended: Some("ACT2021".into()),
                comment: Some("The election was initially run with buggy rules ACT2020. After we pointed out the bugs, the counts on Elections ACT website were changed in 2021 to use the correct rules ACT2021".into()),
                reports: vec!["https://github.com/AndrewConway/ConcreteSTV/blob/main/reports/2020%20Errors%20In%20ACT%20Counting.pdf".into()]
            }, // TODO 2024
            _ => AssociatedRules{rules_used:None,rules_recommended:None,comment:None,reports:vec![]},
        }
    }

    fn read_official_dop_transcript(&self, metadata: &ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        let subfolder = match self.year.as_str() {
            "2020" => Some("D of P as at 26 Mar 2021"),
            _ => None
        };
        self.read_official_dop_transcript_with_subfolder(metadata,subfolder)
    }
}


impl ACTDataLoader {

    pub fn new(finder:&FileFinder,year:&'static str,page_url:&'static str) -> anyhow::Result<Self> {
        let archive_location = "ACT/".to_string()+year.trim_end_matches(".0");
        let electorate_to_ecode = read_electorate_to_ecode(&finder.find_raw_data_file("Electorates.txt",&archive_location,page_url)?)?;
        Ok(ACTDataLoader {
            finder : finder.clone(),
            archive_location,
            year: year.to_string(),
            page_url: page_url.to_string(),
            electorate_to_ecode,
        })
    }


    /// Get parties (without list of candidates)
    fn load_parties(&self,ecode:usize) -> anyhow::Result<Vec<Party>> {
        let path = self.find_raw_data_file("Groups.txt")?;
        let mut res = vec![];
        #[derive(Deserialize)]
        struct GroupsRecord {
            ecode: usize,
            pcode : String,
            pname : String,
            pabbrev : String,
            // cands : usize,
        }
        let mut rdr = csv::Reader::from_path(path)?;
        for result in rdr.deserialize() {
            let record : GroupsRecord = result?;
            if record.ecode == ecode {
                res.push(Party{
                    column_id: record.pcode,
                    name: record.pname,
                    abbreviation: Some(record.pabbrev),
                    atl_allowed: false,
                    candidates: vec![],
                    tickets: vec![],
                })
            }
        }
        Ok(res)
    }
    /// load candidates for a given ecode, and add to appropriate party.
    fn load_candidates(&self,ecode:usize,parties:&mut Vec<Party>) -> anyhow::Result<Vec<Candidate>> {
        let path = self.find_raw_data_file(if self.year=="2024" {"candidates.txt"} else {"Candidates.txt"})?;
        let mut res = vec![];
        #[derive(Deserialize)]
        struct CandidatesRecord {
            ecode: usize,
            pcode : String,
            ccode : usize,
            cname : String,
        }
        let mut rdr = csv::Reader::from_path(path)?;
        for result in rdr.deserialize() {
            let record : CandidatesRecord = result?;
            if record.ecode == ecode {
                let party = parties.iter().position(|p|p.column_id==record.pcode).map(|i|PartyIndex(i));
                if let Some(party_index) = party {
                    parties[party_index.0].candidates.push(CandidateIndex(res.len()));
                }
                res.push(Candidate{
                    name: record.cname,
                    party,
                    position: Some(record.ccode),
                    ec_id: None
                });
            }
        }
        Ok(res)
    }

    /// Read the official distribution of prefererences, which are excel files in a folder "Distribution Of Preferences".
    /// There may be a sub folder such as in the case of 2020 when Elections ACT redid their distribution after fixing some bugs we pointed out.
    pub fn read_official_dop_transcript_with_subfolder(&self,metadata:&ElectionMetadata,sub_folder:Option<&str>) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        let dop_folder = self.finder.find_raw_data_file("Distribution Of Preferences",&self.archive_location,"Just folder")?;
        let dop_folder = if let Some(sub) = sub_folder { dop_folder.join(sub) } else { dop_folder };
        let mut table1 : Option<PathBuf> = None;
        let mut table2 : Option<PathBuf> = None;
        for entry in std::fs::read_dir(&dop_folder)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.contains(&metadata.name.electorate) || name.contains(&metadata.name.electorate.to_ascii_lowercase()) {
                if name.contains("1") { table1 = Some(entry.path()); }
                else if name.contains("2") { table2 = Some(entry.path()); }
            }
        }
        if let Some(table1) = table1 {
            if let Some(table2) = table2 {
                parse_excel_tables_official_dop_transcript(table1,table2,metadata)
            } else { Err(anyhow!("Can't find table2 in {}",dop_folder.to_string_lossy()))}
        } else { Err(anyhow!("Can't find table1 in {}",dop_folder.to_string_lossy()))}
    }
}

fn parse_excel_tables_official_dop_transcript(table1:PathBuf,table2:PathBuf,metadata:&ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
    use calamine::Reader;
    let mut workbook1 = open_workbook_auto(&table1)?;
    let sheet1 = workbook1.worksheet_range_at(0).ok_or_else(||anyhow!("No sheets in table1"))??;
    let mut workbook2 = open_workbook_auto(&table2)?;
    let sheet2 = workbook2.worksheet_range_at(0).ok_or_else(||anyhow!("No sheets in table2"))??;
    let row_index_for_names : u32 = if table1.file_name().unwrap().to_string_lossy().ends_with("xlsx") {
        if metadata.name.year.starts_with("2024") {4} else {2}
    } else {1};
    let mut table1_col_for_candidate = vec![u32::MAX;metadata.candidates.len()];
    let mut table2_col_for_candidate = vec![u32::MAX;metadata.candidates.len()];
    let mut table1_col_for_exhausted_papers : Option<u32> = None;
    let mut table1_col_for_transfer_value : Option<u32> = None;
    let mut table1_col_for_description_of_choices: Option<u32> = None;
    let mut table2_col_for_exhausted_votes : Option<u32> = None;
    let mut table2_col_for_loss_by_fraction : Option<u32> = None;
    let mut table2_col_for_remarks : Option<u32> = None;
    let count_column : u32 = 0;
    let mut candidate_of_name = metadata.get_candidate_name_lookup_multiple_ways();
    if metadata.name.year=="2024" && metadata.name.electorate=="Yerrabi" { candidate_of_name.insert("SoÃ«lily CONSEN-LYNCH".to_string(),CandidateIndex(1));}// bad unicode handling in the 2024 Yerrabi DofP file.
    for col in count_column+1 .. sheet1.width() as u32 {
        if let Some(v) = sheet1.get_value((row_index_for_names,col)) {
            if let Some(s) = v.get_string() {
                let s = if s=="SoÃ«lily CONSEN-LYNCH" { "Soelily CONSEN-LYNCH" } else {s}; // bad unicode handling in the 2024 Yerrabi table 1 xlsx file.
                for candidate_index in 0..metadata.candidates.len() {
                    if s.contains(&metadata.candidates[candidate_index].name) || s.contains(&metadata.candidates[candidate_index].no_comma_name()){
                        table1_col_for_candidate[candidate_index]=col;
                    }
                }
                if s.contains("Papers Exhausted at Count") { table1_col_for_exhausted_papers=Some(col); }
                else if s.contains("Transfer Value") { table1_col_for_transfer_value=Some(col); }
                else if s.contains("Description of Choices Counted") { table1_col_for_description_of_choices=Some(col); }
            }
        }
        if let Some(v) = sheet1.get_value((1,col)) {
            if let Some(s) = v.get_string() {
                if s.contains("Papers Exhausted at Count") { table1_col_for_exhausted_papers=Some(col); }
                else if s.contains("Transfer Value") { table1_col_for_transfer_value=Some(col); }
                else if s.contains("Description of Choices Counted") { table1_col_for_description_of_choices=Some(col); }
            }
        }
    }
    if table1_col_for_candidate.contains(&u32::MAX) { return Err(anyhow!("Could not find all candidates in table1... missing {} of {}",table1_col_for_candidate.iter().enumerate().filter_map(|(candidate,col)|if *col==u32::MAX {Some(candidate.to_string())} else {None}).collect::<Vec<_>>().join(","),table1_col_for_candidate.len()));}
    let table1_col_for_exhausted_papers=table1_col_for_exhausted_papers.ok_or_else(||anyhow!("Could not find exhausted papers column in table 1"))?;
    let table1_col_for_transfer_value=table1_col_for_transfer_value.ok_or_else(||anyhow!("Could not find transfer value column in table 1"))?;
    let table1_col_for_description_of_choices= table1_col_for_description_of_choices.ok_or_else(||anyhow!("Could not find description of choices counted column in table 1"))?;
    let row_index_for_names_table2 = if metadata.name.year=="2024" { 5 } else { row_index_for_names };
    for col in count_column+1 .. sheet2.width() as u32 {
        if let Some(v) = sheet2.get_value((row_index_for_names_table2,col)) {
            if let Some(s) = v.get_string() {
                let s = if s=="SoÃ«lily CONSEN-LYNCH" { "Soelily CONSEN-LYNCH" } else {s}; // bad unicode handling in the 2024 Yerrabi table 2 xlsx file.
                for candidate_index in 0..metadata.candidates.len() {
                    if s.contains(&metadata.candidates[candidate_index].name) || s.contains(&metadata.candidates[candidate_index].no_comma_name()){
                        table2_col_for_candidate[candidate_index]=col;
                    }
                }
                if s.contains("Votes Exhausted at Count") { table2_col_for_exhausted_votes=Some(col); }
                else if s.contains("Loss by fraction") { table2_col_for_loss_by_fraction=Some(col); }
                else if s.contains("Remarks") { table2_col_for_remarks=Some(col); }
            }
        }
        if let Some(v) = sheet2.get_value((1,col)) {
            if let Some(s) = v.get_string() {
                if s.contains("Votes Exhausted at Count") { table2_col_for_exhausted_votes=Some(col); }
                else if s.contains("Loss by fraction") { table2_col_for_loss_by_fraction=Some(col); }
                else if s.contains("Remarks") { table2_col_for_remarks=Some(col); }
            }
        }
    }
    if table2_col_for_candidate.contains(&u32::MAX) { return Err(anyhow!("Could not find all candidates in table2"));}
    let table2_col_for_exhausted_votes=table2_col_for_exhausted_votes.ok_or_else(||anyhow!("Could not find exhausted votes column in table 2"))?;
    let table2_col_for_loss_by_fraction=table2_col_for_loss_by_fraction.ok_or_else(||anyhow!("Could not find loss by fraction column in table 2"))?;
    let table2_col_for_remarks=table2_col_for_remarks.ok_or_else(||anyhow!("Could not find remarks column in table 2"))?;
    // now we understand the columns, we can actually read it.
    let mut row_index = row_index_for_names_table2+1;
    let mut paper_row_index = row_index_for_names+1;
    let quota : Option<QuotaInfo<f64>> = None;
    fn parse_transfer_value(f:&impl DataType) -> Option<f64> { // TV may be a string ratio
        f.get_float().or_else(||{
            if let Some(s)=f.get_string() {
                if let Ok(v) = s.parse::<f64>() { Some(v) }
                else if let Some((num,denom)) = s.split_once('/') {
                    if let Ok(num) = num.trim().parse::<f64>() {
                        if let Ok(denom) = denom.trim().parse::<f64>() {
                            Some(num/denom)
                        } else {None}
                    } else {None}
                } else {None}
            } else {None}
        })
    }
    fn parse_num_possibly_blank(f:Option<&impl DataType>) -> f64 {
        f.and_then(|v|v.get_float().or_else(||v.get_string().and_then(|s|s.trim().parse::<f64>().ok()))).unwrap_or(0.0)
    }
    let mut counts = vec![];
    let mut not_continuing : HashSet<CandidateIndex> = HashSet::default();
    loop {
        let only_1_row = counts.is_empty() && metadata.name.year.starts_with("2020");
        // table 1: row_index contains the delta.
        let transfer_value : Option<f64> = sheet1.get_value((row_index,table1_col_for_transfer_value)).and_then(parse_transfer_value);
        let mut elected : Vec<CandidateIndex> = vec![];
        let mut excluded : Vec<CandidateIndex> = vec![];
        let sheet2_delta_row = row_index; // the row containing delta values on sheet2.
        let remarks_1 = sheet2.get_value((sheet2_delta_row,table2_col_for_remarks)).and_then(|v|v.get_string());
        let remarks_2 = if only_1_row {None} else {sheet2.get_value((sheet2_delta_row+1,table2_col_for_remarks)).and_then(|v|v.get_string())};
        for &remarks in [remarks_1,remarks_2].iter().flatten() {
            for remark in remarks.split('.') {
                //println!("Processing remark {}",remark);
                if let Some((name,_)) = remark.split_once(" elected ") {
                    let name=name.trim();
                    let name = name.trim_start_matches("First preferences "); // in 2021 runs into elected candidate name without period.
                    //println!("Found election of {}",name);
                    let candidate = *candidate_of_name.get(name).ok_or_else(|| anyhow!("Can't find elected candidate name {}",name))?;
                    elected.push(candidate);
                    not_continuing.insert(candidate);
                } else if let Some((name,_)) = remark.split_once("'s votes ") {
                    let name=name.trim();
                    let candidate = *candidate_of_name.get(name).ok_or_else(|| anyhow!("Can't find distributed candidate name {}",name))?;
                    if not_continuing.insert(candidate) {
                        excluded.push(candidate);
                    }
                }
            }
        }
        let paper_total : Option<PerCandidate<usize>> = None;
        let vote_delta : Option<PerCandidate<f64>>=Some(PerCandidate{
            candidate: table2_col_for_candidate.iter().map(|c|parse_num_possibly_blank(sheet2.get_value((sheet2_delta_row, *c)))).collect(),
            exhausted: parse_num_possibly_blank(sheet2.get_value((sheet2_delta_row, table2_col_for_exhausted_votes))),
            rounding: parse_num_possibly_blank(sheet2.get_value((sheet2_delta_row, table2_col_for_loss_by_fraction))).into(),
            set_aside: None,
        });
        let vote_total : Option<PerCandidate<f64>> = if only_1_row { vote_delta.clone() } else {Some(PerCandidate{
            candidate: table2_col_for_candidate.iter().map(|c|parse_num_possibly_blank(sheet2.get_value((sheet2_delta_row +1, *c)))).collect(),
            exhausted: parse_num_possibly_blank(sheet2.get_value((sheet2_delta_row +1, table2_col_for_exhausted_votes))),
            rounding: parse_num_possibly_blank(sheet2.get_value((sheet2_delta_row +1, table2_col_for_loss_by_fraction))).into(),
            set_aside: None,
        })};
        let paper_delta : Option<PerCandidate<isize>>=Some(PerCandidate{
            candidate: table1_col_for_candidate.iter().map(|c|parse_num_possibly_blank(sheet1.get_value((paper_row_index,*c))) as isize).collect(),
            exhausted: parse_num_possibly_blank(sheet1.get_value((paper_row_index,table1_col_for_exhausted_papers))) as isize,
            rounding: 0.into(),
            set_aside: None,
        });
        let mut papers_came_from_counts : Option<Vec<CountIndex>> = None;
        if !counts.is_empty() {
            let (row_offset,start_count_list_string,end_count_list_string) = match metadata.name.year.as_str() {
                "2008" => ( 0,"On Papers at Count ",","),
                "2012" => ( 0,"On Papers at Count ",","),
                "2016" => (-1,"On Papers at Count ",","),
                "2020"|"2020.0"|"2024" => { // 2020.0 is the buggy version, but some older code paths might get it other ways.
                    let buggy_version = if let Some(s) = sheet1.get_value((paper_row_index,table1_col_for_description_of_choices)).and_then(|v|v.get_string()) { // the original DOPs published had lots of bugs and a different format.
                        s.starts_with("From counts")
                    } else { false };
                    if buggy_version { (0, "From counts", "") } else {(-1,"On Papers at Count ","")}
                }
                _ => return Err(anyhow!("Do not know how to parse Description of Choices Counted for {}",metadata.name.year)),
            };
            if let Some(s) = sheet1.get_value(((paper_row_index as i32+row_offset) as u32,table1_col_for_description_of_choices)).and_then(|v|v.get_string()) {
                // println!("counts len {}, tv={:?}, s={}",counts.len(),transfer_value,s);
                papers_came_from_counts=OfficialDOPForOneCount::extract_counts_from_comment(s,start_count_list_string,end_count_list_string)?;
                if metadata.name.year=="2020" && metadata.name.electorate=="Kurrajong" && s=="From counts55" { papers_came_from_counts=Some(vec![CountIndex(51)])} // That year was really special. See the wayback machine to get the spreadsheet needing this.
                if papers_came_from_counts.is_none() { return Err(anyhow!("Could not work out who which counts contributed {}",s))}
            } else {
                return Err(anyhow!("Could not find which counts this came from row {} column {table1_col_for_description_of_choices}",(paper_row_index as i32+row_offset) as u32))
            }
        }
        //println!("{:?}",paper_delta.as_ref().unwrap());
        row_index+=if only_1_row {1} else {2};
        paper_row_index+=2;
        counts.push(OfficialDOPForOneCount{
            transfer_value,
            elected,
            excluded,
            vote_total,
            paper_total,
            vote_delta,
            paper_delta,
            paper_set_aside_for_quota: None,
            count_name: None,
            papers_came_from_counts,
        });
        if sheet2.get_value((row_index,count_column)).is_none()&&sheet2.get_value((row_index+1,count_column)).is_none() {
           return Ok(OfficialDistributionOfPreferencesTranscript{ quota, counts ,missing_negatives_in_papers_delta:true, elected_candidates_are_in_order: true, all_exhausted_go_to_rounding: false, negative_values_in_surplus_distributions_and_rounding_may_be_off: false })
        }
    }


}

fn read_electorate_to_ecode(path : &PathBuf) -> anyhow::Result<HashMap<String, usize>> { // process Electorates.txt
    let mut res : HashMap<String,usize> = HashMap::default();
    #[derive(Deserialize)]
    struct ElectoratesRecord {
        ecode : usize,
        electorate : String,
    }
    let mut rdr = csv::Reader::from_path(path)?;
    for result in rdr.deserialize() {
        let record : ElectoratesRecord = result?;
        res.insert(record.electorate,record.ecode);
    }
    Ok(res)
}