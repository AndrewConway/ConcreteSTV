// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use stv::ballot_pile::{BallotPaperCount, FullySplitByCountNumber};
use stv::preference_distribution::{BigRational, CountNamingMethod, LastParcelUse, PreferenceDistributionRules, SurplusTransferMethod, TransferValueMethod, WhenToDoElectCandidateClauseChecking};
use stv::tie_resolution::MethodOfTieResolution;
use stv::transfer_value::{convert_usize_to_rational, round_rational_down_to_usize, TransferValue};

pub mod parse_wa;


/// My guess at what the legislation means.
/// Appropriate legislation is the "Electoral Act 1907", Schedule 1, "Counting of votes at Legislative Council elections"
/// from which comments below are drawn.
pub struct WALegislativeCouncil {
}

impl PreferenceDistributionRules for WALegislativeCouncil {
    type Tally = usize;

    /// 8(b)
    /// ```text
    /// (b) the total number (if any) of other votes obtained by the
    /// excluded candidate on transfers under this Schedule shall be
    /// transferred from the excluded candidate in the order of the
    /// transfers on which he obtained them, the votes obtained on the
    /// earliest transfer being transferred first, as follows —
    /// ```
    type SplitByNumber = FullySplitByCountNumber;

    /// 5(a) `divided by the number of votes received by him`
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::No }
    /// 5(a) `divided by the number of votes received by him`
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverBallots }

    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { convert_usize_to_rational(tally)  }
    fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { round_rational_down_to_usize(rational)  }

    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue {
        TransferValue::from_surplus(surplus,ballots)
    }

    /// 4(b) and 5(c) and 8b(iii), ...`(disregarding any fraction)`...
    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> usize {
        transfer_value.mul_rounding_down(ballots)
    }

    /// 5(c)(iii) `have a particular continued transfer value` somewhat implies that
    /// votes with the same transfer value are merged. I don't see any mention of
    /// order of transfer value.
    ///
    /// Looking at what the WAEC actually did, 2008 East Metropolitan region, at major count 28,
    /// surplus distribution for HARDEN, Alyssa, the distribution is divided up according to
    /// the count it was received at rather than by transfer value. In particular, the votes
    /// from 1.1, 6.1 and 7.1 (and many others) all have transfer value 1.
    ///
    /// So I have adopted what the WAEC actually did, as the legislation seems ambiguous.
    ///
    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::ScaleTransferValues }

    /// 8(b)
    /// ```text
    /// (b) the total number (if any) of other votes obtained by the
    /// excluded candidate on transfers under this Schedule shall be
    /// transferred from the excluded candidate in the order of the
    /// transfers on which he obtained them, the votes obtained on the
    /// earliest transfer being transferred first, as follows —
    /// ```
    /// They are done by originating count, not transfer value.
    fn sort_exclusions_by_transfer_value() -> bool { false }

    /// Section 12 says it is random.
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::None }
    /// Clause 15:
    /// ```text
    /// Subject to clause 16, where, after any count or transfer under this
    /// Schedule, 2 or more candidates have equal surpluses, the order of any
    /// transfers of the surplus votes of those candidates shall be in
    /// accordance with the relative numbers of votes of those candidates at
    /// the last count or transfer at which each of those candidates had a
    /// different number of votes, the surplus of the candidate with the largest
    /// number of votes at that count or transfer being transferred first, but if
    /// there has been no such count or transfer —
    /// (a) the returning officer shall make out in respect of each of those
    /// candidates, a slip bearing the name of the candidate, and deal
    /// with the slips in accordance with Schedule 2; and
    /// (b) the candidate whose name is on the slip obtained by the
    /// returning officer in accordance with clause 5 of Schedule 2
    /// shall, as between those candidates, be deemed to have had the
    /// largest surplus.
    /// ```
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    /// Not specified (section 13). Order elected is generally not mentioned in
    /// the legislation, and tie resolution is mentioned when it affects surplus distribution etc.
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::None }
    /// ```text
    /// Where the candidate who has the fewest votes is required to be
    /// excluded under clause 8 or 10, and 2 or more candidates (in this clause
    /// called the tied candidates) have an equal number of votes (each other
    /// candidate having a larger number of votes) whichever of the tied
    /// candidates had the fewest votes at the last count or transfer at which
    /// each of the tied candidates had a different number of votes shall be
    /// excluded, but if there has been no such count or transfer —
    /// ```
    /// continues like 15 to random lot.
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }

    /// 5 is ambiguous, but presumably by analogy with 9 the same thing is meant by a transfer
    /// ```text
    /// if on the completion of the transfer of the surplus votes of the
    /// elected candidate to a particular continuing candidate that candidate
    /// has received a number of votes equal to or greater than the quota, that
    /// candidate shall be elected.
    /// ```
    ///
    /// 19 tries to disambiguate:
    /// ```text
    /// For the purposes of this Schedule, a transfer under clause 4, 5 or 9 of
    /// all the surplus votes of an elected candidate, a transfer in accordance
    /// with clause 8(a) of all first preference votes of an excluded candidate
    /// or a transfer in accordance with clause 8(b) of all the votes of an
    /// excluded candidate that were transferred to him from a particular
    /// candidate each constitutes a separate transfer.
    /// ```
    /// This is pretty clearly support for not allowing elections in the middle of a surplus
    /// distribution.
    ///
    /// Furthermore, looking at what the WAEC have actually done,
    /// In 2018 East Metro, count 28.5, XAMON, Alison goes over quota
    /// but is not elected until much later when the surplus distribution is finished.
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { false }
    /// 9
    /// ```text
    /// Any continuing candidate who has received a number of votes equal
    /// to or greater than the quota on the completion of a transfer under
    /// clause 8 or 10 of votes of an excluded candidate shall be elected, and,
    /// unless all the vacancies have been filled, the surplus votes (if any) of
    /// the candidate so elected shall be transferred in accordance with
    /// clause 5, except that, where the candidate so elected is elected before
    /// all the votes of the excluded candidate have been transferred, the
    /// surplus votes (if any) of the candidate so elected shall not be
    /// transferred until the remaining votes of the excluded candidate have
    /// been transferred in accordance with clause 8(a) and (b) to continuing
    /// candidates.
    /// ```
    /// The explicit mention that it is possible for a candidate to be so elected before
    /// all the votes of the excluded candidate have been transferred makes it clear
    /// that one can get elected in the middle of an exclusion.
    ///
    /// Section 11 also makes this clear.
    ///
    /// Section 19 also makes this clear, but brings up another ambiguity:
    /// ```text
    /// For the purposes of this Schedule, a transfer under clause 4, 5 or 9 of
    /// all the surplus votes of an elected candidate, a transfer in accordance
    /// with clause 8(a) of all first preference votes of an excluded candidate
    /// or a transfer in accordance with clause 8(b) of all the votes of an
    /// excluded candidate that were transferred to him from a particular
    /// candidate each constitutes a separate transfer.
    /// ```
    /// This is pretty clearly support for allowing elections in the middle of an exclusion,
    /// but `from a particular candidate` is still ambiguous about
    /// how many transfers are done. If candidate E is being excluded, and candidate
    /// E got votes from candidate D in five different transfers, 8(b) implies that
    /// they are five different transfers, but 19 implies that they are 1 different
    /// transfer.
    ///
    fn check_elected_if_in_middle_of_exclusion() -> bool { true }

    /// There is no real mention of when the election ends, just things that should be done.
    /// So there is an argument that one should continue and do the things that are said to be done,
    /// there is another argument that the election is done. It is an insignificant matter anyway, so I
    /// have chosen to do what the WAEC did.
    ///
    /// 2008 Agricultural region ended at count 25.36, rather than finishing the exclusion of FELS, Anthony,
    /// when DAVIES, Mia went over quota.
    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }

    /// See comments on finish_all_counts_in_elimination_when_all_elected
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }


    /// This doesn't seem well defined by the legislation (clause 12). (and it does affect who is elected)
    /// The only sane thing is to wait until an exclusion is finished though.
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }

    /// Section 13:
    /// ```text
    /// Notwithstanding any other provision of this Schedule, where the
    /// number of continuing candidates is equal to the number of remaining
    /// unfilled vacancies, those candidates shall be elected.
    /// ```
    /// Section 2(1) says
    /// ```text
    /// continuing candidate means a candidate not already elected or not
    /// excluded from the count.
    /// ```
    /// Is a candidate excluded before or after his/her votes are distributed?
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }

    /// No such clause
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never}

    /// No such clause
    fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { false }

    fn name() -> String { "WA2008".to_string() }
    fn how_to_name_counts() -> CountNamingMethod { CountNamingMethod::MajorMinor }

    /// In 2008, South West region, at count 26.1 during the exclusion of SULLIVAN, Dan, a candidate HOLT, Colin reached quota. The next count was named 27.1 rather than 26.2
    fn major_count_if_someone_elected() -> bool { true }
}

