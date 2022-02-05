// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! 2013 is significantly different to later years due to the presence of tickets,
//! and the AEC used significantly different file formats.
//! So I have made the parsing code for them different.



use stv::parse_util::{CandidateAndGroupInformationBuilder, skip_first_line_of_file, GroupBuilder};
use std::path::Path;
use stv::ballot_metadata::{Candidate, PartyIndex, CandidateIndex, ElectionMetadata};
use std::collections::HashMap;
use stv::ballot_paper::{ATL, BTL, RawBallotMarking, RawBallotMarkings, UniqueBTLBuilder};
use std::iter::FromIterator;
use anyhow::anyhow;
use std::fs::File;
use std::io::{Read};

/// Read from a file "SenateGroupVotingTicketsDownload which defines candidates, parties and tickets.
/// Used in 2013.
pub(crate) fn read_from_senate_group_voting_tickets_download_file2013(builder: &mut CandidateAndGroupInformationBuilder, path:&Path, state:&str) -> anyhow::Result<()> {
    fn add_ticket(builder: &mut CandidateAndGroupInformationBuilder,current_ticket : &Vec<usize>,current_ticket_owner:&Option<String>) {
        let party = builder.group_from_group_id(current_ticket_owner.as_ref().unwrap()).unwrap();
        let mut real_ticket = vec![CandidateIndex(999999);builder.candidates.len()];
        for (candidate_index,preference_no) in current_ticket.iter().enumerate() { real_ticket[preference_no-1]=CandidateIndex(candidate_index);}
        builder.parties[party.0].tickets.push(real_ticket);
    }
    let mut rdr = csv::Reader::from_reader(skip_first_line_of_file(path)?);
    let mut first_party_ticket_owner : Option<String> = None; // first party ticket indicates that this is the first time candidates/parties seen.
    let mut candidate_id_to_index : HashMap<String,CandidateIndex> = HashMap::default();
    let mut current_ticket : Vec<usize> = vec![]; // vec of preference numbers. current_ticket[c] is the mark the owning party wants for candidate c.
    let mut current_ticket_owner : Option<String> = None;
    let mut current_ticket_no : usize = 0;
    for result in rdr.records() {
        let record = result?;
        if state==&record[0] { // right state
            let party_ticket_owner = &record[3]; // something like A, B, or UG
            if first_party_ticket_owner == None { first_party_ticket_owner = Some(party_ticket_owner.to_string()) };
            let ticket_no = record[4].parse::<usize>()?; // 1 means first

            // see if a ticket has ended
            if current_ticket_owner.is_none() || current_ticket_owner.as_ref().unwrap()!=party_ticket_owner || current_ticket_no!=ticket_no {
                if current_ticket_owner.is_some() {
                    add_ticket(builder,&current_ticket,&current_ticket_owner);
                    current_ticket.clear();
                }
                current_ticket_owner=Some(party_ticket_owner.to_string());
                current_ticket_no=ticket_no;
            }


            let group_id = &record[6]; // something like A, B, or UG
            let candidate_id = &record[5]; // something like 32847
            let position_in_ticket = record[9].parse::<usize>()?; // 1 means first
            let preference_no = record[12].parse::<usize>()?; // the preference given to said candidate by party_ticket_owner.
            if first_party_ticket_owner.as_ref().unwrap() == party_ticket_owner && ticket_no == 1 { // Defining candidates/parties.
                if builder.parties.len() == 0 || &builder.parties[builder.parties.len() - 1].group_id != group_id { // define a new party.
                    let party_name = &record[11];
                    let party_abreviation = &record[10];
                    let abbreviation = if party_abreviation.is_empty() { None } else { Some(party_abreviation.to_string()) };
                    builder.parties.push(GroupBuilder { name: party_name.to_string(), abbreviation, group_id: group_id.to_string(), ticket_id: None, tickets: vec![] });
                }
                if position_in_ticket != 0 { // real candidate.
                    candidate_id_to_index.insert(candidate_id.to_string(), CandidateIndex(builder.candidates.len()));
                    let name = record[7].to_string() + ", " + &record[8];
                    builder.candidates.push(Candidate {
                        name,
                        ec_id: Some(candidate_id.to_string()),
                        party: Some(PartyIndex(builder.parties.len() - 1)),
                        position: Some(position_in_ticket)
                    })
                }
            }
            current_ticket.push(preference_no);
        }
    }
    add_ticket(builder,&current_ticket,&current_ticket_owner);
    Ok(())
}

/// Where the odd ones out go in the case where the number of ticket votes is not divisible by the num of tickets.
/// Parties can have multiple tickets to spread their votes over.
///
/// If there are multiple tickets, and the number of ticket votes is not divisible by the
/// number of tickets, then one or more tickets gets one more instance than the others.
/// This value determines which get the extra.
///
/// If there are 2 tickets and 1 excess vote, then 0 here means the first ticket gets the excess, 1 means the second.
/// If there are 3 tickets then 0 means the first ticket is special, 1 the second, 2 the third. If there is 1 excess the special ticket gets it, if there are 2, the special ticket does not get it.
/// No more than 3 tickets per party has been observered in the elections so far supported.
fn get_deduced_aec_ticket_splits2013(state:&str) -> anyhow::Result<HashMap<&'static str,usize>> {
    match state {
        "VIC" => Ok(HashMap::<_, _>::from_iter([("E", 0), ("F", 0), ("L", 0), ("AF", 0), ("AI", 0), ("AK", 2)])),
        "NT" => Ok(HashMap::<_, _>::from_iter([("H", 1)])),
        "SA" => Ok(HashMap::<_, _>::from_iter([("A", 1), ("V",0)])),
        "WA" => Ok(HashMap::<_, _>::from_iter([("E", 2), ("P",0)])),
        "NSW" => Ok(HashMap::<_, _>::from_iter([("AI", 0), ("AR",0)])),
        "ACT" => Ok(HashMap::<_, _>::default()),
        "TAS" => Ok(HashMap::<_, _>::from_iter([("S", 0)])),
        "QLD" => Ok(HashMap::<_, _>::from_iter([("H", 0), ("S",1), ("Y",0)])),
        _ => Err(anyhow!("Not a valid state : {}",state)),
    }
}

/// Read the file SenateUseOfGvtByGroupDownload for 2013 to get the number of voters voting for each ticket.
/// Nasty hack - return as BTLs, as ATL has different meaning to tickets, and don't want to complicate ElectionData with ticket votes.
pub(crate) fn read_ticket_votes2013(metadata:&ElectionMetadata,path:&Path, state:&str) -> anyhow::Result<Vec<ATL>> {
    let splits = get_deduced_aec_ticket_splits2013(state)?;
    let mut rdr = csv::Reader::from_reader(skip_first_line_of_file(path)?);
    let mut res = vec![];
    for result in rdr.records() {
        let record = result?;
        if state == &record[0] { // right state
            let group_id = &record[1];
            let ticket_votes = record[4].parse::<usize>()?;
            if ticket_votes>0 {
                let party_index = PartyIndex(metadata.parties.iter().position(|p|&p.column_id==group_id).unwrap());
                let num_tickets = metadata.party(party_index).tickets.len();
                if num_tickets==0 { return Err(anyhow!("Group {} has {} votes but not tickets",group_id,ticket_votes));}
                let portion = ticket_votes/num_tickets;
                let excess = ticket_votes%num_tickets;
                let choice = if excess>0 { match splits.get(&group_id).cloned() { Some(u)=>u, None => return Err(anyhow!("Group {} has an unspecified rounding choice between 0 and {}",group_id,num_tickets-1))} } else {0};
                for i in 0..num_tickets {
                    let extra = if excess==0 {0} else {match num_tickets { // whether there should be a rounding vote added to this ticket.
                        2 => if i==choice {1} else {0},
                        3 => if (i==choice)==(excess==1) {1} else {0},
                        _ => 0,
                    }};
                    res.push(ATL{ parties:vec![party_index] , n: portion+extra, ticket_index: Some(i) });
                }
            }
        }
    }
    Ok(res)
}

pub(crate) fn read_btl_votes2013(metadata:&ElectionMetadata,path:&Path,min_btl_prefs_needed:usize) -> anyhow::Result<(Vec<BTL>,usize)> {
    // let mut res = vec![];
    let mut builder = UniqueBTLBuilder::default();
    let mut informal : usize = 0;
    let candidate_of_id : HashMap<String,CandidateIndex> = metadata.get_candidate_ec_id_lookup();
    let mut zipfile = zip::ZipArchive::new(File::open(path)?)?;
    {
        for i in 0..zipfile.len() {
            let mut file = zipfile.by_index(i)?;
            if file.name().ends_with(".csv") {
                let mut dummy_buf = [0u8];
                while dummy_buf[0]!=b'\n' {
                    // print!("{}",dummy_buf[0] as char);
                    file.read_exact(&mut dummy_buf)?;
                } // quick and dirty way to skip the first line which the csv parser chokes on.
                let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(file); // needs to be flexible as the headers line has a helpful fifth blank field.
                // println!("{:#?}",rdr.headers()?);
                let mut last : Option<(usize,usize)> = None;
                let mut btl_markings = vec![RawBallotMarking::Blank;metadata.candidates.len()];
                for result in rdr.records() {
                    let record = result?;
                    let candidate = *candidate_of_id.get(&record[0]).unwrap();
                    let batch = record[2].parse::<usize>()?;
                    let paper = record[3].parse::<usize>()?;
                    if last!=Some((batch,paper)) {
                        if last.is_some() { // save existing paper.
                            if let Some(btl) = (RawBallotMarkings{ atl: &[], btl: &btl_markings, atl_parties: &[] }).interpret_vote_as_btl(min_btl_prefs_needed) {
                                builder.add_vote(btl);
                            } else { informal+=1;}
                            btl_markings.clear();
                            btl_markings.resize(metadata.candidates.len(),RawBallotMarking::Blank);
                        }
                        last=Some((batch,paper));
                    }
                    let mark = record[1].trim();
                    let preference = if let Ok(preference) = mark.parse::<u16>() { RawBallotMarking::Number(preference) }
                    else {
                        match mark {
                            "" => RawBallotMarking::Blank,
                            "??" => RawBallotMarking::Other,
                            "*" => RawBallotMarking::Other, // in 2013, ticks and crosses are not considered ones when BTL, although they are ATL.
                            "/" => RawBallotMarking::Other, // in 2013, ticks and crosses are not considered ones when BTL, although they are ATL.
                            _ => {
                                /* There are some counts like 14. We could try to save them... but then we wouldn't match the AEC counts.
                                let start_with_number : String = mark.chars().take_while(|c|c.is_numeric()).collect();
                                if start_with_number.len()>0 {
                                    if let Ok(preference) = start_with_number.parse::<u16>() { RawBallotMarking::Number(preference) } else {
                                        println!("Preference mark <{}>",mark); RawBallotMarking::Other
                                    }
                                } else {
                                    println!("Preference mark <{}>",mark); RawBallotMarking::Other
                                }*/
                                println!("Preference mark <{}>",mark); RawBallotMarking::Other
                            },
                        }};
                    btl_markings[candidate.0]=preference;
                }
                // save existing paper.
                if let Some(btl) = (RawBallotMarkings{ atl: &[], btl: &btl_markings, atl_parties: &[] }).interpret_vote_as_btl(min_btl_prefs_needed) {
                    builder.add_vote(btl);
                } else { informal+=1;}
                return Ok((builder.to_btls(),informal))
            }
        }
        Err(anyhow!("Could not find file in zipfile for {}",&metadata.name.electorate))
    }
}