// Copyright 2022 Andrew Conway.
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
use stv::parse_util::{FileFinder, KnowsAboutRawMarkings, MissingFile, RawDataSource, read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions};
use stv::tie_resolution::TieResolutionsMadeByEC;
use crate::Vic2018LegislativeCouncil;
use calamine::{DataType, open_workbook_auto};
use stv::ballot_paper::{parse_marking, RawBallotMarking, RawBallotMarkings, UniqueBTLBuilder};
use stv::ballot_pile::BallotPaperCount;
use stv::distribution_of_preferences_transcript::{CountIndex, PerCandidate, QuotaInfo};
use stv::signed_version::SignedVersion;

pub fn get_vic_data_loader_2014(finder:&FileFinder) -> anyhow::Result<VicDataLoader> {
    VicDataLoader::new(finder,"2014","https://www.vec.vic.gov.au/results/state-election-results/2014-state-election") // Note - not all needed files are public. Ask the VEC nicely if you want them, and maybe they will oblige.
}

pub struct NSWLGEDataSource {}

impl ElectionDataSource for NSWLGEDataSource {
    fn name(&self) -> Cow<'static, str> { "Victorian Upper House".into() }
    fn ec_name(&self) -> Cow<'static, str> { "Victorian Electoral Commission".into() }
    fn ec_url(&self) -> Cow<'static, str> { "https://www.vec.vic.gov.au/".into() }
    fn years(&self) -> Vec<String> { vec!["2014".to_string()] }
    fn get_loader_for_year(&self,year: &str,finder:&FileFinder) -> anyhow::Result<Box<dyn RawDataSource+Send+Sync>> {
        match year {
            "2014" => Ok(Box::new(get_vic_data_loader_2014(finder)?)),
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
        vec![
            "Eastern Metropolitan Region".to_string(),
            "Eastern Victoria Region".to_string(),
            "Northern Metropolitan Region".to_string(),
            "Northern Victoria Region".to_string(),
            "South-Eastern Metropolitan Region".to_string(),
            "Southern Metropolitan Region".to_string(),
            "Western Metropolitan Region".to_string(),
            "Western Victoria Region".to_string(),
        ]
    }
    fn read_raw_data(&self, electorate: &str) -> anyhow::Result<ElectionData> {
        let metadata = self.read_raw_metadata(electorate)?;
        println!("{:?}",metadata);
        let filename = format!("received_from_ec/Ballot Paper Details - {}.csv",electorate);
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
                        builder.add_vote(btl);
                    } else { informal+=1;}
                }
            }
        }
        Ok(ElectionData{
            metadata,
            atl: vec![],
            atl_types: vec![],
            btl: builder.to_btls(),
            btl_types: vec![],
            informal
        })
    }

    fn read_raw_data_best_quality(&self, electorate: &str) -> anyhow::Result<ElectionData> {
        read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<Vic2018LegislativeCouncil,Self>(self, electorate)
    }

    /// Get the metadata from the file like south-easternmetropolitanregionvotesreceived.xls
    fn read_raw_metadata(&self,electorate:&str) -> anyhow::Result<ElectionMetadata> {
        let filename = format!("{}votesreceived.xls",Self::region_human_name_to_computer_name(electorate));
        let path = self.find_raw_data_file(&filename)?;
        use calamine::Reader;
        let mut workbook1 = open_workbook_auto(&path)?;
        let sheet1 = workbook1.worksheet_range_at(0).ok_or_else(||anyhow!("No sheets in {}",path.to_string_lossy()))??;
        // find the row of headings. Probably 7...
        let is_headings_row = |row:usize|{
            if let Some(v) = sheet1.get_value((row as u32,0)) {
                if let Some(s) = v.get_string() {
                    s.ends_with(" District")
                } else {false}
            } else {false}
        };
        let headings_row = (0..sheet1.height()).into_iter().find(|&row|is_headings_row(row)).ok_or_else(||anyhow!("Could not find headlines in {}",path.to_string_lossy()))?;
        let mut candidates = vec![];
        let mut parties = vec![];
        let mut current_party_name = "".to_string();
        let mut position_in_party = 0;
        for col in 1..sheet1.width() {
            if let Some(v) = sheet1.get_value((headings_row as u32,col as u32)) {
                if let Some(s) = v.get_string() {
                    let lines : Vec<&str> = s.split('\n').collect();
                    if lines.len()==3 || (lines.len()==2 && lines[0].starts_with("Group ")) {
                        let last_line = lines[lines.len()-1];
                        if last_line!="ATL" && last_line!="TOTAL" {
                            let party_line = lines[lines.len()-1];
                            if parties.is_empty() || current_party_name!=lines[1] {
                                current_party_name=party_line.to_string();
                                position_in_party=1;
                                parties.push(Party{
                                    column_id: lines[0].to_string(),
                                    name: if lines.len()==3 { party_line.to_string() } else {"".to_string()},
                                    abbreviation: None,
                                    atl_allowed: true,
                                    candidates: vec![],
                                    tickets: vec![]
                                })
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
                rules_used: Some("VEC2014".into()),
                rules_recommended: None,
                comment: Some("Untested.".into()),
                reports: vec![],
            },
            _ => AssociatedRules { rules_used: None, rules_recommended: None, comment: None, reports: vec![] },
        }
    }

    fn read_official_dop_transcript(&self, metadata: &ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        let filename = format!("{}distributions.xls",Self::region_human_name_to_computer_name(&metadata.name.electorate));
        let path = self.find_raw_data_file(&filename)?;
        use calamine::Reader;
        let mut workbook1 = open_workbook_auto(&path)?;
        let sheet1 = workbook1.worksheet_range_at(0).ok_or_else(||anyhow!("No sheets in {}",path.to_string_lossy()))??;
        let format = DOPFileFormat::new(&sheet1,metadata).context(path.to_string_lossy().to_string())?;
        format.parse_spreadsheet(&sheet1,metadata).context(path.to_string_lossy().to_string())
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
        let filename = format!("{}distributions.xls",Self::region_human_name_to_computer_name(&metadata.name.electorate));
        let path = self.find_raw_data_file(&filename)?;
        use calamine::Reader;
        let mut workbook1 = open_workbook_auto(&path)?;
        let sheet1 = workbook1.worksheet_range_at(0).ok_or_else(||anyhow!("No sheets in {}",path.to_string_lossy()))??;
        let format = DOPFileFormat::new(&sheet1,metadata).context(path.to_string_lossy().to_string())?;
        format.reorder_candidates_to_match_this(metadata);
        Ok(())
    }
    /// Convert a name like "South-Eastern Metropolitan Region" to "south-easternmetropolitanregion" as used in file names.
    fn region_human_name_to_computer_name(electorate:&str) -> String {
        electorate.chars().filter(|c|!c.is_whitespace()).map(|c|c.to_ascii_lowercase()).collect()
    }

}