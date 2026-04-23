// Copyright 2026 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use stv::ballot_metadata::NumberOfCandidates;
use stv::ballot_pile::{BallotPaperCount, DoNotSplitByCountNumber};
use stv::fixed_precision_decimal::FixedPrecisionDecimal;
use stv::preference_distribution::{BigRational, LastParcelUse, PreferenceDistributionRules, SurplusTransferMethod, TransferValueMethod, WhenToDoElectCandidateClauseChecking};
use stv::tie_resolution::MethodOfTieResolution;
use stv::transfer_value::TransferValue;

/// This is a preliminary attempt at the New Zealand Meek STV algorithm.
/// See  [Schedule 1A (New  Zealand method of counting single transferable votes), Local Electoral Regulations 2001, (SR 2001/145), Version as at 1 July 2025](https://www.legislation.govt.nz/secondary-legislation/pco-drafted/2001/145/en/latest/#DLM57125)
///
/// It still does not implement many significant features of the NZ system but
/// is a placeholder for architectural changes allowing Meek style algorithms.
///
/// A.K.A. NOT SUITABLE FOR USE FOR CHECKING ELECTION RESULTS!
pub struct NZMeek {
}

impl PreferenceDistributionRules for NZMeek {
    type Tally = FixedPrecisionDecimal<9>;
    type KeepValueType = FixedPrecisionDecimal<9>; // TODO Note that this does not handle the requirement to always round up in the legislation for multiplication. Instead I have implemented a (IMO more sensible) approach that seems to have been used in the one instance I know of.

    type SplitByNumber = DoNotSplitByCountNumber;

    /// Part 1, Step 1, Clause 5:
    /// Calculate a quota using the following formula:
    ///
    /// q = (v − vnt) ÷ (n + 1) + 0.000 000 001
    ///
    /// where—
    ///        - **q** is the quota
    ///        - **v** is the total number of valid voting documents
    ///        - **vnt** is the number of non transferable votes
    ///        - **n** is the number of vacancies
    ///
    /// and q is truncated to 9 decimal digits after the point with no rounding.
    fn compute_quota_formula(total_first_preferences:BallotPaperCount,non_transferred_votes:Self::Tally,candidates_to_be_elected:NumberOfCandidates) -> Self::Tally {
        let v : Self::Tally = total_first_preferences.into();
        (v-non_transferred_votes)/(1+candidates_to_be_elected.0)+Self::Tally::from_scaled_value(1)
    }
    fn is_meek_method() -> bool { true }

    /// At what point the candidate with the lowest tally should be excluded, if doing Meek style
    /// iteration. This is irrelevant if not doing Meek style STV.
    /// * total_surplus is the sum of the surplus of each of the successful candidates.
    /// * lowest_tally is the tally of the candidate with the lowest tally
    /// * second_lowest_tally is the tally of the candidate with the second-lowest tally, should there be more than one hopeful candidate.
    ///
    /// 13. Exclude the hopeful candidate with the least votes if the sum of his or her votes and the total surplus
    ///     is less than the votes of any other hopeful candidate or if the total surplus is less than 0.0001.
    fn should_exclude_lowest_candidate_meek_method(total_surplus:Self::Tally,lowest_tally:Self::Tally,second_lowest_tally:Option<Self::Tally>) -> bool {
        if let Some(second_lowest_tally) = second_lowest_tally && lowest_tally+total_surplus<second_lowest_tally { return true}
        total_surplus<Self::Tally::from_scaled_value(100000)
    }

    /// FIXME have not checked the things below at all.
    /// FIXME deal with NZ surplus ordering
    /// FIXME deal with NZ PRNG
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::No }
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverContinuingBallotsLimitedToPriorTransferValue }
    fn make_transfer_value(surplus: Self::Tally, ballots: BallotPaperCount) -> TransferValue {
        TransferValue::from_surplus(surplus.get_scaled_value() as usize,BallotPaperCount(ballots.0*(Self::Tally::SCALE as usize)))
    }
    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { tally.to_rational()  }
    fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { Self::Tally::from_rational_rounding_down(rational) }

    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> Self::Tally {
        Self::Tally::from_scaled_value(transfer_value.mul_rounding_down(BallotPaperCount(ballots.0*(Self::Tally::SCALE as usize))) as u64)
    }
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { true } // not applicable as distribute_surplus_all_with_same_transfer_value.
    fn check_elected_if_in_middle_of_exclusion() -> bool { true }
    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::JustOneTransferValue }
    fn sort_exclusions_by_transfer_value() -> bool { true }

    // all below same as ACTpre2020.
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::None }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never }
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }
    fn count_set_aside_due_to_transfer_value_limit_as_rounding() -> bool { true }
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never }

    fn name() -> String { "NZMeek".to_string() }
}

