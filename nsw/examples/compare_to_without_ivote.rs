// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use nsw::NSWECLocalGov2021;
use nsw::parse_lge::get_nsw_lge_data_loader_2021;
use stv::compare_transcripts::{DeltasInCandidateLists, DifferentCandidateLists};
use stv::parse_util::{FileFinder, RawDataSource};

/// Compare the "official" website results vs. the results of counting vs the results if iVote votes are not included.
fn main() -> anyhow::Result<()> {
    let loader = get_nsw_lge_data_loader_2021(&FileFinder::find_ec_data_repository())?;
    let mut num_electorate = 0;
    let mut num_different_official = 0;
    let mut num_different_ivotes = 0;
    for electorate in loader.all_electorates() {
        num_electorate+=1;
        let data_with_ivotes = loader.read_raw_data_possibly_rejecting_some_types(&electorate,None)?;
        let data_without_ivotes = loader.read_raw_data_possibly_rejecting_some_types(&electorate,Some(vec!["iVote".to_string()].into_iter().collect()))?;
        let total_num_votes = data_with_ivotes.num_votes();
        let total_num_ivotes = total_num_votes-data_without_ivotes.num_votes();
        let turnout = if let Some(enrolment) = data_with_ivotes.metadata.enrolment { format!(" enrolment {} informal {} turnout {:.1}%",enrolment.0,data_with_ivotes.informal,100.0*(total_num_votes+data_with_ivotes.informal) as f64/enrolment.0 as f64) } else { "".to_string() };
        println!("Electorate {} {} formal votes including {} formal iVotes ({:.1}%){}",&electorate,total_num_votes,total_num_ivotes,100.0*total_num_ivotes as f64/total_num_votes as f64,turnout);
        let transcript_with_ivotes = data_with_ivotes.distribute_preferences::<NSWECLocalGov2021>();
        let transcript_without_ivotes = data_without_ivotes.distribute_preferences::<NSWECLocalGov2021>();
        let compare_official : DeltasInCandidateLists = DifferentCandidateLists{ list1: data_with_ivotes.metadata.results.as_ref().unwrap().clone(), list2: transcript_with_ivotes.elected.clone() }.into();
        if !compare_official.is_empty() {
            println!("  Different to official results for {} : {}",&electorate,compare_official.pretty_print(&data_with_ivotes.metadata));
            num_different_official+=1;
        }
        let compare_without_ivote  : DeltasInCandidateLists = DifferentCandidateLists{ list1: transcript_with_ivotes.elected.clone(), list2: transcript_without_ivotes.elected.clone() }.into();
        if !compare_without_ivote.is_empty() {
            println!("  Different when excluding iVotes for {} : {}",&electorate,compare_without_ivote.pretty_print(&data_with_ivotes.metadata));
            num_different_ivotes+=1;
        }
        // let difference = stv::compare_transcripts::compare_transcripts(&transcript_with_ivotes,&transcript_without_ivotes);
    }
    println!("Of {} electorates, {} differ from official results and {} differ when iVote votes are removed",num_electorate,num_different_official,num_different_ivotes);
    Ok(())
}