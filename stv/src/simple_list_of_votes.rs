// Copyright 2024 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! A simple list of votes

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::num::{ParseIntError};
use std::str::FromStr;
use num::rational::ParseRatioError;
use serde::{Deserialize, Serialize};
use crate::ballot_metadata::CandidateIndex;
use crate::transfer_value::TransferValue;

/// It is possible (but not usual) to include a list of all the votes at every point in a transcript.
/// For non-trivial elections this results in a very large transcript, but it could be useful for countbacks.
///
/// This structure contains a list of votes, ordered by transfer value highest to lowe
#[derive(Clone,Serialize,Deserialize, PartialEq,Debug,Default)]
#[serde(transparent)]
pub struct ListOfVotes {
    pub tvs : Vec<VotesWithGivenTransferValue>
}

impl ListOfVotes {
    pub fn sub<'a>(&'a self,rhs:&'a Self) -> Self {
        let mut by_tv : HashMap<TransferValue,HashMap<&'a Vec<CandidateIndex>,isize>> = HashMap::new();
        let mut add = |mul:isize,what:&'a Self| { // add mul*what to by_tv
            for vtv in &what.tvs {
                let tv = by_tv.entry(vtv.tv.clone()).or_insert_with(||HashMap::new());
                for v in &vtv.votes {
                    *tv.entry(&v.candidates).or_insert(0)+=mul*v.n;
                }
            }
        };
        add(1,self);
        add(-1,rhs);
        let tvs = by_tv.into_iter().map(|(tv,vs)|VotesWithGivenTransferValue{ tv, votes: vs.into_iter().filter(|(_,n)|*n!=0).map(|(v,n)|Vote{n,candidates:v.clone()}).collect() }).filter(|v|!v.votes.is_empty()).collect();
        ListOfVotes{tvs}
    }
}
impl Display for ListOfVotes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut empty = true;
        let mut last_tv : Option<&TransferValue> = None;
        for tv in &self.tvs {
            if !tv.votes.is_empty() {
                let is_same_tv_as_last = if let Some(last) = last_tv { last.eq(&tv.tv) } else { tv.tv.is_one() };
                if !is_same_tv_as_last {
                    last_tv=Some(&tv.tv);
                    if empty {empty=false;} else { write!(f,";")?; }
                    write!(f,"TV:{}",tv.tv)?;
                }
                for v in &tv.votes {
                    if empty {empty=false;} else { write!(f,";")?; }
                    write!(f,"{}",v)?;
                }
            }
        }
        Ok(())
    }
}


impl FromStr for ListOfVotes {
    type Err = ParseVoteError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut res : ListOfVotes = ListOfVotes::default();
        for vote in s.split(';') {
            if vote.starts_with("TV:") {
                res.tvs.push(VotesWithGivenTransferValue{
                    tv: vote[3..].parse::<TransferValue>().map_err(|e|ParseVoteError::NotTransferValue(e))?,
                    votes: vec![],
                });
            } else {
                let vote : Vote = vote.parse()?;
                if res.tvs.is_empty() {
                    res.tvs.push(VotesWithGivenTransferValue{ tv: TransferValue::one(), votes: vec![]})
                };
                // previous statement ensured that tvs is non-empty so last_mut won't return None, so unwrap() is safe.
                res.tvs.last_mut().unwrap().votes.push(vote);
            }
        }
        Ok(res)
    }
}

#[derive(Clone,Serialize,Deserialize, PartialEq,Debug)]
pub struct VotesWithGivenTransferValue {
    pub tv : TransferValue,
    pub votes : Vec<Vote>
}

impl Display for VotesWithGivenTransferValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}*{}",self.tv,self.votes.iter().map(|s|s.to_string()).collect::<Vec<_>>().join(","))
    }
}


impl FromStr for VotesWithGivenTransferValue {
    type Err = ParseVoteError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut res : ListOfVotes = s.parse()?;
        match res.tvs.len() {
            0 => Ok(VotesWithGivenTransferValue{tv:TransferValue::one(),votes:vec![]}),
            1 => Ok(res.tvs.swap_remove(0)),
            _ => Err(ParseVoteError::MultipleTransferValues)
        }
    }
}

#[derive(Clone,Serialize,Deserialize, PartialEq,Debug)]
pub struct Vote {
    /// The number of voters who voted this way. May be negative as deltas may be negative.
    pub n : isize,
    /// prefs[0] is the first preferenced candidate.
    pub candidates : Vec<CandidateIndex>,
}


impl Display for Vote {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}*{}",self.n,self.candidates.iter().map(|s|s.to_string()).collect::<Vec<_>>().join(";"))
    }
}

/// A Vote represented as a string should be n*p1,p2,p3 where n is an unsigned integer and p1,p2,p3 are candidate indices (integers from 0 to 1 fewer than the number of candidates)
///
/// A list of votes should be v1;v2;v3;TV:tv1;v4;v5;TV:tv2;v6;v7 where v1..v7 are votes as above, and tv1 and tv2 are transfer values.
/// A transfer value means all votes after have that transfer value. A TV of 1 is assumed at the start.
/// This works for votes with or without transfer values.
#[derive(Clone,Debug)]
pub enum ParseVoteError {
    NoTimes,
    NotInteger(ParseIntError),
    NotTransferValue(ParseRatioError),
    MultipleTransferValues,
}
impl FromStr for Vote {
    type Err = ParseVoteError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (n,prefs) = s.split_once('*').ok_or(ParseVoteError::NoTimes)?;
        let n : isize = n.parse().map_err(|e|ParseVoteError::NotInteger(e))?;
        let prefs : Result<Vec<CandidateIndex>,ParseIntError> = prefs.split(',').map(|s|s.parse::<CandidateIndex>()).collect();
        let candidates = prefs.map_err(|e|ParseVoteError::NotInteger(e))?;
        Ok(Vote { n, candidates })
    }
}

