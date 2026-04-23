// Copyright 2026 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! A test of the NZ Meek data on some synthetic vote data.


#[cfg(test)]
mod tests {
    use stv::preference_distribution::distribute_preferences;
    use std::collections::HashSet;
    use stv::tie_resolution::TieResolutionsMadeByEC;
    use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
    use std::fs::File;
    use nzmeek::NZMeek;
    use stv::compare_transcripts::{DeltasInCandidateLists, DifferentCandidateLists};
    use stv::election_data::ElectionData;
    use stv::random_util::Randomness;

    fn load_synthetic() -> ElectionData {
        let json = include_str!("synthetic_Cargill2004.stv");
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn test_synthetic() {
        let data = load_synthetic();
        data.print_summary();
        let transcript = distribute_preferences::<NZMeek>(&data, data.metadata.vacancies.unwrap(), &HashSet::default(), &TieResolutionsMadeByEC::default(),None,true,&mut Randomness::ReverseDonkeyVote);
        let transcript = TranscriptWithMetadata{ metadata: data.metadata, transcript };
        std::fs::create_dir_all("test_transcripts").unwrap();
        let file = File::create("test_transcripts/synthetic_Cargill2004.json").unwrap();
        serde_json::to_writer_pretty(file,&transcript).unwrap();
        if let Some(official_results) = &transcript.metadata.results && *official_results!=transcript.transcript.elected {
            let lists : DeltasInCandidateLists = DifferentCandidateLists{list1:official_results.clone(),list2:transcript.transcript.elected.clone() }.into();
            println!("Official elected differs from computed : {}",lists.pretty_print(&transcript.metadata));
        }
        // TODO actually test values.
    }
    

}
