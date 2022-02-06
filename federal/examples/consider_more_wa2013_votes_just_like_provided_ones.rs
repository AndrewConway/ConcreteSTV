// Copyright 2021-2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! This analyses the WA 2013 federal election controversy in a similar way to the NSWEC analysed the NSW LGE 2021 election missing votes.

use std::collections::HashMap;
use std::io::Write;
use std::thread;
use federal::parse::get_federal_data_loader_2013;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use federal::FederalRulesUsed2013;
use stv::compare_transcripts::{DeltasInCandidateLists, DifferentCandidateLists, pretty_print_candidate_list};
use stv::election_data::ElectionData;
use stv::monte_carlo::SampleWithReplacement;
use stv::parse_util::{FileFinder, RawDataSource};

const WRITE_CHARACTER_PER_RUN:bool = true;

/// Do an analysis similar to the (dubious) analysis at https://elections.nsw.gov.au/NSWEC/media/NSWEC/LGE21/iVote-Assessment-Methodology.pdf except for federal senate, WA 2013.
fn main() -> anyhow::Result<()> {
    let loader = get_federal_data_loader_2013(&FileFinder::find_ec_data_repository());
    //let mut rng = ChaCha20Rng::from_entropy();
    let num_runs = 1000;
    let electorate = "WA";
    let data : ElectionData = loader.read_raw_data_best_quality(electorate)?;
    let num_elements_to_add = 1370;
    let sampler = build_sampler(&data);
    let mut num_same = 0;
    let mut num_others: HashMap<DeltasInCandidateLists,usize> = Default::default();
    //let res = run_elections(&data,&sampler,num_elements_to_add,num_runs,&mut rng);
    let res = run_lots_threads(&data,&sampler,num_elements_to_add,num_runs);
    for diff in res {
        if diff.is_empty() { num_same+=1 } else { *num_others.entry(diff).or_insert(0)+=1; }
    }
    println!("\nElectorate {} adding {} same as official {} different {}",electorate,num_elements_to_add,num_same,num_runs-num_same);
    for (result,num) in num_others {
        println!("-{}, +{} : {}",pretty_print_candidate_list(&result.list1only,&data.metadata),pretty_print_candidate_list(&result.list2only,&data.metadata),num);
    }
    Ok(())
}

fn run_lots_threads(data:&ElectionData, sampler:&SampleWithReplacement<usize>, num_elements_to_add:usize, num_runs:usize) -> Vec<DeltasInCandidateLists> {
    let num_threads = 8;
    let mut handles = vec![];
    for thread_no in 0..num_threads {
        let data = data.clone();
        let sampler = sampler.clone();
        let num_to_do = num_runs/num_threads+(if num_runs%num_threads>thread_no {1} else {0});
        let handle = thread::spawn(move || {
            let mut rng = ChaCha20Rng::seed_from_u64(thread_no as u64);
            run_elections(&data,&sampler,num_elements_to_add,num_to_do,&mut rng)
        });
        handles.push(handle);
    }
    let res = handles.into_iter().map(|h|h.join().unwrap()).flatten().collect::<Vec<_>>();
    res
}

fn run_elections(data:&ElectionData, sampler:&SampleWithReplacement<usize>, num_elements_to_add:usize, num_runs:usize, rng: &mut impl Rng) -> Vec<DeltasInCandidateLists> {
    let mut res = vec![];
    let mut data = data.clone();
    let num_atl = data.atl.len();
    let mut undos : Vec<usize> = vec![];
    for _ in 0..num_runs {
        // get rid of prior manipulations
        for index in undos.drain(..) {
            if index<num_atl { data.atl[index].n-=1; } else { data.btl[index-num_atl].n-=1; }
        }
        // make new manipulation
        for _ in 0..num_elements_to_add {
            let index = sampler.get(rng);
            undos.push(index);
            if index<num_atl { data.atl[index].n+=1; } else { data.btl[index-num_atl].n+=1; }
        }
        let result = data.distribute_preferences::<FederalRulesUsed2013>().elected;
        let diff : DeltasInCandidateLists = DifferentCandidateLists{ list1: data.metadata.results.as_ref().unwrap().clone(), list2: result }.into();
        if WRITE_CHARACTER_PER_RUN {
            if diff.is_empty() { print!("."); } else { print!("*"); }
            std::io::stdout().flush().unwrap();
        }
        res.push(diff);
    }
    res
}

/// Get the sampler. The thing being sampled is an integer similar to RetroscopeVoteIndex.
fn build_sampler(data:&ElectionData) -> SampleWithReplacement<usize> {
    let mut res : SampleWithReplacement<usize> = Default::default();
    for atl_index in 0..data.atl.len() {
        res.add_multiple(atl_index,data.atl[atl_index].n);
    }
    let num_atl = data.atl.len();
    for btl_index in 0..data.btl.len() {
        res.add_multiple(btl_index+num_atl,data.btl[btl_index].n);
    }
    res
}