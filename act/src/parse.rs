// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.



//! Parse files used by the AEC for vote data



use std::collections::{HashMap, HashSet};
use stv::parse_util::{FileFinder, MissingFile, RawDataSource};
use std::path::PathBuf;
use stv::ballot_metadata::{Party, CandidateIndex, Candidate, PartyIndex, ElectionMetadata, DataSource, ElectionName, NumberOfCandidates};
use stv::election_data::ElectionData;
use stv::tie_resolution::TieResolutionsMadeByEC;
use stv::ballot_paper::UniqueBTLBuilder;
use anyhow::anyhow;
use serde::Deserialize;
use stv::official_dop_transcript::{OfficialDistributionOfPreferencesTranscript, OfficialDOPForOneCount};
use calamine::{open_workbook_auto, DataType};
use stv::distribution_of_preferences_transcript::{PerCandidate, QuotaInfo};

pub fn get_act_data_loader_2020(finder:&FileFinder) -> anyhow::Result<ACTDataLoader> {
    ACTDataLoader::new(finder,"2020","https://www.elections.act.gov.au/elections_and_voting/past_act_legislative_assembly_elections/2020-election")
}
pub fn get_act_data_loader_2016(finder:&FileFinder) -> anyhow::Result<ACTDataLoader> {
    ACTDataLoader::new(finder,"2016","https://www.elections.act.gov.au/elections_and_voting/past_act_legislative_assembly_elections/2016-election")
}
pub fn get_act_data_loader_2012(finder:&FileFinder) -> anyhow::Result<ACTDataLoader> {
    ACTDataLoader::new(finder,"2012","https://www.elections.act.gov.au/elections_and_voting/past_act_legislative_assembly_elections/2012_act_legislative_assembly_election")
}
pub fn get_act_data_loader_2008(finder:&FileFinder) -> anyhow::Result<ACTDataLoader> {
    ACTDataLoader::new(finder,"2008","https://www.elections.act.gov.au/elections_and_voting/past_act_legislative_assembly_elections/2008_election")
}

pub struct ACTDataLoader {
    finder : FileFinder,
    archive_location : String,
    year : String,
    page_url : String,
    // data used for multiple electorates/reasons
    electorate_to_ecode : HashMap<String,usize>, // convert a human readable electorate to a ecode used in Elections ACT datafiles. An ecode is a small integer.
}

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
    fn ec_decisions(&self,state:&str) -> TieResolutionsMadeByEC { // TODO
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
        Ok(ElectionData{ metadata, atl:vec![], btl:btl.to_btls(), informal:0 })
    }

    fn find_raw_data_file(&self,filename:&str) -> Result<PathBuf,MissingFile> {
        self.finder.find_raw_data_file(filename,&self.archive_location,&self.page_url)
    }
    fn all_electorates(&self) -> Vec<String> {
        self.electorate_to_ecode.keys().cloned().collect()
    }

}


impl ACTDataLoader {

    pub fn new(finder:&FileFinder,year:&'static str,page_url:&'static str) -> anyhow::Result<Self> {
        let archive_location = "ACT/".to_string()+year;
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
                    tickets: vec![]
                })
            }
        }
        Ok(res)
    }
    /// load candidates for a given ecode, and add to appropriate party.
    fn load_candidates(&self,ecode:usize,parties:&mut Vec<Party>) -> anyhow::Result<Vec<Candidate>> {
        let path = self.find_raw_data_file("Candidates.txt")?;
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
                files: vec!["Candidates.txt".to_string(),"Electorates.txt".to_string(),"Groups.txt".to_string()],
                comments: None
            }],
            results: None,
            vacancies: Some(vacancies),
            secondary_vacancies: None,
            excluded: self.excluded_candidates(electorate),
            tie_resolutions : self.ec_decisions(electorate),
        })
    }

    /// Read the official distribution of prefererences, which are excel files in a folder "Distribution Of Preferences".
    /// There may be a sub folder such as in the case of 2020 when Elections ACT redid their distribution after fixing some bugs we pointed out.
    pub fn read_official_dop_transcript(&self,metadata:&ElectionMetadata,sub_folder:Option<&str>) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
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
    let row_index_for_names : u32 = if table1.file_name().unwrap().to_string_lossy().ends_with("xlsx") {2} else {1};
    let mut table1_col_for_candidate = vec![u32::MAX;metadata.candidates.len()];
    let mut table2_col_for_candidate = vec![u32::MAX;metadata.candidates.len()];
    let mut table1_col_for_exhausted_papers : Option<u32> = None;
    let mut table1_col_for_transfer_value : Option<u32> = None;
    let mut table2_col_for_exhausted_votes : Option<u32> = None;
    let mut table2_col_for_loss_by_fraction : Option<u32> = None;
    let mut table2_col_for_remarks : Option<u32> = None;
    let count_column : u32 = 0;
    for col in count_column+1 .. sheet1.width() as u32 {
        if let Some(v) = sheet1.get_value((row_index_for_names,col)) {
            if let Some(s) = v.get_string() {
                for candidate_index in 0..metadata.candidates.len() {
                    if s.contains(&metadata.candidates[candidate_index].name) || s.contains(&metadata.candidates[candidate_index].no_comma_name()){
                        table1_col_for_candidate[candidate_index]=col;
                    }
                }
            }
        }
        if let Some(v) = sheet1.get_value((1,col)) {
            if let Some(s) = v.get_string() {
                if s.contains("Papers Exhausted at Count") { table1_col_for_exhausted_papers=Some(col); }
                else if s.contains("Transfer Value") { table1_col_for_transfer_value=Some(col); }
            }
        }
    }
    if table1_col_for_candidate.contains(&u32::MAX) { return Err(anyhow!("Could not find all candidates in table1"));}
    let table1_col_for_exhausted_papers=table1_col_for_exhausted_papers.ok_or_else(||anyhow!("Could not find exhausted papers column in table 1"))?;
    let table1_col_for_transfer_value=table1_col_for_transfer_value.ok_or_else(||anyhow!("Could not find transfer value column in table 1"))?;
    for col in count_column+1 .. sheet2.width() as u32 {
        if let Some(v) = sheet2.get_value((row_index_for_names,col)) {
            if let Some(s) = v.get_string() {
                for candidate_index in 0..metadata.candidates.len() {
                    if s.contains(&metadata.candidates[candidate_index].name) || s.contains(&metadata.candidates[candidate_index].no_comma_name()){
                        table2_col_for_candidate[candidate_index]=col;
                    }
                }
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
    let mut row_index = row_index_for_names+1;
    let mut paper_row_index = row_index_for_names+1;
    let quota : Option<QuotaInfo<f64>> = None;
    fn parse_transfer_value(f:&DataType) -> Option<f64> { // TV may be a string ratio
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
    fn parse_num_possibly_blank(f:Option<&DataType>) -> f64 {
        f.and_then(|v|v.get_float().or_else(||v.get_string().and_then(|s|s.trim().parse::<f64>().ok()))).unwrap_or(0.0)
    }
    let mut counts = vec![];
    let candidate_of_name = metadata.get_candidate_name_lookup_multiple_ways();
    let mut not_continuing : HashSet<CandidateIndex> = HashSet::default();
    loop {
        let only_1_row = counts.is_empty() && !(metadata.name.year.starts_with("200")||metadata.name.year.starts_with("201"));
        // table 1: row_index contains the delta.
        let transfer_value : Option<f64> = sheet1.get_value((row_index,table1_col_for_transfer_value)).and_then(parse_transfer_value);
        let mut elected : Vec<CandidateIndex> = vec![];
        let mut excluded : Vec<CandidateIndex> = vec![];
        let remarks_1 = sheet2.get_value((row_index,table2_col_for_remarks)).and_then(|v|v.get_string());
        let remarks_2 = if only_1_row {None} else {sheet2.get_value((row_index+1,table2_col_for_remarks)).and_then(|v|v.get_string())};
        for &remarks in [remarks_1,remarks_2].iter().flatten() {
            for remark in remarks.split('.') {
                //println!("Processing remark {}",remark);
                if let Some((name,_)) = remark.split_once(" elected ") {
                    let name=name.trim();
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
            candidate: table2_col_for_candidate.iter().map(|c|parse_num_possibly_blank(sheet2.get_value((row_index,*c)))).collect(),
            exhausted: parse_num_possibly_blank(sheet2.get_value((row_index,table2_col_for_exhausted_votes))),
            rounding: parse_num_possibly_blank(sheet2.get_value((row_index,table2_col_for_loss_by_fraction))),
            set_aside: None,
        });
        let vote_total : Option<PerCandidate<f64>> = if only_1_row { vote_delta.clone() } else {Some(PerCandidate{
            candidate: table2_col_for_candidate.iter().map(|c|parse_num_possibly_blank(sheet2.get_value((row_index+1,*c)))).collect(),
            exhausted: parse_num_possibly_blank(sheet2.get_value((row_index+1,table2_col_for_exhausted_votes))),
            rounding: parse_num_possibly_blank(sheet2.get_value((row_index+1,table2_col_for_loss_by_fraction))),
            set_aside: None,
        })};
        let paper_delta : Option<PerCandidate<isize>>=Some(PerCandidate{
            candidate: table1_col_for_candidate.iter().map(|c|parse_num_possibly_blank(sheet1.get_value((paper_row_index,*c))) as isize).collect(),
            exhausted: parse_num_possibly_blank(sheet1.get_value((paper_row_index,table1_col_for_exhausted_papers))) as isize,
            rounding: 0,
            set_aside: None,
        });
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
            paper_delta
        });
        if sheet2.get_value((row_index,count_column)).is_none()&&sheet2.get_value((row_index+1,count_column)).is_none() {
           return Ok(OfficialDistributionOfPreferencesTranscript{ quota, counts ,missing_negatives_in_papers_delta:true})
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