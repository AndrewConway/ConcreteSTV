// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use stv::ballot_pile::{BallotPaperCount, SplitFirstCount};
use stv::preference_distribution::{BigRational, CountNamingMethod, PreferenceDistributionRules, SurplusTransferMethod, TransferValueMethod, WhenToDoElectCandidateClauseChecking};
use stv::tie_resolution::MethodOfTieResolution;
use stv::transfer_value::{convert_usize_to_rational, round_rational_down_to_usize, TransferValue};

pub mod parse_vic;


/// My guess at what the legislation means.
/// Appropriate legislation is the Electoral Act 2002, Section 114A.
/// Called 2018 as the legislation changed 114A(28)(c) in 2018 to match this behaviour, although
/// it was used by the VEC in 2014 anyway IMAO.
pub struct Vic2018LegislativeCouncil {
}

impl PreferenceDistributionRules for Vic2018LegislativeCouncil {
    type Tally = usize;
    /// Sections (12) and (28)
    /// Prior to a legislation modification in 2018, `S. 114A(28)(c) substituted by No. 30/2018 s. 36.`
    /// it implied that there should be split by every count, although the VEC in practice only split by
    /// the first count. The legislation was changed to match what the VEC did (and what I
    /// expect the legislators probably intended based on the wording of (12).
    type SplitByNumber = SplitFirstCount;

    fn use_last_parcel_for_surplus_distribution() -> bool { false }
    /// (7)(a)
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverBallots }

    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { convert_usize_to_rational(tally)  }
    fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { round_rational_down_to_usize(rational)  }

    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue {
        TransferValue::from_surplus(surplus,ballots)
    }

    /// 12(b)(ii) ...`(disregarding any fraction)`...
    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> usize {
        transfer_value.mul_rounding_down(ballots)
    }

    /// All done by clause 7.
    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::JustOneTransferValue }

    fn sort_exclusions_by_transfer_value() -> bool { true }

    /// ```text
    /// (25) If on the final count or transfer 2 candidates have
    /// an equal number of votes, the result is to be
    /// determined by lot by the election manager.
    /// ```
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::None }
    /// ```text
    /// (21) Subject to subsection (23), if after any count or
    /// transfer, 2 or more candidates have equal
    /// surpluses, the order of any transfers of the surplus
    /// votes of those candidates is to be in accordance
    /// with the relative numbers of votes of those
    /// candidates at the last count or transfer at which
    /// each of those candidates had a different number of
    /// votes, the surplus of the candidate with the largest
    /// number of votes at that count or transfer being
    /// transferred first.
    /// ```
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    /// Not specified
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::None }
    /// ```text
    /// (24) If on any count or transfer 2 or more candidates
    /// have the fewest number of votes and the candidate
    /// who has the fewest number of votes is required to
    /// be excluded, the result is to be determined—
    /// (a) by declaring whichever of those candidates
    ///     had the fewest votes at the last count at
    ///     which those candidates had a different
    ///     number of votes to be excluded; or
    /// (b) if a result is still not obtained or there has
    ///     been no count or transfer, by lot by the
    ///     election manager.
    /// ```
    /// This is not as unambiguous as section (21) which explicitly state `each of` before `those candidates had a different number of votes`
    /// but the implication that there is a well defined `whichever of those candidates had the fewest votes` is something.
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }

    /// No such middle.
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { false }
    ///
    /// ```text
    /// (13) Any continuing candidate who has received a
    /// number of votes equal to or greater than the quota
    /// on the completion of a transfer of votes of an
    /// excluded candidate under subsection (12) or (16)
    /// is to be declared elected by the election manager.
    /// ```
    ///
    /// A transfer is made clear by
    /// ```text
    /// (28) For the purposes of this section each of the
    /// following constitutes a separate transfer—
    /// (a) a transfer under subsection (7), (9) or (14) of
    ///     all the surplus votes of an elected candidate;
    /// (b) a transfer in accordance with
    ///     subsection (12)(a) of all first preference
    ///     votes of an excluded candidate;
    /// (c) a transfer to a candidate in accordance with
    ///     subsection (12)(b) of all of the votes of an
    ///     excluded candidate or candidates, as the
    ///     case may be, at a particular transfer value.
    /// ```
    ///
    /// ```text
    /// (15) If a candidate elected under subsection (13) is
    /// elected before all the votes of the excluded
    /// candidate have been transferred, the surplus votes,
    /// if any, of the elected candidate are not to be
    /// transferred until the remaining votes of the
    /// excluded candidate have been transferred in
    /// accordance with subsection (12) to continuing
    /// candidates.
    /// ```
    fn check_elected_if_in_middle_of_exclusion() -> bool { true }

    /// This is not at all clear. Section 12 says `all that candidate's votes are to be transferred` implying it should be finished.
    /// but sections (19) may shortcut this by `Despite any other provision of this section`.
    /// It can't change who is elected or what order, and the VEC has interpreted it as not to continue
    /// * evidence : 2014, northern metropolitan region, ended on count 263 in the middle
    ///   of eliminating YIGIT, Burhan when there were still 171 votes with TV≈0.0437416 to distribute.
    /// So I have adopted the same interpretation.
    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }

    /// The clause talking about distribution starts:
    /// ```text
    /// (7) Unless all the vacancies have been filled, the
    // surplus votes of each elected candidate are to be
    // transferred to the continuing candidates as
    // follows—
    /// ```
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }


    /// This doesn't seem well defined by the legislation. (and it does affect who is elected)
    /// The only sane thing is to wait until an exclusion is finished though.
    /// ```text
    /// (18) In respect of the last vacancy for which
    /// 2 continuing candidates remain, the continuing
    /// candidate who has the larger number of votes is
    /// to be elected notwithstanding that that number is
    /// below the quota.
    /// ```
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfExclusionNotOngoing }

    /// This doesn't seem well defined by the legislation. (but it doesn't affect who is elected)
    /// ```text
    /// (19) Despite any other provision of this section, if the
    /// number of continuing candidates is equal to the
    /// number of remaining unfilled vacancies, those
    /// candidates are to be declared elected by the
    /// election manager.
    /// ```
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuota }

    /// No such clause
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never}

    /// No such clause
    fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { false }

    fn name() -> String { "Vic2018".to_string() }
    fn how_to_name_counts() -> CountNamingMethod { CountNamingMethod::SimpleNumber }
}

