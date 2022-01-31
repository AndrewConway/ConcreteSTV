// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Find raw votes in the official results that match your vote - see if it is really there!
//! Actually, look for similar things as you or the EC may have made an error.
//! Note that tests for this are in the statistics module (as they require some actual data)


use std::collections::HashMap;
use crate::ballot_paper::{parse_marking, RawBallotMarking, RawBallotMarkings};
use crate::parse_util::CanReadRawMarkings;
use serde::{Serialize, Deserialize};

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct FindVoteHit {
    pub metadata : HashMap<String,String>,
    pub votes : String, // comma separated list of votes.
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct SearchMatchesWithSameScore {
    pub score : usize,
    pub hits : Vec<FindVoteHit>,
    /// the number of hits not mentioned here.
    pub truncated : usize,
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct FindMyVoteResult {
    /// best matches, highest score earliest.
    pub best : Vec<SearchMatchesWithSameScore>,
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct FindMyVoteQuery {
    /// a comma separated string of preferences
    pub query : String,
    pub blank_matches_anything : bool,
}

impl FindMyVoteQuery {
    pub fn parse_query(&self) -> Vec<RawBallotMarking> {
        self.query.split(",").map(parse_marking).collect()
    }
}

const MAX_SCORES_WANTED: usize = 3;
const MAX_HITS_PER_SCORE_WANTED: usize = 10;

impl FindVoteHit {
    fn new(markings:&RawBallotMarkings,meta:&[(&str,&str)]) -> Self {
        let votes = markings.btl.iter().map(|m|m.to_string()).collect::<Vec<_>>().join(",");
        let mut metadata = HashMap::default();
        for (key,value) in meta {
            metadata.insert(key.to_string(),value.to_string());
        }
        FindVoteHit{ metadata, votes }
    }
}
impl FindMyVoteResult {
    /// find the appropriate place to insert a given scored hit.
    fn find_where_to_insert(&mut self, score:usize) -> Option<&mut Vec<FindVoteHit>> {
        let mut skip_over = 0;
        while skip_over<self.best.len() && self.best[skip_over].score>score { skip_over+=1; }
        // now the first skip_over are bigger than this score.
        if skip_over>= MAX_SCORES_WANTED {
            None // we have plenty of better scores already.
        } else {
            if skip_over<self.best.len() && self.best[skip_over].score==score {
                // found another one.
                if self.best[skip_over].hits.len()==MAX_HITS_PER_SCORE_WANTED { // already have plenty with this score
                    self.best[skip_over].truncated+=1;
                    None
                } else {
                    Some(&mut self.best[skip_over].hits)
                }
            } else { // our score is better than this score.
                self.best.insert(skip_over,SearchMatchesWithSameScore{score,hits:vec![],truncated:0});
                self.best.truncate(MAX_SCORES_WANTED); // get rid of extra
                Some(&mut self.best[skip_over].hits)
            }
        }
    }

    pub fn compute<S:CanReadRawMarkings>(loader:&S,electorate:&str,query:&FindMyVoteQuery) -> anyhow::Result<Self> {
        let mut res = FindMyVoteResult { best: vec![] };
        let my_query = query.parse_query();
        let callback = |markings:&RawBallotMarkings,meta:&[(&str,&str)]| {
            let mut score : usize = 0;
            for i in 0..markings.btl.len().min(my_query.len()) {
                let me = my_query[i];
                let them = markings.btl[i];
                if me==them || (query.blank_matches_anything && me==RawBallotMarking::Blank) { score+=1; }
            }
            if let Some(destination) = res.find_where_to_insert(score) {
                destination.push(FindVoteHit::new(markings,meta));
            }
        };
        let _metadata = loader.iterate_over_raw_markings(electorate,callback)?;
        Ok(res)
    }
}