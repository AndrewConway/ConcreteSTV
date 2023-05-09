// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! This does some very early tests for NSW Legislative Council.


#[cfg(test)]
mod tests {
    use std::fs::File;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use nsw::nsw_random_rules::{NSWECRandomLC2015, NSWECRandomLC2019};
    use stv::parse_util::{FileFinder, RawDataSource};
    use nsw::parse_lc::{get_nsw_lc_data_loader_2015, get_nsw_lc_data_loader_2019, get_nsw_lc_data_loader_2023, NSWLCDataLoader, NSWLCDataSource};
    use stv::distribution_of_preferences_transcript::TranscriptWithMetadata;
    use stv::official_dop_transcript::{DifferenceBetweenOfficialDoPAndComputed, test_official_dop_without_actual_votes};
    use stv::preference_distribution::{distribute_preferences, PreferenceDistributionRules};
    use stv::random_util::Randomness;
    use stv::tie_resolution::{TieResolutionAtom, TieResolutionExplicitDecisionInCount};


    #[test]
    fn test_2015_data() {
        let finder = FileFinder::find_ec_data_repository();
        println!("Found files at {:?}",finder.path);
        let loader = get_nsw_lc_data_loader_2015(&finder).unwrap();
        println!("Made loader");
        let metadata = loader.read_raw_metadata("").unwrap();
        println!("{:?}",metadata);
        let data = loader.read_raw_data("").unwrap();
        data.print_summary();
        let official = loader.read_official_dop_transcript(&metadata).unwrap();
        assert!(official.quota.is_some());
        assert_eq!(391,official.counts.len());
        test::<NSWECRandomLC2015>("",&loader);
    }

    #[test]
    fn test_2019_data() {
        let finder = FileFinder::find_ec_data_repository();
        println!("Found files at {:?}",finder.path);
        let loader = get_nsw_lc_data_loader_2019(&finder).unwrap();
        println!("Made loader");
        let metadata = loader.read_raw_metadata("").unwrap();
        println!("{:?}",metadata);
        let data = loader.read_raw_data("").unwrap();
        data.print_summary();
        let official = loader.read_official_dop_transcript(&metadata).unwrap();
        assert!(official.quota.is_some());
        assert_eq!(343,official.counts.len());
        assert_eq!("Some(0.8688)",format!("{:?}",official.counts[1].transfer_value));
        test::<NSWECRandomLC2019>("",&loader);
    }

    #[test]
    fn test_2023_data() {
        let finder = FileFinder::find_ec_data_repository();
        println!("Found files at {:?}",finder.path);
        let loader = get_nsw_lc_data_loader_2023(&finder).unwrap();
        println!("Made loader");
        let metadata = loader.read_raw_metadata("").unwrap();
        println!("{:?}",metadata);
        let data = loader.read_raw_data("").unwrap();
        data.print_summary();
        let official = loader.read_official_dop_transcript(&metadata).unwrap();
        assert!(official.quota.is_some());
        assert_eq!(287,official.counts.len());
        assert_eq!("Some(0.8751)",format!("{:?}",official.counts[1].transfer_value));
        assert_eq!(data.num_votes(),official.counts[0].paper_total.as_ref().unwrap().sum());
        test::<NSWECRandomLC2019>("",&loader);
    }

    #[test]
    fn test_2015_internally_consistent() {
        assert_eq!(test_internally_consistent::<NSWECRandomLC2015>("2015").unwrap(),Ok(None));
    }

    #[test]
    fn test_2019_internally_consistent() {
        assert_eq!(test_internally_consistent::<NSWECRandomLC2019>("2019").unwrap(),Ok(None));
    }

    #[test]
    fn test_2023_internally_consistent() {
        assert_eq!(test_internally_consistent::<NSWECRandomLC2019>("2023").unwrap(),Ok(None));
    }


    /// Test a particular year & electorate against a particular set of rules.
    /// Outermost error is IO type errors.
    /// Innermost error is discrepancies with the official DoP.
    fn test_internally_consistent<Rules:PreferenceDistributionRules>(year:&str) -> anyhow::Result<Result<Option<TieResolutionExplicitDecisionInCount>, DifferenceBetweenOfficialDoPAndComputed<Rules::Tally>>> where <Rules as PreferenceDistributionRules>::Tally: Send+Sync+'static {
        test_official_dop_without_actual_votes::<Rules,_>(&NSWLCDataSource{},year,"",true)
    }

    fn test<Rules:PreferenceDistributionRules>(electorate:&str,loader:&NSWLCDataLoader) {
        let data = loader.read_raw_data(electorate).unwrap();
        data.print_summary();
        let mut tie_resolutions = data.metadata.tie_resolutions.clone();
        let official_transcript = loader.read_official_dop_transcript(&data.metadata).unwrap();
        let mut randomness = Randomness::PRNG(ChaCha20Rng::seed_from_u64(1));
        loop {
            let transcript = distribute_preferences::<Rules>(&data, loader.candidates_to_be_elected(electorate), &data.metadata.excluded.iter().cloned().collect(), &tie_resolutions,None,false,&mut randomness);
            let transcript = TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript };
            std::fs::create_dir_all("test_transcripts").unwrap();
            {
                let file = File::create(format!("test_transcripts/NSW LC{}.transcript",transcript.metadata.name.year)).unwrap();
                serde_json::to_writer_pretty(file,&transcript).unwrap();
            }
            match official_transcript.compare_with_transcript_checking_for_ec_decisions(&transcript.transcript,true) {
                Ok(None) => { return; }
                Ok(Some(decision)) => {
                    println!("Observed tie resolution {}", decision.decision);
                    tie_resolutions.tie_resolutions.push(TieResolutionAtom::ExplicitDecision(decision));
                }
                Err(DifferenceBetweenOfficialDoPAndComputed::DifferentNumbersOfCounts(official,our)) => {
                    println!("Official DoP had {} counts; ConcreteSTV had {}. Not surprising as the algorithm contains random elements.",official,our);
                    return;
                }
                Err(DifferenceBetweenOfficialDoPAndComputed::DifferentOnCount(count_index,_,diff)) => {
                    println!("Tie resolutions : {:?}",tie_resolutions);
                    println!("There was a difference between the official DoP and ConcreteSTV's on count {} : {}",1+count_index.0,diff);
                    if count_index.0<2 {
                        panic!("A count error on count {} is not explainable by the random part of the algorithm : {}",1+count_index.0,diff);
                    } else {
                        println!("This is probably due to the random elements of the algorithm.");
                        return;
                    }
                }
                Err(e) => {
                    println!("Tie resolutions : {:?}",tie_resolutions);
                    panic!("There was a difference between the official DoP and ConcreteSTV's : {}",e);
                }
            }
        }
    }


}

