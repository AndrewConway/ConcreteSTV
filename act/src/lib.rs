// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use stv::preference_distribution::{PreferenceDistributionRules, WhenToDoElectCandidateClauseChecking, TransferValueMethod, BigRational, SurplusTransferMethod, LastParcelUse};
use stv::tie_resolution::MethodOfTieResolution;
use stv::transfer_value::{TransferValue, convert_usize_to_rational, round_rational_down_to_usize};
use stv::ballot_pile::{BallotPaperCount, DoNotSplitByCountNumber, SplitByWhenTransferValueWasCreated};
use stv::fixed_precision_decimal::FixedPrecisionDecimal;

pub mod parse;


/// The rules used pre2020 for the ACT Legislative Assembly, when votes were integers
pub struct ACTPre2020 {
}

impl PreferenceDistributionRules for ACTPre2020 {
    type Tally = usize;
    type SplitByNumber = DoNotSplitByCountNumber;

    /// See below comment, (5)
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::LiterallyLast }

    /// Electoral Act 1992, Schedule 4, 1C
    /// ```text
    /// Meaning of transfer value—sch 4
    /// (1) For this schedule, the transfer value of a ballot paper is the transfer
    ///     value worked out under this clause.
    ///     Note Transfer value, for pt 4.3 (Casual vacancies)—see cl 13.
    /// (2) For the allotment of votes from the surplus of a successful candidate,
    ///     the transfer value of a ballot paper that specifies a next available
    ///     preference is worked out as follows:
    ///          S / CP [ edited for ASCII ]
    /// (3) For the allotment of votes under clause 9 (2) (c) (Votes of excluded
    ///     candidates), the transfer value is—
    ///    (a) for a ballot paper in relation to which votes were allotted to the
    ///        excluded candidate under clause 3 (First preferences)—1; or
    ///    (b) for a ballot paper in relation to which count votes were allotted
    ///        to the excluded candidate under clause 6 (3) (Surplus votes) or
    ///        clause 9 (2) (c) (Votes of excluded candidates)—the transfer
    ///        value of the ballot paper when counted for that allotment.
    /// (4) However, if the transfer value of a ballot paper worked out in
    ///     accordance with subclause (2) would be greater than the transfer
    ///     value of the ballot paper when counted for the successful candidate,
    ///     the transfer value of that ballot paper is the transfer value of the ballot
    ///     paper when counted for the successful candidate.
    /// (5) In this clause:
    ///     CP means the number of ballot papers counted for the candidate at
    ///        the count at which the candidate became successful and that specify
    ///        a next available preference.
    ///     S means the surplus.
    /// ```
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverContinuingBallotsLimitedToPriorTransferValue }

    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue {
        TransferValue::from_surplus(surplus,ballots)
    }
    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { convert_usize_to_rational(tally)  }
    fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { round_rational_down_to_usize(rational)  }

    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> usize {
        transfer_value.mul_rounding_down(ballots)
    }
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { true } // not applicable as distribute_surplus_all_with_same_transfer_value.
    fn check_elected_if_in_middle_of_exclusion() -> bool { true }
    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::JustOneTransferValue }
    fn sort_exclusions_by_transfer_value() -> bool { true }

    /// Not applicable.
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::None }
    /// Electoral Act 1992, Part 4.2 7 (3)(c)
    /// ```text
    /// if 2 or more successful candidates (contemporary candidates)
    /// who obtained a quota at the earliest count have the same surplus,
    /// being a surplus larger than that of any other candidate who
    /// obtained a quota at the count and—
    /// (i) 1 of the contemporary candidates had more total votes than
    ///     any other contemporary candidate at the last count—that
    ///     candidate is the relevant candidate; or
    /// (ii) 2 or more contemporary candidates have the same total
    ///      votes, being a total larger than that of any other
    ///      contemporary candidate (a non-tied contemporary
    ///      candidate) at the last count—each non-tied contemporary
    ///      candidate is no longer considered under this clause and—
    ///      (A) subparagraph (i) and this subparagraph are applied to
    ///          each preceding count until a relevant candidate is
    ///          worked out; or
    ///      (B) if a relevant candidate cannot be worked out by
    ///          applying subparagraph (i) and this subparagraph to
    ///          the preceding count—the contemporary candidate
    ///          who is determined by the commissioner by lot is the
    ///          relevant candidate.
    /// ```
    /// Note that this applies to the order of surplus distribution, which I am assuming is
    /// also the order of election.
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    /// The act doesn't really talk about order of election, which doesn't appear to matter. So anything is OK, may as well be same as others.
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    /// Electoral Act 1992, Part 4.2 8 (2) is very similar to 7(3)(c) except reversed as it deals with identifying lowest rather than highest.
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }



    /// Electoral Act 1992, Part 4.2 4 (1)
    /// ```text
    /// If, after a calculation under clause 3 (3), 6 (4) or 9 (2) (d), the number
    /// of successful candidates is equal to the number of positions to be
    /// filled, the scrutiny shall cease.
    /// ```
    /// 3(3) is first preferences
    /// 6(4) is surplus distribution
    /// 9(2)(d) is excluded candidates, for a single transfer value.
    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }


    /// The ACT legislation is rather minimilist, and has no such rule.
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never }


    /// Electoral Act 1992, Part 4.2 4 (2)
    /// ```text
    /// If, after a calculation under clause 3 (3) or 6 (4) or after all the ballot
    /// papers counted for an excluded candidate have been dealt with under
    /// clause 9—
    /// (a) the number of continuing candidates is equal to the number of
    /// positions remaining to be filled; and
    /// (b) no successful candidate has a surplus not already dealt with
    /// under clause 6;
    /// each of those continuing candidates is successful and the scrutiny
    /// shall cease.
    /// ```
    /// 3(3) is first preferences
    /// 6(4) is surplus distribution
    /// 9(2)(d) is excluded candidates, for a single transfer value.
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never }

    /// If the TV calculation is limited due to incoming TV (such as in ACT) this causes votes to be set aside.
    /// These will normally be counted as set aside, but Elections ACT counts them as lost to rounding.
    /// I presume this is because the sane way to compute rounding is to compute the total votes in and subtract votes out, and if you forget about set aside votes, they go to rounding, so this bizarre decision could just be forgetting to deal with them. Then when someone asked why the votes lost due to rounding was so big, and realised why, they maybe decided it was not worth adding a new column for votes set aside, and rationalized not doing anything about it. Just a guess.
    /// Anyway, it doesn't really matter, there is no legislative requirement to count rounding. Although it would be darkly amusing if the rules changes to truncate to 6 decimal digits instead of to an integer was caused by seeing a large number of votes ostensibly lost due to rounding and wanting to do something about it. Just a guess.
    fn count_set_aside_due_to_transfer_value_limit_as_rounding() -> bool { true }

    fn name() -> String { "ACTPre2020".to_string() }

}



/// The rules used after the 2020 changes
///   * Votes should be rounded down to 6 decimal places rather than an integer
///   * The legislation has a probably unintended situation whereby a surplus less than
///     1 is not considered a surplus by a literal reading. ElectionsACT looked at this
///     carefully and concluded that the intention was to count it as a surplus. It
///     certainly seems to me as if that not adjusting the ">=1" clause to ">0" was an
///     unintentional oversight by the people writing the legislation. So I think that
///     ElectionACT's position on this is reasonable, and I will do the same.
///
/// This is labeled ACT2021 as ElectionsACT didn't actually use these rules in 2020, but
/// had three classes of bugs. After we pointed them out, they denied the worst, but then
/// in 2021 quietly fixed them and replaced their transcript of distributions of preferences on
/// their website, and used the corrected rules when an elected candidate had to be replaced in 2021.
pub struct ACT2021 {
}

impl PreferenceDistributionRules for ACT2021 {
    type Tally = FixedPrecisionDecimal<6>;
    type SplitByNumber = DoNotSplitByCountNumber;

    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::LiterallyLast }
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

    fn name() -> String { "ACT2021".to_string() }
}


/// The rules used by ElectionsACT in 2020, as best I can reverse engineer.
/// Like ACT2021, except
///  * Round to nearest instead of down
///  * Round transfer values to six digits if rule 1C(4) applies.
///  * Count transfer values computed in rule 1C(4) as having a different value to all other transfer values with the same value.
///  * Round exhausted votes to an integer when doing exclusions (instead of 6 decimal places). This can't change who is elected, just the transcript.
///  * Surplus distribution is completed even after everyone is elected. This can't change who is elected, just the transcript.
/// See our report for more details.
pub struct ACT2020 {
}

impl PreferenceDistributionRules for ACT2020 {
    type Tally = FixedPrecisionDecimal<6>;
    /// * Count transfer values computed in rule 1C(4) as having a different value to all other transfer values with the same value.
    /// E.g. Ginninderra Count 39
    type SplitByNumber = SplitByWhenTransferValueWasCreated;

    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::LiterallyLast }
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverContinuingBallotsLimitedToPriorTransferValue }
    fn make_transfer_value(surplus: Self::Tally, ballots: BallotPaperCount) -> TransferValue {
        TransferValue::from_surplus(surplus.get_scaled_value() as usize,BallotPaperCount(ballots.0*(Self::Tally::SCALE as usize)))
    }
    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { tally.to_rational()  }
    fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { Self::Tally::from_rational_rounding_down(rational) }

    /// Round to nearest instead of down
    /// E.g. Murrumbidgee count 22
    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> Self::Tally {
        Self::Tally::from_scaled_value(transfer_value.mul_rounding_nearest(BallotPaperCount(ballots.0*(Self::Tally::SCALE as usize))) as u64)
    }
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { true } // not applicable as distribute_surplus_all_with_same_transfer_value.
    fn check_elected_if_in_middle_of_exclusion() -> bool { true }
    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::JustOneTransferValue }
    fn sort_exclusions_by_transfer_value() -> bool { true }

    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::None }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }
    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    /// Surplus distribution is completed even after everyone is elected. This can't change who is elected, just the transcript.
    fn finish_all_surplus_distributions_when_all_elected() -> bool { true }
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never }
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }
    fn count_set_aside_due_to_transfer_value_limit_as_rounding() -> bool { true }

    /// Round exhausted votes to an integer when doing exclusions (instead of 6 decimal places).
    /// e.g. Ginninderra count 25
    fn munge_exhausted_votes(exhausted:Self::Tally,is_exclusion:bool) -> Self::Tally { if is_exclusion { exhausted.round_down() } else {exhausted} }

    /// Round transfer values to 6 decimal places when rule 1C(4) is used.
    /// e.g Murrumbidgee count 32
    fn munge_transfer_value_when_used_as_limit(original:TransferValue) -> TransferValue {
        let num = original.mul_rounding_nearest(BallotPaperCount(1000000));
        TransferValue::from_surplus(num,BallotPaperCount(1000000))
    }
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never }

    fn name() -> String { "ACT2020".to_string() }
}
