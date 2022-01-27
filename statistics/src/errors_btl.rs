// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use stv::ballot_paper::RawBallotMarkings;
use stv::ballot_pile::BallotPaperCount;
use stv::parse_util::{CanReadRawMarkings, RawDataSource};
use serde::{Serialize,Deserialize};

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct ObviousErrorsInBTLVotes {
    /// repeated_papers[i] is the number of repetitions of markings of i+1. That is, if one ballot paper contains 5 markings of a particular preference, then that counts as 4 repetitions.
    pub repeated : Vec<usize>,
    /// repeated_papers[i] is the number of ballot papers that have more than 1 marking of i+1.
    pub repeated_papers : Vec<BallotPaperCount>,
    /// missing[i] means that i+1 is missing, although i (unless 0) is present exactly once and i+2 is present exactly once (unless i+1 is the last).
    pub missing : Vec<BallotPaperCount>,
    /// ok_up_to[i] means that the numbers 1..i are all present exactly once, and i+1 is not present exactly once.
    pub ok_up_to : Vec<BallotPaperCount>,
}


impl ObviousErrorsInBTLVotes {
    pub fn compute<S:RawDataSource+CanReadRawMarkings>(loader:S,electorate:&str) -> anyhow::Result<Self> {

        let metadata = loader.read_raw_metadata(electorate)?;
        let num_candidates = metadata.candidates.len();
        let mut res = ObviousErrorsInBTLVotes {
            repeated: vec![0;num_candidates],
            repeated_papers: vec![BallotPaperCount(0);num_candidates],
            missing: vec![BallotPaperCount(0);num_candidates],
            ok_up_to: vec![BallotPaperCount(0);num_candidates+1]
        };
        let callback = |markings:&RawBallotMarkings,_meta:&[(&str,&str)]| {
            let mut found = vec![0;num_candidates];
            for &m in markings.btl {
                if let Some(n) = m.as_preference(num_candidates) {
                    let n=n-1; // make 0 based to match arrays.
                    found[n]+=1;
                    if found[n]>1 {
                        res.repeated[n]+=1;
                        if found[n]==2 { res.repeated_papers[n]+=BallotPaperCount(1); }
                    }
                }
            }
            // look for missing
            for i in 0..num_candidates-1 {
                if found[i]==0 && (i==0 || found[i-1]==1) && (i+1==num_candidates || found[i+1]==1) {
                    res.missing[i]+=BallotPaperCount(1);
                }
            }
            // look for ok_up_to
            let mut ok_up_to = 0;
            while ok_up_to<num_candidates && found[ok_up_to]==1 { ok_up_to+=1; }
            res.ok_up_to[ok_up_to]+=BallotPaperCount(1);
        };
        let _metadata = loader.iterate_over_raw_markings(electorate,callback)?;
        Ok(res)
    }
}