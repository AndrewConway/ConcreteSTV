// Copyright 2023 Andrew Conway, Alexander Ek.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use stv::ballot_pile::{BallotPaperCount, DoNotSplitByCountNumber};
use stv::preference_distribution::{BigRational, CountNamingMethod, LastParcelUse, PreferenceDistributionRules, SurplusTransferMethod, TransferValueMethod, WhenToDoElectCandidateClauseChecking};
use stv::tie_resolution::MethodOfTieResolution;
use stv::transfer_value::{convert_usize_to_rational, round_rational_down_to_usize, TransferValue};
use stv::fixed_precision_decimal::FixedPrecisionDecimal;

pub mod parse_minimal;


/// My guess at what the legislation means.
/// Appropriate legislation is the "Electoral Act 1907", Schedule 1, "Counting of votes at Legislative Council elections"
/// from which comments below are drawn.
pub struct Minimal {
}

impl PreferenceDistributionRules for Minimal {
    type Tally = FixedPrecisionDecimal<6>;

    // ???
    type SplitByNumber = DoNotSplitByCountNumber;

    // Don't use last parcel
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::No }
    // Do not exclude exhausted ballots in TV calculation
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverBallots }

    // Use float tallies and transfer values
    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { tally.to_rational() }
    fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { Self::Tally::from_rational_rounding_down(rational) }

    // Use float tallies and transfer values
    fn make_transfer_value(surplus: Self::Tally, ballots: BallotPaperCount) -> TransferValue {
        TransferValue::from_surplus(surplus.get_scaled_value() as usize,BallotPaperCount(ballots.0*(Self::Tally::SCALE as usize)))
    }
    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> Self::Tally {
        Self::Tally::from_scaled_value(transfer_value.mul_rounding_down(BallotPaperCount(ballots.0*(Self::Tally::SCALE as usize))) as u64)
    }

    // Transfer eveything in one go
    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::MergeSameTransferValuesAndScale }

    fn sort_exclusions_by_transfer_value() -> bool { false }

    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::None }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::None }
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }

    // Transfer all ballots in one go
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { false }
    fn check_elected_if_in_middle_of_exclusion() -> bool { false }

    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }

    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }

    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }

    /// No such clause
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never}

    /// No such clause
    fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { false }

    fn name() -> String { "Minimal".to_string() }
    fn how_to_name_counts() -> CountNamingMethod { CountNamingMethod::MajorMinor }

    fn major_count_if_someone_elected() -> bool { true }
}

