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
/// This ties to match what is actually used rather than the legislation. Differences are:
/// * Rounding in clause 10 is different: each multiplication is rounded down, other than the last which is rounded up. (actually better in many ways than the legislation)
/// * Clause 13 (exclusion) is inserted at the end of step 1, just after clause 6 (probably harmless other than affecting tie resolution)
///
/// There are some ambiguities I have not resolved, particularly in tie resolution
/// * Clause 23 seems to use an Oracle and I don't understand the timing.
/// * Is clause 43 an action (do this now) and/or a prescription (this is how you do it when referenced in clause 44,46,47)? That is, after doing 43 and 44 have you discarded 4 or 5 values?
///
/// I have found no real data to be able to work out what was done in practice for these ambiguities.
///
/// A.K.A. NOT YET SUITABLE FOR USE FOR CHECKING ELECTION RESULTS!
pub struct NZMeek {
}

impl PreferenceDistributionRules for NZMeek {
    type Tally = FixedPrecisionDecimal<9>;
    type KeepValueType = FixedPrecisionDecimal<9>;

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

    /// This is not done in the legislation (otherwise clause 13 would occur just after clause 6).
    /// However, this appears to happen in practice.
    ///
    /// #Evidence
    ///
    /// [https://www.dunedin.govt.nz/__data/assets/pdf_file/0009/1260459/Dunedin-City-Council-2025-Triennial-Elections-Final-STV-Result.pdf](Dunedin City Council 2025 Triennial Election),
    /// Strath Tairi Community board,
    /// In "Iteration 1", 3 candidates were elected with values well over quota.
    /// Every candidate had an integer number of votes incompatible with rules 7 to 10 having been applied.
    /// But still a candidate was excluded, presumably using clause 13.
    ///
    /// Extract from calculator commentary:
    /// ```text
    /// THOMAS, elected at iteration 1, reason: Candidate received 47.000000000 votes and quota was 41.000000001
    /// BAIN, elected at iteration 1, reason: Candidate received 45.000000000 votes and quota was 41.000000001
    /// THOMAS, elected at iteration 1, reason: Candidate received 42.000000000 votes and quota was 41.000000001
    /// RAMSAY, excluded at iteration 1, reason: Candidate received the lowest vote count 8.000000000, less than the
    /// second lowest by more than total surplus (10.999999997)
    /// ```
    ///
    /// I don't think this will change who is elected *unless tie resolution is used* in which case it will
    /// affect the count backs and the iteration number (which affects PRNs by clause 48).
    fn may_do_meek_exclusion_round_0() -> bool { true }
    /// whether Meek exclusion is done at the start of a count step
    fn do_meek_exclusion_at_start_of_count_step() -> bool { false }
    /// whether Meek exclusion is done at the end of a count step (as it is in New Zealand)
    fn do_meek_exclusion_just_after_quota_determination() -> bool { true }
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
    
    /// Not Applicable for Meek.
    fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { LastParcelUse::No }
    /// Not Applicable for Meek.
    fn transfer_value_method() -> TransferValueMethod { TransferValueMethod::SurplusOverContinuingBallotsLimitedToPriorTransferValue }
    /// Not Applicable for Meek.
    fn make_transfer_value(_surplus: Self::Tally, _ballots: BallotPaperCount) -> TransferValue { panic!("Should not make transfer values in Meek"); }
    fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { tally.to_rational()  }
    /// Not Applicable for Meek.
    fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { Self::Tally::from_rational_rounding_down(rational) }

    /// Not Applicable for Meek. Well, is used in first preference count, with TV 1.
    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> Self::Tally {
        assert!(transfer_value.is_one());
        ballots.into()
    }
    /// Not Applicable for Meek.
    fn check_elected_if_in_middle_of_surplus_distribution() -> bool { true }
    /// Not Applicable for Meek.
    fn check_elected_if_in_middle_of_exclusion() -> bool { true }
    /// Not Applicable for Meek.
    fn surplus_distribution_subdivisions() -> SurplusTransferMethod { SurplusTransferMethod::JustOneTransferValue }
    /// Not Applicable for Meek.
    fn sort_exclusions_by_transfer_value() -> bool { true }

    /// when_to_check_if_just_two_standing_for_shortcut_election not used. No such rule needed or even really appropriate.
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::None }
    /// No rules about order of election.
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::None }
    /// No rules about order of election.
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::None }
    /// ```
    /// ### Ties
    /// 19. This clause applies if a candidate with the lowest number of votes is to be excluded but 2 or more candidates share the lowest number of votes.
    /// If this clause applies, exclude the candidate identified by the AAFD method as the candidate to exclude.
    /// If the AAFD method does not identify a single candidate to exclude, exclude the candidate with the lowest PRN.
    /// ```
    /// The AAFD method is defined
    /// ```
    /// ### Ahead at first difference method (AAFD method)
    /// 40. To use the Ahead At First Difference Method determine which tied candidate, or candidates, 
    /// did not have more votes than another tied candidate at the earliest step at which the candidates had different numbers of votes. 
    /// If one candidate is identified, exclude him or her.
    /// ```
    /// This is not entirely clear. Do all candidates need to have different numbers? Presumably not or the
    /// phrase "or candidates" would not be necessary, and it would just say "determine which candidate had the fewest votes".
    /// So we want the MethodOfTieResolution::AnyDifferenceIsADiscriminatorGiveUpIfNotFullSolution, plus the NZ Meek PRN.
    ///
    /// Note that my implementation of this is not reliable. The legislation is (to me) ambiguous, and I could not find any useful data to test.
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminatorUseNZPRNsIfNotFullSolution }
    /// Not Applicable for Meek.
    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    /// Not Applicable for Meek.
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }
    /// Not Done.
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never }
    /// Paragraph 3 (extract)
    /// ```
    /// Counting is also complete when the number of successful candidates and hopeful candidates is equal to the number of vacancies. 
    /// In this case, the hopeful candidates become successful candidates.
    /// ```
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuota }
    /// Not Applicable for Meek.
    fn count_set_aside_due_to_transfer_value_limit_as_rounding() -> bool { true }
    /// No Such Rule
    fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::Never }

    fn name() -> String { "NZMeek".to_string() }
}

