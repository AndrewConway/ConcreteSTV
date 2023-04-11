// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Rules for the NSW legislative council based on random sampling.

use std::marker::PhantomData;
use stv::ballot_pile::{BallotPaperCount, DoNotSplitByCountNumber};
use stv::preference_distribution::{BigRational, DeferSurplusDistribution, LastParcelUse, PreferenceDistributionRules, SurplusTransferMethod, TransferValueMethod, WhenToDoElectCandidateClauseChecking};
use stv::tie_resolution::MethodOfTieResolution;
use stv::transfer_value::{convert_usize_to_rational, TransferValue};

/// Many variants on the NSW randomized rules are used, partly due to them
/// fixing various bugs we pointed out, and partly due to the differences between
/// the Legislative Council (LC) legislation (constitution) and the Local Government Elections (LGE) legislation.
///
/// This encapsulates the differences between them.
pub trait NSWRandomVariations {

    fn name() -> String;

    /// From the "Functional Requirements for Count Module, 1.4.8 Step 8:
    /// '''text
    /// If the Election is a Legislative Council Election and there are 2 or more Candidates with equal
    /// Progressive Totals that exceed the Quota, then a draw must be conducted to determine the order of
    /// distributing surpluses for these Candidates.
    ///
    /// If the election is a Local Government Election and there are 2 or more Candidates with equal
    /// Progressive Totals that exceed the Quota, then the Count process must be reviewed to go back and
    /// determine the previous Count at which the Progressive Totals for these 2 or more Candidates were
    /// last unequal and elect first the Candidate with the highest Progressive Total at that point.
    /// If the 2 or more Candidates have had equal Progressive Totals at all preceding Counts (including
    /// Count 1), then a draw must be conducted to determine the order of distributing surpluses for these
    /// Candidates.
    /// '''
    ///
    /// This is ambiguous for LGEs, probably AnyDifferenceIsADiscriminator but unambiguous for LCs as None.
    ///
    /// We don't know if the 2016 NSWEC bug in resolving ties for exclusion (common) also applied to this (rare) case.
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution;

    /// Taken from "Functional Requirements for Count Module", 1.4.25, Step 25 Exclusion - Draw for Exclusion?
    /// ```text
    /// If the Election is a Legislative Council Election and there are 2 or more Candidates with equal lowest
    /// current Progressive Totals, then a draw must be conducted to determine the Candidate to be
    /// Excluded.
    ///
    /// If the Election is a Local Government Election and there are 2 or more Candidates with equal lowest
    /// current Progressive Totals, then the Count process must be reviewed to go back and determine the
    /// previous Count at which the Progressive Totals for these 2 or more Candidates were last unequal
    /// and Exclude the Candidate with the lowest Progressive Total at that point. If the 2 or more
    /// Candidates have had equal Progressive Totals at all preceding Counts (including Count 1), then a
    /// draw must be conducted to determine the Candidate to be Excluded.
    ///
    /// This is ambiguous for LGEs, probably AnyDifferenceIsADiscriminator but unambiguous for LCs as None.
    ///
    /// Furthermore, there was a bug we discovered in the NSWEC implementation - see report "2016 NSW LGE Errors.pdf"
    /// ```
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution;

    /// Taken from "Functional Requirements for Count Module", 1.4.12, Step 12 (see also 1.4.23)
    /// ```text
    /// a)For Legislative Council Elections, if the sum of all undistributed surplus votes from Elected
    /// Candidates is > or = to the difference between the Progressive Totals of the 2 Continuing
    /// Candidates with the lowest current Progressive Totals, then the next count is a Distribution of
    /// Surplus Votes. The process continues to Step 13.
    ///
    /// b)For Local Government Elections, if the sum of all undistributed surplus votes from Elected
    /// Candidates is > the difference between the Progressive Totals of the 2 Continuing Candidates
    /// with the lowest current Progressive Totals, then the next count is a Distribution of the Surplus
    /// Votes. The process continues to Step 13.
    ///
    /// c)Otherwise, the next count is an Exclusion and the distribution of any outstanding surplus’ from
    /// elected candidates is deferred. The process continues to Step 24.
    /// ```
    fn when_should_surplus_distribution_be_deferred() -> DeferSurplusDistribution;

    /// There was a bug we noticed in the 2012 LGE that probably resulted in the wrong person getting
    /// elected due to an error in this. See "LSWLGE2012CountErrorTechReport.pdf" for details.
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse;

    /// Whether computations should be done exactly or approximately. Needed to emulate the bug in the 2016 Bland Shire Council results,
    /// described in report "2016 NSW LGE Errors.pdf".
    fn use_f64_arithmetic_when_applying_transfer_values_instead_of_exact() -> bool;

    fn when_checking_if_top_few_have_overwhelming_votes_require_exactly_one() -> bool;
}

/*
 *
 *    Local Government Elections
 *
 */


/// How we think it should be - at least based upon the "Functional Requirements for Count Module"
pub struct NSWLGE{}
impl NSWRandomVariations for NSWLGE {
    fn name() -> String { "NSWrandomLGE".to_string() }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    /// This is one of the errors (and the only one I can emulate) in our report "2016 NSW LGE Errors.pdf", presumably present in 2012 and fixed in 2017.
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    fn when_should_surplus_distribution_be_deferred() -> DeferSurplusDistribution { DeferSurplusDistribution::DeferIfSumOfUndistributedSurplussesLessThanOrEqualToDifferenceBetweenTwoLowestContinuingCandidates }
    /// This is the error in our report "LSWLGE2012CountErrorTechReport.pdf", fixed after 2012.
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::LastPlusIfItWasSurplusDistributionPriorSurplusDistributionsWithoutAnyoneElected }
    fn use_f64_arithmetic_when_applying_transfer_values_instead_of_exact() -> bool { false }
    fn when_checking_if_top_few_have_overwhelming_votes_require_exactly_one() -> bool { false }
}
/// How we think it should be - at least based upon the "Functional Requirements for Count Module"
pub type NSWrandomLGE = NSWRandomSamplingVariant<NSWLGE>;

/// The count used by the NSWEC in 2012, as far as I can tell.
pub struct NSWECLGE2012{}
impl NSWRandomVariations for NSWECLGE2012 {
    fn name() -> String { "NSWECrandomLGE2012".to_string() }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    /// Assuming this bug was not newly created in 2016
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::None }
    fn when_should_surplus_distribution_be_deferred() -> DeferSurplusDistribution { DeferSurplusDistribution::DeferIfSumOfUndistributedSurplussesLessThanOrEqualToDifferenceBetweenTwoLowestContinuingCandidates }

    /// This is the error in our report "LSWLGE2012CountErrorTechReport.pdf"
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::LastPlusIfItWasSurplusDistributionPriorSurplusDistributionsWithoutAnyoneElectedPlusOneBonus }
    fn use_f64_arithmetic_when_applying_transfer_values_instead_of_exact() -> bool { true }
    fn when_checking_if_top_few_have_overwhelming_votes_require_exactly_one() -> bool { false }
}
pub type NSWECrandomLGE2012 = NSWRandomSamplingVariant<NSWECLGE2012>;

/// The count used by the NSWEC in 2016, as far as I can tell, other than the fact that they also sometimes got rounding wrong but not in a way I can predict and emulate.
pub struct NSWECLGE2016{}
impl NSWRandomVariations for NSWECLGE2016 {
    fn name() -> String { "NSWECrandomLGE2016".to_string() }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::None }
    fn when_should_surplus_distribution_be_deferred() -> DeferSurplusDistribution { DeferSurplusDistribution::DeferIfSumOfUndistributedSurplussesLessThanOrEqualToDifferenceBetweenTwoLowestContinuingCandidates }
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::LastPlusIfItWasSurplusDistributionPriorSurplusDistributionsWithoutAnyoneElected }
    fn use_f64_arithmetic_when_applying_transfer_values_instead_of_exact() -> bool { true }
    fn when_checking_if_top_few_have_overwhelming_votes_require_exactly_one() -> bool { false }
}
pub type NSWECrandomLGE2016 = NSWRandomSamplingVariant<NSWECLGE2016>;

/// The count used by the NSWEC in 2017, as far as I can tell.
/// Currently the same as NSWLGE.
pub struct NSWECLGE2017{}
impl NSWRandomVariations for NSWECLGE2017 {
    fn name() -> String { "NSWECrandomLGE2017".to_string() }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    fn when_should_surplus_distribution_be_deferred() -> DeferSurplusDistribution { DeferSurplusDistribution::DeferIfSumOfUndistributedSurplussesLessThanOrEqualToDifferenceBetweenTwoLowestContinuingCandidates }
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::LastPlusIfItWasSurplusDistributionPriorSurplusDistributionsWithoutAnyoneElected }
    fn use_f64_arithmetic_when_applying_transfer_values_instead_of_exact() -> bool { false }
    fn when_checking_if_top_few_have_overwhelming_votes_require_exactly_one() -> bool { false }
}
pub type NSWECrandomLGE2017 = NSWRandomSamplingVariant<NSWECLGE2017>;


/*
 *
 *    Legislative Council Elections
 *
 */

pub struct NSWLC{}
impl NSWRandomVariations for NSWLC {
    fn name() -> String { "NSWrandomLC".to_string() }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::None }
    /// This is one of the errors (and the only one I can emulate) in our report "2016 NSW LGE Errors.pdf", presumably present in 2012 and fixed in 2017.
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::None }
    fn when_should_surplus_distribution_be_deferred() -> DeferSurplusDistribution { DeferSurplusDistribution::DeferIfSumOfUndistributedSurplussesLessThanDifferenceBetweenTwoLowestContinuingCandidates }
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::LastPlusIfItWasSurplusDistributionPriorSurplusDistributionsWithoutAnyoneElected }
    fn use_f64_arithmetic_when_applying_transfer_values_instead_of_exact() -> bool { true }
    fn when_checking_if_top_few_have_overwhelming_votes_require_exactly_one() -> bool { true }
}
/// How we think it should be - at least based upon the "Functional Requirements for Count Module"
pub type NSWrandomLC = NSWRandomSamplingVariant<NSWLC>;





pub struct NSWRandomSamplingVariant<V:NSWRandomVariations> {
    phantom : PhantomData<V>
}

impl <V:NSWRandomVariations> PreferenceDistributionRules for NSWRandomSamplingVariant<V> {
    type Tally = usize;
    type SplitByNumber = DoNotSplitByCountNumber;

    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { V::use_last_parcel_for_surplus_distribution() }
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverContinuingBallots }

    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { convert_usize_to_rational(tally)  }
    fn convert_rational_to_tally_after_applying_transfer_value(_rational: BigRational) -> Self::Tally { panic!("NSW Random sampling never does conversion to rational")  }

    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue {
        if surplus>=ballots.0 { TransferValue::one() }
        else { TransferValue::from_surplus(surplus,ballots) }
    }

    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> usize {
        transfer_value.mul_rounding_down(ballots)
    }

    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::PickRandomlyAfterDistribution }
    fn sort_exclusions_by_transfer_value() -> bool { false }

    // NA
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::None }
    /// From the "Functional Requirements for Count Module, 1.4.8 Step 8:
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { V::resolve_ties_elected_by_quota() }
    /// There doesn't seem to be mention of this in the functional requirements document, either in section 1.4.10 or 1.4.30.
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::None }

    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { V::resolve_ties_choose_lowest_candidate_for_exclusion() }

    /// NA as there is no middle
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { false }
    /// A count can be interrupted in before papers are distributed if the rest are elected.
    fn check_elected_if_in_middle_of_exclusion() -> bool { true }

    /// NA as there is no middle
    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    /// The functional requirements are pretty clear on this.
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }


    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never }

    /// Section 1.4.11 (after quota) and 1.4.27 (after exclusion, before doing distribution or checking quota)
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterDeterminingWhoToExcludeButBeforeTransferringAnyPapersOrQuotaButOnlyIfContinuingCandidatesEqualsUnfilledVacanciesAndNotAfterSurplusIfMoreSurplusesAvailable }

    /// Section 14.4.11 says do this for LC for 1 vote, and LGE for any number
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterDeterminingWhoToExcludeButBeforeTransferringAnyPapersOrQuotaButOnlyIfContinuingCandidatesEqualsUnfilledVacanciesAndNotAfterSurplusIfMoreSurplusesAvailable}
    /// See comment for [when_to_check_if_top_few_have_overwhelming_votes()]
    fn when_checking_if_top_few_have_overwhelming_votes_require_exactly_one() -> bool { V::when_checking_if_top_few_have_overwhelming_votes_require_exactly_one() }

    /// Taken from "Functional Requirements for Count Module", 1.4.12
    fn when_should_surplus_distribution_be_deferred() -> DeferSurplusDistribution { V::when_should_surplus_distribution_be_deferred() }

    /// 2016 LGE Ballina Shire Council - C Ward there is one vote that is exhausted on round 1, and if it were used the quota would be 1 higher.
    fn should_exhausted_votes_count_for_quota_computation() -> bool { false }
    fn use_f64_arithmetic_when_applying_transfer_values_instead_of_exact() -> bool { V::use_f64_arithmetic_when_applying_transfer_values_instead_of_exact() }

    fn name() -> String { V::name() }
}
