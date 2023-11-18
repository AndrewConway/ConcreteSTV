// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This runs the federal elections and compares the results to the AEC provided transcripts.


#[cfg(test)]
mod tests {
    use std::cell::OnceCell;
    use act::parse::{get_act_data_loader_2020};
    use stv::preference_distribution::{distribute_preferences_with_extractors, PreferenceDistributionRules};
    use std::collections::HashSet;
    use std::fs::File;
    use stv::tie_resolution::TieResolutionsMadeByEC;
    use std::sync::{Arc, Mutex};
    use stv::parse_util::{RawDataSource, FileFinder};
    use act::{ACT2021};
    use stv::ballot_metadata::CandidateIndex;
    use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
    use stv::election_data::ElectionData;
    use stv::extract_votes_in_pile::{ExtractionRequest, WhatToDoWithExtractedVotes, WhatToExtract};
    use stv::random_util::Randomness;

    
    fn test_extract_votes2020<Rules:PreferenceDistributionRules>(electorate:&str, ex_mla:&str,excluded_names_in_recount:&[&str]) -> anyhow::Result<()> {
        let loader = get_act_data_loader_2020(&FileFinder::find_ec_data_repository())?;
        let data = loader.read_raw_data(electorate)?;
        println!("{:?}",data.metadata.candidates);
        let candidate_name_lookup = data.metadata.get_candidate_name_lookup_multiple_ways();
        let who = *candidate_name_lookup.get(ex_mla).unwrap();
        let extracted_data : Arc<OnceCell<ElectionData>> = Arc::new(OnceCell::new());
        let what_to_extract = WhatToExtract::ACTVotesUsedToElectCandidate(who);
        let cloned_extracted_data = extracted_data.clone();
        let what_to_do_with_it = WhatToDoWithExtractedVotes::CallFunction(Box::new(Mutex::new(move |e:ElectionData|{cloned_extracted_data.set(e).unwrap();})));
        let extractors = vec![ExtractionRequest{ what_to_extract, what_to_do_with_it  }];
        let transcript = distribute_preferences_with_extractors::<Rules>(&data, loader.candidates_to_be_elected(electorate), &HashSet::default(), &TieResolutionsMadeByEC::default(),None,true,&mut Randomness::ReverseDonkeyVote,&extractors);
        let mut excluded_in_recount: HashSet<CandidateIndex> = HashSet::default();
        for &c in &transcript.elected {
            excluded_in_recount.insert(c);
        }
        for &c in excluded_names_in_recount {
            excluded_in_recount.insert(*candidate_name_lookup.get(c).unwrap());
        }
        let transcript = TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript };
        std::fs::create_dir_all("test_transcripts/extract")?;
        let file = File::create(format!("test_transcripts/extract/transcript{}{}.json",electorate,transcript.metadata.name.year))?;
        serde_json::to_writer_pretty(file,&transcript)?;
        let extracted_data = extracted_data.get().unwrap();
        let file = File::create(format!("test_transcripts/extract/CasualVacancy{}{}-{}.stv",electorate,transcript.metadata.name.year,ex_mla))?;
        serde_json::to_writer_pretty(file,&extracted_data)?;
        extracted_data.print_summary();
        // TODO make correct rules that handle quota correctly - recompute at each round.
        let transcript = distribute_preferences_with_extractors::<Rules>(&extracted_data, extracted_data.metadata.vacancies.unwrap(), &excluded_in_recount, &TieResolutionsMadeByEC::default(),None,true,&mut Randomness::ReverseDonkeyVote,&[]);
        let transcript = TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript };
        let file = File::create(format!("test_transcripts/extract/Casual Vacancy {} Transcript {} {}.json",ex_mla,electorate,transcript.metadata.name.year))?;
        serde_json::to_writer_pretty(file,&transcript)?;
        Ok(())
    }
    #[test]
    #[allow(non_snake_case)]
    fn test_JohnathonDavis() { test_extract_votes2020::<ACT2021>("Brindabella", "DAVIS, Johnathan",&[]).unwrap() }

    #[test]
    #[allow(non_snake_case)]
    fn test_AllistairCoe() { test_extract_votes2020::<ACT2021>("Yerrabi", "COE, Alistair",&["STRANG, Bernie","KEARSLEY, John","YOUNG, Scott","WILLIAMS, Bethany","BRENNAN, Bernie","HORNE, Francine","FISCHER, Tom","ORR, Suzanne","PHILLIPS, Georgia","CROSS, Helen"]).unwrap() }


}