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

use stv::preference_distribution::{BigRational, CountNamingMethod, PreferenceDistributionRules, SurplusTransferMethod, TransferValueMethod, WhenToDoElectCandidateClauseChecking};
use stv::ballot_pile::{BallotPaperCount, FullySplitByCountNumber};
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

