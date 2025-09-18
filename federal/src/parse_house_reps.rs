// Copyright 2025 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Parse the House of Representatives data.
//! Note that the AEC does not publish individual votes, just the Distribution of Preferences.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;
use anyhow::bail;
use stv::ballot_metadata::{Candidate, CandidateIndex, DataSource, ElectionMetadata, ElectionName, NumberOfCandidates, Party, PartyIndex};
use stv::ballot_pile::{BallotPaperCount, DoNotSplitByCountNumber, HowSplitByCountNumber};
use stv::distribution_of_preferences_transcript::{PerCandidate, Transcript};
use stv::official_dop_transcript::{OfficialDOPForOneCount, OfficialDistributionOfPreferencesTranscript};
use stv::parse_util::{skip_first_line_of_file};
use stv::preference_distribution::{BigRational, CountNamingMethod, LastParcelUse, PreferenceDistributionRules, SurplusTransferMethod, TransferValueMethod, WhenToDoElectCandidateClauseChecking};
use stv::signed_version::SignedVersion;
use stv::tie_resolution::MethodOfTieResolution;
use stv::transfer_value::{convert_usize_to_rational, round_rational_down_to_usize, TransferValue};

pub struct DivisionInfo {
    pub metadata: ElectionMetadata,
    pub dop : OfficialDistributionOfPreferencesTranscript,
}

/// Parse the HouseDopByDivisionDownloadNNNNN.csv file to get all the distribution of preferences for a year.
#[allow(non_snake_case)]
pub fn parse_HouseDopByDivisionDownload(path:&Path,year:&str,url:&str) -> anyhow::Result<Vec<DivisionInfo>> {
    let mut divisions : HashMap<u64,DivisionInfo> = HashMap::new();
    let mut rdr = csv::Reader::from_reader(skip_first_line_of_file(path)?);
    for result in rdr.records() {
        let record = result?;
        let division_id : u64 = record[1].parse()?;
        let division = divisions.entry(division_id).or_insert_with(||DivisionInfo{
            metadata: ElectionMetadata {
                name: ElectionName {
                    year: year.to_string(),
                    authority: "Australian Electoral Commission".to_string(),
                    name: "Federal House of Representatives".to_string(),
                    electorate: format!("{} ({})", &record[2], &record[0]),
                    modifications: vec![],
                    comment: None,
                },
                candidates: vec![],
                parties: vec![],
                source: vec![DataSource{
                    url: url.to_string(),
                    files: vec![path.file_name().unwrap().to_str().unwrap().to_string()],
                    comments: None,
                }],
                results: None,
                vacancies: Some(NumberOfCandidates(1)),
                enrolment: None,
                secondary_vacancies: None,
                excluded: vec![],
                tie_resolutions: Default::default(),
            },
            dop: Default::default(),
        });
        let count_number : usize = record[3].parse()?;
        let ballot_position : usize = record[4].parse()?;
        let candidate_index = CandidateIndex(ballot_position-1);
        if ballot_position==division.metadata.candidates.len()+1 { // need to add a new candidate
            let party_name = record[9].trim();
            let party_abr = record[8].trim();
            if !party_name.is_empty() {
                division.metadata.parties.push(Party{column_id:"".to_string(),name:party_name.to_string(),abbreviation:if party_abr.is_empty() {None} else {Some(party_abr.to_string())},candidates:vec![candidate_index],how_to_vote_atl:vec![],atl_allowed:false,how_to_vote_btl:vec![],tickets:vec![]})
            }
            division.metadata.candidates.push(Candidate{
                name: format!("{} {}",&record[7],&record[6]),
                party: if party_name.is_empty() {None} else {Some(PartyIndex(division.metadata.parties.len()-1))},
                position: if party_name.is_empty() {None} else {Some(1)},
                ec_id: Some(record[5].to_string()),
            });
            if record[10].trim()=="Y" { // candidate elected
                division.metadata.results=Some(vec![candidate_index]);
            }
        }
        assert!(ballot_position<=division.metadata.candidates.len());
        if count_number==division.dop.counts.len() { // starting a new count
            assert_eq!(ballot_position,1);
            division.dop.counts.push(OfficialDOPForOneCount{
                transfer_value: None,
                elected: vec![],
                excluded: vec![],
                vote_total: Some(PerCandidate{candidate:vec![],exhausted:0.0,rounding:SignedVersion::from(0.0),set_aside:None}),
                paper_total: Some(PerCandidate{candidate:vec![],exhausted:0,rounding:SignedVersion::from(0),set_aside:None}),
                vote_delta: Some(PerCandidate{candidate:vec![],exhausted:0.0,rounding:SignedVersion::from(0.0),set_aside:None}),
                paper_delta: Some(PerCandidate{candidate:vec![],exhausted:0,rounding:SignedVersion::from(0),set_aside:None}),
                paper_set_aside_for_quota: None,
                count_name: None,
                papers_came_from_counts: None,
            });
        }
        assert_eq!(count_number + 1, division.dop.counts.len());
        let value = &record[13];
        match &record[12] {
            "Preference Count" => {
                let candidate = &mut division.dop.counts.last_mut().unwrap().paper_total.as_mut().unwrap().candidate;
                let value : usize = value.parse()?;
                assert_eq!(candidate_index.0,candidate.len());
                candidate.push(value);
                division.dop.counts.last_mut().unwrap().vote_total.as_mut().unwrap().candidate.push(value as f64);
            }
            "Preference Percent" => {}
            "Transfer Count" => {
                let value : isize = if count_number==0 {*division.dop.counts.last_mut().unwrap().paper_total.as_mut().unwrap().candidate.last().unwrap() as isize} else {value.parse()?};
                if value<0 { // this candidate is being excluded
                    division.dop.counts.last_mut().unwrap().excluded.push(candidate_index);
                }
                let candidate = &mut division.dop.counts.last_mut().unwrap().paper_delta.as_mut().unwrap().candidate;
                assert_eq!(candidate_index.0,candidate.len());
                candidate.push(value);
                division.dop.counts.last_mut().unwrap().vote_delta.as_mut().unwrap().candidate.push(value as f64);
            }
            "Transfer Percent" => {}
            _ => bail!("Unknown CalculationType {}", &record[12])
        }
    }
    let mut divisions : Vec<DivisionInfo> = divisions.into_iter().map(|(_,v)|v).collect();
    for division in &mut divisions {
        division.dop.counts.last_mut().unwrap().elected = division.metadata.results.as_ref().unwrap().clone();
    }
    Ok(divisions)
}



/// The Federal IRV rules, my interpretation of the Commonwealth Electoral Act 1918,
/// Section 274, most importantly subsections (7) to (9)
pub struct FederalHouseRepresentativesIRV {
}

impl PreferenceDistributionRules for FederalHouseRepresentativesIRV { 
    type Tally = usize;
    type SplitByNumber = DoNotSplitByCountNumber;

    /// MAKE IT IRV!
    fn has_quota() -> bool { false }
    // a bunch of not applicable functions.
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::No }
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverContinuingBallots }
    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { convert_usize_to_rational(tally)  }
    fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { round_rational_down_to_usize(rational)  }
    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue { // NA
        TransferValue::from_surplus(surplus,ballots)
    }
    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> usize {
        transfer_value.mul_rounding_down(ballots)
    }
    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::ScaleTransferValues }
    fn sort_exclusions_by_transfer_value() -> bool { false }

    /// Section (9), 
    /// ```text
    /// (in the case of ties for exclusion)
    /// the candidate to be excluded is the candidate with less votes than
    /// any of the other lowest ranking candidates at the last count at
    /// which one of those candidates had less votes than any of the others
    /// ```
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalUniqueLowest }
    /// Section (9C)
    /// ```text
    /// If, after the fresh scrutinies referred to in subsection (9A), 2 or
    /// more candidates have an equal number of votes, the Divisional
    /// Returning Officer shall give to the Electoral Commissioner written
    /// notice that the election cannot be decided.
    /// ```
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::None }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::None } // NA
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::None } // NA

    // more not applicable functions.
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { false }
    fn check_elected_if_in_middle_of_exclusion() -> bool { false }
    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }

    // termination condition in case of tie for top.
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfExclusionNotOngoing }
    // normal termination condition
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterFirstPreferencesAndWhenOnly2CandidatesAreContinuing} 

    /// The Commonwealth Electoral Act 1918, Section 274, subsection (7AA)(b)(i) rule should be used. 
    /// ```text
    /// if the total number of first preference votes for all the
    /// candidates, other than the first and second ranked
    /// candidates, is less than the number of first preference
    /// votes for the second ranked candidate—exclude all the
    /// candidates other than the first and second ranked
    /// candidates
    /// ```
    fn should_eliminate_all_but_top_two_candidates_if_all_remaining_add_to_fewer_on_count_2() -> bool { true } 

    /// The Commonwealth Electoral Act 1918, Section 274, subsection (9C)
    /// ```text
    /// if , after the fresh scrutinies referred to in subsection (9A), 2 or
    /// more candidates have an equal number of votes, the Divisional
    /// Returning Officer shall give to the Electoral Commissioner written
    /// notice that the election cannot be decided.
    /// ```
    fn declare_election_over_if_2_tied_candidates_remain() -> bool { true }
    fn name() -> String { "FederalIRV".to_string() }
    fn how_to_name_counts() -> CountNamingMethod { CountNamingMethod::SimpleNumber }

    fn sort_subcounts_by_count() -> Option<Box<dyn FnMut(&Transcript<Self::Tally>,<<Self as PreferenceDistributionRules>::SplitByNumber as HowSplitByCountNumber>::KeyToDivide,<<Self as PreferenceDistributionRules>::SplitByNumber as HowSplitByCountNumber>::KeyToDivide) -> Ordering>> {
        None
    }
    fn should_exhausted_votes_count_for_quota_computation() -> bool { false }
}


/// My interpretation of the IRV rules used by the AEC to generate the distribution of
/// preferences in the AEC provided file `HouseDopByDivisionDownloadNNNNN.csv`
///
/// This is significantly different from the legislation,
/// Commonwealth Electoral Act 1918,
/// Section 274, most importantly subsections (7) to (9).
/// In particular, the legislation has two main differences
/// * It checks after counting first preferences to see if someone has a majority, and declares them elected then.
/// * It checks after counting first preferences to see if it can be simplified by multiple exclusion into a 2 part race in subsection (7AA)(b)(i).
/// 
/// However, neither of these can change the outcome of the election, just make the count shorter, and using them would
/// provide less information. So it seems to me to be reasonable for the AEC to provide the more detailed data.
///
pub struct FederalHouseRepresentativesIRVAlwaysSimpleIRVToTwoCandidates {
}

impl PreferenceDistributionRules for FederalHouseRepresentativesIRVAlwaysSimpleIRVToTwoCandidates {
    type Tally = usize;
    type SplitByNumber = DoNotSplitByCountNumber;

    /// MAKE IT IRV!
    fn has_quota() -> bool { false }
    // a bunch of not applicable functions.
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::No }
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverContinuingBallots }
    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { convert_usize_to_rational(tally)  }
    fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { round_rational_down_to_usize(rational)  }
    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue { // NA
        TransferValue::from_surplus(surplus,ballots)
    }
    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> usize {
        transfer_value.mul_rounding_down(ballots)
    }
    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::ScaleTransferValues }
    fn sort_exclusions_by_transfer_value() -> bool { false }

    /// Section (9), 
    /// ```text
    /// (in the case of ties for exclusion)
    /// the candidate to be excluded is the candidate with less votes than
    /// any of the other lowest ranking candidates at the last count at
    /// which one of those candidates had less votes than any of the others
    /// ```
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalUniqueLowest }
    /// Section (9C)
    /// ```text
    /// If, after the fresh scrutinies referred to in subsection (9A), 2 or
    /// more candidates have an equal number of votes, the Divisional
    /// Returning Officer shall give to the Electoral Commissioner written
    /// notice that the election cannot be decided.
    /// ```
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::None }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::None } // NA
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::None } // NA

    // more not applicable functions.
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { false }
    fn check_elected_if_in_middle_of_exclusion() -> bool { false }
    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }

    // termination condition in case of tie for top.
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfExclusionNotOngoing }
    // normal termination condition
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::WhenOnly2CandidatesAreContinuing} // CHANGE 1 - ignore legislation

    /// The Commonwealth Electoral Act 1918, Section 274, subsection (7AA)(b)(i) rule should be used. 
    /// ```text
    /// if the total number of first preference votes for all the
    /// candidates, other than the first and second ranked
    /// candidates, is less than the number of first preference
    /// votes for the second ranked candidate—exclude all the
    /// candidates other than the first and second ranked
    /// candidates
    /// ```
    fn should_eliminate_all_but_top_two_candidates_if_all_remaining_add_to_fewer_on_count_2() -> bool { false } // CHANGE 2 - ignore legislation.

    /// The Commonwealth Electoral Act 1918, Section 274, subsection (9C)
    /// ```text
    /// if , after the fresh scrutinies referred to in subsection (9A), 2 or
    /// more candidates have an equal number of votes, the Divisional
    /// Returning Officer shall give to the Electoral Commissioner written
    /// notice that the election cannot be decided.
    /// ```
    fn declare_election_over_if_2_tied_candidates_remain() -> bool { true }
    fn name() -> String { "AEC_IRV".to_string() }
    fn how_to_name_counts() -> CountNamingMethod { CountNamingMethod::SimpleNumber }

    fn sort_subcounts_by_count() -> Option<Box<dyn FnMut(&Transcript<Self::Tally>,<<Self as PreferenceDistributionRules>::SplitByNumber as HowSplitByCountNumber>::KeyToDivide,<<Self as PreferenceDistributionRules>::SplitByNumber as HowSplitByCountNumber>::KeyToDivide) -> Ordering>> {
        None
    }
    fn should_exhausted_votes_count_for_quota_computation() -> bool { false }
}