// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Rules for NSW counting.
//! The NSW state house is very probabilistic, and is not currently handled here as the output format is not entirely appropriate for a very probabilistic algorithm.
//! The local council elections use totally different legislation with its own set of problems - it is wildly ambiguous.
//! # Legislation
#![doc = include_str!("../NSWLocalCouncilLegislation2021.md")]
//! # My thoughts.
#![doc = include_str!("../NSWLocalCouncilLegislation2021Commentary.md")]

pub mod parse_lge;

use std::cmp::Ordering;
use stv::preference_distribution::{BigRational, CountNamingMethod, PreferenceDistributionRules, SurplusTransferMethod, TransferValueMethod, WhenToDoElectCandidateClauseChecking};
use stv::ballot_pile::{BallotPaperCount, DoNotSplitByCountNumber, FullySplitByCountNumber, HowSplitByCountNumber};
use stv::distribution_of_preferences_transcript::{CountIndex, Transcript};
use stv::transfer_value::{convert_usize_to_rational, round_rational_down_to_usize, TransferValue};
use stv::tie_resolution::MethodOfTieResolution;

/// My guess at what the legislation means. See my comments below
/// for reasons behind things. I am not claiming these are right;
/// just that they generally seem no more wrong than any other interpretation.
#[doc = include_str!("../NSWLocalCouncilLegislation2021.md")]
pub struct NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation {
}

impl PreferenceDistributionRules for NSWLocalCouncilLegislation2021MyGuessAtHighlyAmbiguousLegislation {
    type Tally = usize;
    type SplitByNumber = FullySplitByCountNumber;

    fn use_last_parcel_for_surplus_distribution() -> bool { false }
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverContinuingBallots }

    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { convert_usize_to_rational(tally)  }
    fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { round_rational_down_to_usize(rational)  }

    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue { // NA
        TransferValue::from_surplus(surplus,ballots)
    }

    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> usize {
        transfer_value.mul_rounding_down(ballots)
    }

    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::MergeSameTransferValuesAndScale }
    fn sort_exclusions_by_transfer_value() -> bool { false }

    /// In general, Highly ambiguous, although OK for 2 person case.
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    /// same as [resolve_ties_elected_one_of_last_two]
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    /// same as [resolve_ties_elected_one_of_last_two]
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }

    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }

    /// see discussion
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { false }
    /// see discussion
    fn check_elected_if_in_middle_of_exclusion() -> bool { true }

    /// 9(7) says
    /// ```text
    /// (7)  This clause is subject to clause 11 of this Schedule, and if at any time there is one remaining vacancy which can be filled under that clause, no further exclusion under this clause can be made.
    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    /// 7(5) and 6(2) says
    /// ```text
    /// (5)  However, this clause is subject to clause 11 of this Schedule, and if at any time there is one remaining vacancy which can be filled under that clause, no further transfer under this clause can be made.
    /// ```
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }


    /// See discussion
    /// ```text
    /// 11 (4)  When only one vacancy remains unfilled, and there are only 2 continuing candidates, and those 2 candidates each have the same number of votes, and no surplus votes remain capable of transfer, one candidate is excluded in accordance with clause 9(5) and (6) of this Schedule and the other is elected.
    /// ```
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }

    /// See discussion.
    /// ```text
    /// 11 (1)  When the number of continuing candidates is reduced to the number of vacancies remaining unfilled the continuing candidates are elected, even if they have not reached the quota.
    /// ```
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfExclusionNotOngoing }

    /// See discussion.
    /// ```text
    /// 11 (2)  When only one vacancy remains unfilled and the votes of one continuing candidate exceed the total of all the votes of the other continuing candidates, together with any surplus not transferred, that candidate is elected.
    /// 11 (3)  When more than one vacancy remains unfilled and the votes of the candidate who (if all the vacancies were filled by the successive election of the continuing candidates with the largest number of votes) would be the last to be elected exceed the total of any surplus not transferred plus the votes of all the continuing candidates with fewer votes than that candidate, that candidate and all the other continuing candidates who do not have fewer votes than that candidate are elected.
    /// ```
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfExclusionNotOngoing}
    /// The legislation may be bad, but at least it doesn't have this buggy mess!
    fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { false }

    fn name() -> String { "NSWLocalGov2021".to_string() }
    fn how_to_name_counts() -> CountNamingMethod { CountNamingMethod::BasedOnSourceName }
}


/// My guess at what the algorithm used by NSWEC in the 2021 Local Government Elections.
/// The legislation is ambiguous so this is inspired by the legislation and looking at the detailed distribution of preferences transcript.
pub struct NSWECLocalGov2021 {
}

impl PreferenceDistributionRules for NSWECLocalGov2021 {
    type Tally = usize;
    type SplitByNumber = FullySplitByCountNumber;

    fn use_last_parcel_for_surplus_distribution() -> bool { false }
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverContinuingBallots }

    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { convert_usize_to_rational(tally)  }
    fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { round_rational_down_to_usize(rational)  }

    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue { // NA
        TransferValue::from_surplus(surplus,ballots)
    }

    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> usize {
        transfer_value.mul_rounding_down(ballots)
    }

    /// NSWEC split surplus distribution into separate transfers for each source transfer in. E.g. City of Albury, count 47
    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::ScaleTransferValues }
    fn sort_exclusions_by_transfer_value() -> bool { false }

    /// Evidence that only when an action is finished is relevant : City of Albury, count 39,
    /// TIERNAN Jodie was eliminated. At the end of count 38, TIERNAN Jodie was tied on 94 with DOCKSEY Graham.
    /// A subset of votes are shown here.
    /// ```text
    /// Count     TIERNAN Jodie   DOCKSEY Graham
    /// 37             88             92
    /// ...
    /// 38.27.23.1     94             93
    /// ...
    /// 38             94             94
    /// ```
    /// The NSWEC decided to exclude TIERNAN Jodie, presumably because of count 37, ignoring the subcounts of 38 such as 38.27.23.1.
    ///
    /// In City of Campbelltown, at the end of counts 29.xx, there is a 3 way tie for 25. Going backwards
    /// ```text
    /// count    BA    HP   IC
    /// 22       23    18   21
    /// 24       23    25   25
    /// 29       25    25   25
    /// ```text
    /// The NSWEC excluded BA at count 24, which means there was not a requirement for all to be different.
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinished }
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinished }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinished }
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinished }

    /// NSWEC didn't check in the middle of surplus. E.g. City of Albury, count 47
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { false }
    /// NSWEC didn't check in the middle of exclusion. E.g. City of Albury, count 46
    fn check_elected_if_in_middle_of_exclusion() -> bool { false }


    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfExclusionNotOngoing }
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfExclusionNotOngoing}
    fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { false }

    fn name() -> String { "NSWECLocalGov2021".to_string() }
    fn how_to_name_counts() -> CountNamingMethod { CountNamingMethod::BasedOnSourceName }

    /// There is some bizarre ordering thing in the City of Albury. Count 42.41.20.5.1 is listed after counts 42.41.20.14.1 and 42.41.20.14.5.1.
    /// count 42 is the exclusion of COHN Amanda
    /// count 41 is the exclusion of ISAACS Kofi
    /// count 20 is the exclusion of MONTE Susie
    /// count 14 is the exclusion of PATTINSON Jill
    /// count 5 is the distribution of EDWARDS Ashley's preferences.
    /// count 1 is first preferences.
    /// in count 41, 41.20.5.1 is listed before 41.20.14.1.
    /// similarly count 46.40.30.23.4.1 is listed before 46.40.30.4.1. I believe this is because the counts are sorted numerically on the first 3 numbers, but lexicographically on the fourth number.
    /// This is almost certainly a bug in the NSWEC code. I think it will not affect who is elected,
    /// as the order of subcounts doesn't matter in their interpretation of the legislation. Nevertheless,
    /// the legislation is specific in 9(2)(b) about the order it should be done.
    /// A very similar thing happens for surplus distributions - compare 47.45.40.4.1 and 47.45.40.37.6.2.1
    fn sort_subcounts_by_count() -> Option<Box<dyn FnMut(&Transcript<Self::Tally>,<<Self as PreferenceDistributionRules>::SplitByNumber as HowSplitByCountNumber>::KeyToDivide,<<Self as PreferenceDistributionRules>::SplitByNumber as HowSplitByCountNumber>::KeyToDivide) -> Ordering>> {
        Some(Box::new(Self::sort_counts_by_name_as_dotted_number_sequences_numerically_first_3_fields_lexicographically_afterwards))
    }

    /// In Benalla B Ward, CADWALLADER Sharon was deemed ineligible. There were two formal
    /// votes that contained only a 1 for her. The question is whether these should be included
    /// in the computation of quota. The NSWEC records 9087 "Formal BPs" in the section of the
    /// detailed dopfulldetails.xlsx adjacent to the Vacancies and Quota numbers in a way that implies
    /// that this is the number used in the quota computation. 9087 includes the two votes exhausted on
    /// round 1 as they could not go to an eligible candidate. However this is not conclusive as the 2 votes
    /// make no difference after rounding.
    ///
    /// More illuminating is the City of Broken Hill. At the start of the `dopfulldetails.xls` spreadsheet are
    /// the following table
    /// ```text
    /// City of Broken Hill
    /// Formal BPs  10,395
    /// Vacancies   9
    /// Quota       1,039
    /// ```
    /// Now it is clear that these numbers are not consistent with being the numbers used in the formula to
    /// compute the quota. - round_down((10395)/(9+1))+1 = 1040, not 1039. The 10395 is indeed the number
    /// of formal BPs; however the total number of valid first preference votes is 6 fewer, as KENNEDY Tom
    /// is ruled ineligible. This accounts for the lower quota. This is a perfectly reasonable thing to do
    /// (and what ConcreteSTV has done for every other jurisdiction so far implemented);
    /// it is at the most a misleading collection of figures; the "Formal BPs"
    /// number (which is not used in the counting legislation) would be better off to be
    /// replaced by a "First Preferences" 10389 row (which is used in the dop counting legislation
    /// but is not listed in the `dopfulldetails.xls` file.
    fn should_exhausted_votes_count_for_quota_computation() -> bool { false }
}

// helper functions to reproduce idiosyncratic NSWEC 2021 ordering.
impl NSWECLocalGov2021 {
    /// Compare two strings for ordering. The two strings should be in the form of integers separated by `.` characters.
    /// For the first 3-n_already_done, compare integers numerically. Afterwards sort lexicographically.
    /// # Examples
    /// ```
    /// use std::cmp::Ordering;
    /// use nsw::NSWECLocalGov2021;
    /// assert_eq!(Ordering::Equal,NSWECLocalGov2021::sort_names_as_dotted_number_sequences_numerically_first_3_fields_lexicographically_afterwards("3.5.6.7","3.5.6.7",0));
    /// assert_eq!(Ordering::Less,NSWECLocalGov2021::sort_names_as_dotted_number_sequences_numerically_first_3_fields_lexicographically_afterwards("3.5.6.7","3.5.26.7",0));
    /// assert_eq!(Ordering::Greater,NSWECLocalGov2021::sort_names_as_dotted_number_sequences_numerically_first_3_fields_lexicographically_afterwards("3.5.6.7","3.5.6.27",0));
    /// ```
    pub fn sort_names_as_dotted_number_sequences_numerically_first_3_fields_lexicographically_afterwards(name1:&str,name2:&str,n_already_done:usize) -> Ordering {
        if name1==name2 { Ordering::Equal }
        else if name1.is_empty() { Ordering::Less } // should never happen
        else if name2.is_empty() { Ordering::Greater } // should never happen
        else {
            let (name1_prefix,name1_suffix) = name1.split_once('.').unwrap_or((name1,""));
            let (name2_prefix,name2_suffix) = name2.split_once('.').unwrap_or((name2,""));
            if name1_prefix==name2_prefix { Self::sort_names_as_dotted_number_sequences_numerically_first_3_fields_lexicographically_afterwards(name1_suffix,name2_suffix,n_already_done+1)}
            else if n_already_done<3 {
                name1_prefix.parse::<usize>().unwrap().cmp(&name2_prefix.parse::<usize>().unwrap())
            } else {
                name1_prefix.cmp(name2_prefix)
            }
        }
    }

    fn sort_counts_by_name_as_dotted_number_sequences_numerically_first_3_fields_lexicographically_afterwards(transcript_so_far : &Transcript<usize>,count1:CountIndex,count2:CountIndex) -> Ordering {
        let name1 = transcript_so_far.count(count1).count_name.as_ref().unwrap();
        let name2 = transcript_so_far.count(count2).count_name.as_ref().unwrap();
        Self::sort_names_as_dotted_number_sequences_numerically_first_3_fields_lexicographically_afterwards(name1,name2,1) // use 1 already done as the current count will be prefixed to this.
    }
}


/// A simple IRV computation.
pub struct SimpleIRVAnyDifferenceBreaksTies {
}

impl PreferenceDistributionRules for SimpleIRVAnyDifferenceBreaksTies {
    type Tally = usize;
    type SplitByNumber = DoNotSplitByCountNumber;

    /// MAKE IT IRV!
    fn has_quota() -> bool { false }
    // a bunch of not applicable functions.
    fn use_last_parcel_for_surplus_distribution() -> bool { false }
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

    /// Legislation is ambiguous - use method there was evidence EC used for similar legislation for STV..
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinished }
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinished }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinished }
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinished }

    // more not applicable functions.
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { false }
    fn check_elected_if_in_middle_of_exclusion() -> bool { false }
    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }

    // termination condition in case of tie for top.
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfExclusionNotOngoing }
    // normal termination condition
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuota}

    fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { false }
    fn name() -> String { "IRV".to_string() }
    fn how_to_name_counts() -> CountNamingMethod { CountNamingMethod::SimpleNumber }

    fn sort_subcounts_by_count() -> Option<Box<dyn FnMut(&Transcript<Self::Tally>,<<Self as PreferenceDistributionRules>::SplitByNumber as HowSplitByCountNumber>::KeyToDivide,<<Self as PreferenceDistributionRules>::SplitByNumber as HowSplitByCountNumber>::KeyToDivide) -> Ordering>> {
        None
    }
    fn should_exhausted_votes_count_for_quota_computation() -> bool { false }
}