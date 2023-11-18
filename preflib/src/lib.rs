// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Parsing preference data from https://www.preflib.org/




use std::fs::File;
use std::io::BufRead;
use std::path::Path;
use anyhow::anyhow;
use stv::ballot_metadata::{Candidate, CandidateIndex, DataSource, ElectionMetadata, ElectionName};
use stv::ballot_paper::BTL;
use stv::election_data::ElectionData;

pub fn parse<P:AsRef<Path>>(path:P) -> anyhow::Result<ElectionData> {
    let file = File::open(path)?;
    let mut candidates : Vec<Candidate> = vec![];
    let mut name : ElectionName = ElectionName {
        year: "".to_string(),
        authority: "".to_string(),
        name: "".to_string(),
        electorate: "".to_string(),
        modifications: vec![],
        comment: None,
    };
    let mut source : Vec<DataSource> = vec![];
    let mut btl : Vec<BTL> = vec![];
    for line in std::io::BufReader::new(file).lines() {
        let line = line?;
        if line.starts_with("#") { // metadata
            if let Some((metadata_name,metadata_value)) = line[1..].split_once(':') {
                let metadata_value = metadata_value.trim();
                match metadata_name.trim() {
                    "FILE NAME" => { source.push(DataSource{url:format!("https://www.preflib.org/static/data/irish/{}",metadata_value),files:vec![metadata_value.to_string()],comments:None});}
                    "TITLE" => {name.name=metadata_value.to_string()}
                    "DESCRIPTION" => { if !metadata_value.is_empty() { name.comment=Some(metadata_value.to_string());}}
                    "DATA TYPE" => {
                        if metadata_value=="soc" || metadata_value=="soi" {} else {
                            return Err(anyhow!("Can only handle formats soc or soi, got {}",metadata_value));
                        }
                    }
                    "NUMBER ALTERNATIVES" => {
                        let n : usize = metadata_value.parse()?;
                        candidates.resize_with(n,||Candidate::from_name("unspecified"))
                    }
                    s if s.starts_with("ALTERNATIVE NAME") => { // #ALTERNATIVE NAME n : name
                        let n : usize = s.trim_start_matches("ALTERNATIVE NAME").trim_start().parse()?;
                        if n==0 { return Err(anyhow!("NUMBER ALTERNATIVES 0 is not understood"))}
                        if candidates.len()<n { candidates.resize_with(n,||Candidate::from_name("unspecified"))}
                        candidates[n-1].name=metadata_value.to_string();
                    }
                    _ => {}
                }
            } else {
                return Err(anyhow!("Metadata line without colon : {}",line));
            }
        } else if line.is_empty() {}
        else { // preferences line
            if let Some((n,prefs)) = line.split_once(':') {
                let num_candidates = candidates.len();
                let n : usize = n.trim().parse()?;
                let mut candidates : Vec<CandidateIndex> = vec![];
                for candidate in prefs.trim().split(',') {
                    let candidate : usize = candidate.trim().parse()?;
                    if candidate<1 || candidate>num_candidates { return Err(anyhow!("Expecting candidate index between 1 and {}, got {}",num_candidates,candidate)) }
                    candidates.push(CandidateIndex(candidate-1));
                }
                btl.push(BTL{candidates,n});
            } else {
                return Err(anyhow!("Expecting line of the form n : <preference list> got {}",line))
            }
        }
    }
    let metadata = ElectionMetadata{
        name,
        candidates,
        parties: vec![],
        source,
        results: None,
        vacancies: None,
        enrolment: None,
        secondary_vacancies: None,
        excluded: vec![],
        tie_resolutions: Default::default(),
    };
    let data = ElectionData{
        metadata,
        atl: vec![],
        atl_types: vec![],
        atl_transfer_values: vec![],
        btl,
        btl_types: vec![],
        btl_transfer_values: vec![],
        informal: 0,
    };
    Ok(data)
}