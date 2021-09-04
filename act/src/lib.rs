// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use stv::preference_distribution::{PreferenceDistributionRules, WhenToDoElectCandidateClauseChecking, TransferValueMethod};
use stv::tie_resolution::MethodOfTieResolution;
use stv::transfer_value::{TransferValue};
use stv::ballot_pile::{BallotPaperCount, DoNotSplitByCountNumber};

pub mod parse;
mod test_act;


/// The rules used pre2020, when votes were integers
pub struct ACTPre2020 {
}

impl PreferenceDistributionRules for ACTPre2020 {
    type Tally = usize;
    type SplitByNumber = DoNotSplitByCountNumber;

    /// See below comment, (5)
    fn use_last_parcel_for_surplus_distribution() -> bool { true }

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

    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> usize {
        transfer_value.mul_rounding_down(ballots)
    }

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

    /// If the TV calculation is limited due to incoming TV (such as in ACT) this causes votes to be set aside.
    /// These will normally be counted as set aside, but Elections ACT counts them as lost to rounding.
    /// I presume this is because the sane way to compute rounding is to compute the total votes in and subtract votes out, and if you forget about set aside votes, they go to rounding, so this bizarre decision could just be forgetting to deal with them. Then when someone asked why the votes lost due to rounding was so big, and realised why, they maybe decided it was not worth adding a new column for votes set aside, and rationalized not doing anything about it. Just a guess.
    /// Anyway, it doesn't really matter, there is no legislative requirement to count rounding. Although it would be darkly amusing if the rules changes to truncate to 6 decimal digits instead of to an integer was caused by seeing a large number of votes ostensibly lost due to rounding and wanting to do something about it. Just a guess.
    fn count_set_aside_due_to_transfer_value_limit_as_rounding() -> bool { true }

    fn name() -> String { "ACTPre2020".to_string() }
}
