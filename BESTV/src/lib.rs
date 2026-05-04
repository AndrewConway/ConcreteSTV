// Copyright 2023 Andrew Conway, Alexander Ek.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use stv::ballot_pile::{BallotPaperCount, DoNotSplitByCountNumber};
use stv::preference_distribution::{BigRational, CountNamingMethod, LastParcelUse, PreferenceDistributionRules, SurplusTransferMethod, TransferValueMethod, WhenToDoElectCandidateClauseChecking};
use stv::tie_resolution::MethodOfTieResolution;
use stv::transfer_value::{TransferValue};
use stv::fixed_precision_decimal::FixedPrecisionDecimal;

/// A best-effort set of simple STV rules, to match the inferences in the STV margin paper by
/// Blom, Ek, Stuckey, Teague and Vukcevic.
/// These rules include
/// - transfer by WIGM, with BigRational weights and transfer values,
/// - minimal count splitting,
/// - completing all surplus distributions and exclusions before checking whether any candidate is over quota.
/// There is no parser associated with this rule set, because this is an idealised STV version
/// not taken from any actual jurisdiction.
pub struct beSTV {
}

impl PreferenceDistributionRules for beSTV {

    /// Don't break up piles of ballots according to the count number they were transferred in.
    type SplitByNumber = DoNotSplitByCountNumber;
    /// Don't use last parcel
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::No }

    /// Do not exclude exhausted ballots in TV calculation. This is the 'I' in WIGM.
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverBallots }

    /// Use fixed-precision tallies and transfer values
    type Tally = FixedPrecisionDecimal<6>;
    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { tally.to_rational() }
    fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { Self::Tally::from_rational_rounding_down(rational) }

    /// Scale transfer values, so that a ballot's weight is the _product_ of its prior weight and
    /// the current transfer value. This is the 'W' in WIGM.
    fn make_transfer_value(surplus: Self::Tally, ballots: BallotPaperCount) -> TransferValue {
        TransferValue::from_surplus(surplus.get_scaled_value() as usize,BallotPaperCount(ballots.0*(Self::Tally::SCALE as usize)))
    }
    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> Self::Tally {
        Self::Tally::from_scaled_value(transfer_value.mul_rounding_down(BallotPaperCount(ballots.0*(Self::Tally::SCALE as usize))) as u64)
    }

    /// Transfer everything in one go
    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::MergeSameTransferValuesAndScale }

    fn sort_exclusions_by_transfer_value() -> bool { false }

    /// Rather arbitrary tie resolution, which certainly can make a difference, but hopefully only
    /// in very rare cases.
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::None }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::None }
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }

    /// Transfer all ballots in one go
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { false }
    fn check_elected_if_in_middle_of_exclusion() -> bool { false }

    /// Shortcutting and finishing rules. Shouldn't make any difference.
    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never}
    fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { false }

    fn name() -> String { "BESTV".to_string() }
    fn how_to_name_counts() -> CountNamingMethod { CountNamingMethod::MajorMinor }

    fn major_count_if_someone_elected() -> bool { true }
}

