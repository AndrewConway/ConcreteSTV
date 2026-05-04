// Copyright 2021-2024 Andrew Conway, Vanessa Teague.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

/// Based on nsw2021_ivote_changes.rs, but with only the simple parts and with **beSTV not the NSW LGE rules**.
/// Edit YEAR to choose a different year of NSW local govt election data (currently we have 2012,
/// 2016, 2017, 2021, 2024) and find small changes that can lead to a different election outcome.
/// You can then use these outputs as upper bounds in https://github.com/michelleblom/pymarginstv.
use std::collections::HashSet;
use std::fs::{create_dir_all, File};
use std::io::Write;
use BESTV::beSTV;
use margin::choose_votes::ChooseVotesOptions;
use margin::find_outcome_changes::find_outcome_changes;
use margin::record_changes::ElectionChanges;
use nsw::parse_lge::{NSWLGEDataSource};
use stv::ballot_metadata::NumberOfCandidates;
use stv::datasource_description::ElectionDataSource;
use stv::parse_util::{FileFinder, RawDataSource};

const CHANGES_DIR: &str = "changes";
const STV_TALLY_DIR: &str = "tallies";
const BESTV: &str = "BESTV_NSW_LGE";
const YEAR: &str = "2024";

fn main() -> anyhow::Result <()> {

    let finder = FileFinder::find_ec_data_repository();
    println!("Found files at {:?}",finder.path);
    let datasource : NSWLGEDataSource = NSWLGEDataSource{};
    let loader = &datasource.get_loader_for_year(YEAR, &finder)?;
    println!("Made loader");
    let electorates = loader.all_electorates();

    let changes_path= format!("{CHANGES_DIR}/{BESTV}/{YEAR}");
    let tally_path= format!("{STV_TALLY_DIR}/{BESTV}/{YEAR}");
    create_dir_all(&changes_path)?;
    create_dir_all(&tally_path)?;
    let mut summary = File::create(format!("{changes_path}/summary.csv"))?;


    let ballot_types_considered_unverifiable = ["iVote"];
    let ballot_types_considered_unverifiable : HashSet<String> = ballot_types_considered_unverifiable.iter().map(|s|s.to_string()).collect();
    let options1 = ChooseVotesOptions{ allow_atl: true, allow_first_pref: true, allow_verifiable: true, ballot_types_considered_unverifiable:ballot_types_considered_unverifiable.clone(), allow_additions: false, allow_from: None, allow_to: None };
    writeln!(summary,"Electorate,Votes,Vacancies,Candidates,Min Manipulation")?;
    for electorate in &electorates {
        println!("Electorate: {}", electorate);
        let (data,old_min_add,old_min_manipulation,old_changes) = { // read in the published data, if available, to get the same order of votes, to make new results more directly comparable to old results.

            if let Ok(existing_parsed_file) = File::open(format!("published/{}.vchange", electorate)) {
                let old_changes : ElectionChanges<usize> = serde_json::from_reader(existing_parsed_file)?;
                let old_min_add = old_changes.changes.iter().filter( |vc | !vc.requires.changed_ballots).map(|vc| vc.ballots.n).min();
                let old_min_manipulation = old_changes.changes.iter().filter( |vc | vc.requires.changed_ballots).map(|vc| vc.ballots.n).min();
                (old_changes.original,old_min_add,old_min_manipulation,old_changes.changes)
            } else {
                let data = loader.read_raw_data_best_quality(electorate)?;
                (data,None,None,vec![])
            }

        };
        let data = loader.read_raw_data_best_quality(electorate)?;
        println!("Electorate: {}, vacancies: {:?}, candidates: {}", electorate, &data.metadata.vacancies, &data.metadata.candidates.len());
        data.print_summary();
        let out = File::create(format!("{tally_path}/{electorate}.stv"))?;
        serde_json::to_writer(out,&data)?;

        let mut results = find_outcome_changes::<beSTV>(&data,&options1,true,None);
        results.sort();

        let out = File::create(format!("{changes_path}/{electorate}.vchange"))?;
        serde_json::to_writer(out,&results)?;

        let min_manipulation = results.changes.iter().filter( |vc | vc.requires.changed_ballots).map(|vc| vc.ballots.n).min();
        writeln!(summary, "{},{},{},{},{}", electorate , data.num_votes(), data.metadata.vacancies.unwrap_or(NumberOfCandidates(0)),
                data.metadata.candidates.len(), min_manipulation.map(|vc| vc.to_string()).unwrap_or("".to_string()))?;

        summary.flush()?;
    }

    Ok(())
}

