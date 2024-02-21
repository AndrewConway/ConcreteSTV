// Copyright 2024 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Parse the  .blt format.
//! This is a text file.
//! The first line contains two integers separated by whitespace; the first is the number of candidates, the second the number of seats.
//! The next many lines represent votes, as a list of candidate indices (starting at 1) separated by whitespace and ending with 0 and starting with the number of people who voted that way.
//! The last of these lines just contains "0"
//! Subsequent lines list the "candidate name" "party", one per line.


use std::fs::File;
use std::io::BufRead;
use std::num::ParseIntError;
use std::path::Path;
use anyhow::anyhow;
use stv::ballot_metadata::{Candidate, CandidateIndex, DataSource, ElectionMetadata, ElectionName, NumberOfCandidates, Party, PartyIndex};
use stv::ballot_paper::{BTL, UniqueBTLBuilder};
use stv::election_data::ElectionData;

fn get_line(s:Option<std::io::Result<String>>) -> anyhow::Result<String> { Ok(s.ok_or_else(||anyhow!("No lines in file"))??) }
fn parse_as_ints(s:String) -> Result<Vec<i64>,ParseIntError> {
    let fields : Result<Vec<i64>,ParseIntError> = s.split_whitespace().map(|s|s.parse::<i64>()).collect();
    fields
}

pub fn parse<P:AsRef<Path>>(path:P) -> anyhow::Result<ElectionData> {
    let file = File::open(path.as_ref())?;
    let filename = path.as_ref().file_name().and_then(|s|s.to_str()).unwrap_or("");
    let name : ElectionName = ElectionName {
        year: "".to_string(),
        authority: "".to_string(),
        name: filename.to_string(),
        electorate: "".to_string(),
        modifications: vec![],
        comment: None,
    };
    let source : Vec<DataSource> = vec![ DataSource{url:"".to_string(),files:vec![filename.to_string()], comments: None }];
    let mut btls = UniqueBTLBuilder::default();
    let mut lines =  std::io::BufReader::new(file).lines();
    let firstline = parse_as_ints(get_line(lines.next())?)?;
    if firstline.len()!=2 { return Err(anyhow!("Expecting the first line to have two fields")); }
    let num_candidates = NumberOfCandidates(firstline[0] as usize);
    let vacancies = NumberOfCandidates(firstline[1] as usize);
    let mut excluded : Vec<CandidateIndex> = vec![];
    loop {
        let line = parse_as_ints(get_line(lines.next())?)?;
        if line.len()==0 { return Err(anyhow!("Found blank line")); }
        if line[0]<0 { // remove candidate
            for c in line {
                excluded.push(CandidateIndex((-c-1) as usize))
            }
        } else {
            if line.last().copied()!=Some(0) {  return Err(anyhow!("Found preference line not ending in 0")); }
            if line.len()==1 { break; }
            let candidates : Vec<CandidateIndex> = line[1..line.len()-1].iter().map(|c|CandidateIndex((*c - 1) as usize)).collect();
            btls.add_vote(BTL{candidates,n: line[0] as usize });
        }
    }
    let mut parties : Vec<Party> = vec![];
    let mut candidates : Vec<Candidate> = vec![];
    for _ in 0..num_candidates.0 {
        let candidate_name_line = get_line(lines.next())?;
        let fields : Vec<&str> = candidate_name_line.split('"').collect();
        if fields.len()!=5 { return Err(anyhow!("Expecting candidate name and party in quotes")); }
        let candidate_name = fields[1];
        let party_name = fields[3];
        let party = parties.iter().position(|p|party_name==&p.name);
        let party = party.unwrap_or_else(||{parties.push(Party{
            column_id: "".to_string(),
            name: party_name.to_string(),
            abbreviation: None,
            atl_allowed: false,
            candidates: vec![],
            tickets: vec![],
        }); parties.len()-1});
        parties[party].candidates.push(CandidateIndex(candidates.len()));
        candidates.push(Candidate{
            name: candidate_name.to_string(),
            party: Some(PartyIndex(party)),
            position: None,
            ec_id: None,
        })
    }
    let metadata = ElectionMetadata{
        name,
        candidates,
        parties,
        source,
        results: None,
        vacancies: Some(vacancies),
        enrolment: None,
        secondary_vacancies: None,
        excluded,
        tie_resolutions: Default::default(),
    };
    let data = ElectionData{
        metadata,
        atl: vec![],
        atl_types: vec![],
        atl_transfer_values: vec![],
        btl : btls.to_btls(),
        btl_types: vec![],
        btl_transfer_values: vec![],
        informal: 0,
    };
    Ok(data)
}


