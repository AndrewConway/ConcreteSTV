// Copyright 2026 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Somewhat similar (but generally much smaller) than simple_list_of_votes::ListOfVotes
//! Describes now many votes with different weight sources are present.

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;
use num::rational::ParseRatioError;
use serde::{Deserialize, Serialize};
use crate::ballot_metadata::CandidateIndex;
use crate::ballot_pile::BallotPaperCount;

#[derive(Clone,Serialize,Deserialize, PartialEq,Debug,Default)]
#[serde(transparent)]
pub struct ListOfMeekVoteSources {
    pub sources : Vec<MeekVotesWithSameKeepValueRoute>,
}

impl Display for ListOfMeekVoteSources {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.sources.iter().map(|s|s.to_string()).collect::<Vec<_>>().join(";"))
    }
}

impl FromStr for ListOfMeekVoteSources {
    type Err = ParseMeekVoteSourceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ListOfMeekVoteSources{sources: if s.is_empty() { vec![] } else {
            let sources: Result<Vec<MeekVotesWithSameKeepValueRoute>, ParseMeekVoteSourceError> = s.split(';').map(|s| s.parse::<MeekVotesWithSameKeepValueRoute>()).collect();
            sources?
        }})
    }
}


#[derive(Clone,Serialize,Deserialize, PartialEq,Debug)]
pub struct MeekVotesWithSameKeepValueRoute {
    pub count : BallotPaperCount,
    /// The Meek weight of each paper. Product of the keep values of elements of the route.
    pub weight : String,
    /// an ordered list of the keep values the vote went through.
    pub route : Vec<CandidateIndex>,
}

#[derive(Clone,Debug)]
pub enum ParseMeekVoteSourceError {
    NoTimes,
    NoAt,
    NotInteger(ParseIntError),
    NotTransferValue(ParseRatioError),
    MultipleTransferValues,
}
impl Display for MeekVotesWithSameKeepValueRoute {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}*{}@{}",self.weight,self.count,self.route.iter().map(|s|s.to_string()).collect::<Vec<_>>().join("&"))
    }
}

impl FromStr for MeekVotesWithSameKeepValueRoute {
    type Err = ParseMeekVoteSourceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (weight,after_star) = s.split_once('*').ok_or(ParseMeekVoteSourceError::NoTimes)?;
        let weight = weight.to_string();
        let (count,after_at) = after_star.split_once('@').ok_or(ParseMeekVoteSourceError::NoAt)?;
        let count : BallotPaperCount = FromStr::from_str(count).map_err(|e|ParseMeekVoteSourceError::NotInteger(e))?;
        let route : Vec<CandidateIndex> = if after_at.is_empty() { vec![] } else {
            let route : Result<Vec<CandidateIndex>,ParseMeekVoteSourceError> = after_at.split('&').map(|s|s.parse::<usize>().map(|c|CandidateIndex(c)).map_err(|e|ParseMeekVoteSourceError::NotInteger(e))).collect();
            route?
        };
        Ok(MeekVotesWithSameKeepValueRoute{count,weight,route})
    }
}

/// Used to build up a ListOfMeekVoteSources from individual votes
#[derive(Clone,Debug,Default)]
pub struct ListOfMeekVoteSourcesBuilder {
    sources : HashMap<Vec<CandidateIndex>,(String,BallotPaperCount)>,
}

impl ListOfMeekVoteSourcesBuilder {
    /// add an entry.
    pub fn add(&mut self,count:BallotPaperCount,source:&Vec<CandidateIndex>,weight:impl FnOnce()->String) {
        // It would be nice to do this with the hashmap entry_ref RFC. However for practical cases most uses will have the first case below work most of the time so the difference is not high.
        if let Some(v) = self.sources.get_mut(source) {
            v.1 += count;
        } else {
            self.sources.insert(source.clone(),(weight(),count));
        }
    }

    pub fn into(self) -> ListOfMeekVoteSources {
        let sources = self.sources.into_iter().map(|(route,(weight,count))| MeekVotesWithSameKeepValueRoute{count,weight,route}).collect();
        ListOfMeekVoteSources{ sources }
    }
}



