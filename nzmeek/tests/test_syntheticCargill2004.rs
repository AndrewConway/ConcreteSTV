// Copyright 2026 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! A test of the NZ Meek data on some synthetic vote data.


#[cfg(test)]
mod tests {
    use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules};
    use std::collections::HashSet;
    use stv::tie_resolution::TieResolutionsMadeByEC;
    use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
    use std::fs::File;
    use nzmeek::NZMeek;
    use stv::compare_transcripts::{DeltasInCandidateLists, DifferentCandidateLists};
    use stv::election_data::ElectionData;
    use stv::fixed_precision_decimal::FixedPrecisionDecimal;
    use stv::official_dop_transcript::OfficialDistributionOfPreferencesTranscript;
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
        // test against transcript. This is not perfect as it doesn't check keep values nor is infinitely precise.
        let expected_transcript : TranscriptWithMetadata<<NZMeek as PreferenceDistributionRules>::Tally> = serde_json::from_str(include_str!("synthetic_Cargill2004_expected_transcript.json")).unwrap();
        let expected_transcript : OfficialDistributionOfPreferencesTranscript = expected_transcript.transcript.into();
        assert_eq!(Ok(None),expected_transcript.compare_with_transcript_checking_for_ec_decisions(&transcript.transcript,true));
        // test values not tested by official DoP code. This is Meek specific stuff and full precision stuff.
        assert_eq!(transcript.transcript.counts.len(),10);
        let last_count = &transcript.transcript.counts.last().unwrap().status;
        let scaled_expected_keep_values = vec![0,1000000000,1000000000,0,891075081,0,0,0,667755806,0]; // taken from https://www.prsa.org.au/2004-10-09_meek_stv_dunedin_cargill_ward.docx
        let expected_keep_values : Vec<String> = scaled_expected_keep_values.iter().copied().map(FixedPrecisionDecimal::<9>::from_scaled_value).map(|v|v.to_string()).collect();
        assert_eq!(last_count.keep_values,expected_keep_values);
        let scaled_expected_tallies : Vec<u64> = vec![0,1193722713168,1112880325624,0,1189526083194,0,0,0,1195068471724,0]; // taken from https://www.prsa.org.au/2004-10-09_meek_stv_dunedin_cargill_ward.docx
        let expected_tallies : Vec<FixedPrecisionDecimal<9>> = scaled_expected_tallies.iter().copied().map(FixedPrecisionDecimal::<9>::from_scaled_value).collect();
        assert_eq!(last_count.tallies.candidate,expected_tallies);
        assert_eq!(last_count.tallies.exhausted,FixedPrecisionDecimal::<9>::from_scaled_value(518802406290)); // taken from https://www.prsa.org.au/2004-10-09_meek_stv_dunedin_cargill_ward.docx
    }
    

}
