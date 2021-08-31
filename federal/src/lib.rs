// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Describe the rules used for Federal elections, as best I can tell.

use stv::preference_distribution::{PreferenceDistributionRules, WhenToDoElectCandidateClauseChecking};
use stv::ballot_pile::{BallotPaperCount, DoNotSplitByCountNumber};
use stv::transfer_value::{TransferValue, LostToRounding};
use stv::tie_resolution::MethodOfTieResolution;

pub mod parse;
mod test_federal;
pub mod parse2013;

pub struct FederalRules {
}

impl PreferenceDistributionRules for FederalRules {
    type Tally = usize;
    type SplitByNumber = DoNotSplitByCountNumber;

    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue {
        TransferValue::from_surplus(surplus,ballots)
    }

    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> (usize, LostToRounding) {
        transfer_value.mul_rounding_down(ballots)
    }

    /// Require that at some prior point *all* the counts were different
    /// ```text
    /// Commonwealth Electoral Act 1918, Section 273, 20(b) extract:
    ///
    ///   ...if any 2 or more of
    /// those candidates each have the same number of votes, the
    /// order in which they shall be taken to have been elected shall
    /// be taken to be in accordance with the relative numbers of
    /// their votes at the last count before their election at which
    /// each of them had a different number of votes, the candidate
    /// with the largest number of votes at that count being taken to
    /// be the earliest elected, and if there has been no such count the
    /// Australian Electoral Officer for the State shall determine the
    /// order in which they shall be taken to have been elected.
    /// ```
    /// Order of surplus distribution is basically the same.
    /// Technically the EC could make a different decision if it wanted to be perverse.
    /// ```text
    /// Commonwealth Electoral Act 1918, Section 273, 22:
    /// Subject to subsection (23), where, after any count under this
    /// section, 2 or more candidates have equal surpluses, the order of
    /// any transfers of the surplus votes of those candidates shall be in
    /// accordance with the relative numbers of votes of those candidates
    /// at the last count at which each of those candidates had a different
    /// number of votes, the surplus of the candidate with the largest
    /// number of votes at that count being transferred first, but if there
    /// has been no such count the Australian Electoral Officer for the
    /// State shall determine the order in which the surpluses shall be dealt
    /// with.
    ///```
    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    /// same as [resolve_ties_elected_one_of_last_two]
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    /// same as [resolve_ties_elected_one_of_last_two]
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }

    /// Commonwealth Electoral Act 1918, Section 273, 13(a)
    /// ```text
    /// (a) the candidate who stands lowest in the poll must be excluded;
    /// ```
    /// Commonwealth Electoral Act 1918, Section 273, 31(b)
    /// ```text
    /// if 2 or more continuing candidates have the same number of
    /// votes, those candidates shall stand in the poll in the order of
    /// the relative number of votes of each of those candidates at the
    /// last count at which each of them had a different number of
    /// votes, with the continuing candidate with the greater or
    /// greatest number of votes at that count standing higher in the
    /// poll and the continuing candidate with the fewer or fewest
    /// number of votes at that count standing lower in the poll, but
    /// if there has been no such count the Australian Electoral
    /// Officer for the State shall determine the order of standing of
    /// those candidates in the poll.
    /// ```
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }


    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    /// Commonwealth Electoral Act 1918, Section 273, (9)
    /// ```text
    /// Unless all the vacancies have been filled, the number (if any) of
    /// votes in excess of the quota (in this section referred to as surplus
    /// votes) of each elected candidate shall be transferred to the
    /// continuing candidates as follows: ...
    /// ```
    /// Similarly (10) and (14) have the crucial "Unless all the vacancies have been filled"
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }


    /// Commonwealth Electoral Act 1918, Section 273, (17)
    /// ```text
    /// In respect of the last vacancy for which two continuing candidates
    /// remain, the continuing candidate who has the larger number of
    /// votes shall be elected notwithstanding that that number is below
    /// the quota, and if those candidates have an equal number of votes
    /// the Australian Electoral Officer for the State shall have a casting
    /// vote but shall not otherwise vote at the election.
    /// ```
    /// See discussion above about when to do rule 18. This seems similar,
    /// except there is explicit mention of this subsection in subsection
    /// 15, which is otherwise similar to 13, which implies maybe this rule
    /// doesn't apply after first pref distributions or surplus distributions,
    /// but does after exclusions. But even that is tenuous, as (15) talks about
    /// reaching Quota rather than being elected.
    ///
    /// It seems to have been done differently in different years, so precedent is not much use.
    ///
    /// This is actually a very significant issue, as it can easily affect who
    /// is elected. For instance (has happened in the Federal election 2019,
    /// Queensland Senate), you could end up with a situation like the end of count 287.
    /// There are 3 candidates remaining, and 2 vacant seats. One candidate (G. Rennick) has got a
    /// quota, and so gets elected. There are now two remaining candidates and one seat,
    /// the conditions for this ending. Does this get applied immediately, in which
    /// case whichever of the remaining two candidates with the higher tally before
    /// G. Rennick's excess is distributed gets elected, or does it get applied after some or
    /// all of G. Rennick's excess is distributed, in which case a different outcome is
    /// possible in principle? The different outcome did not occur in that specific case,
    /// but could easily do so in a similar situation since preference flows tend to be highly non-random.
    ///
    /// I am assigning it a generous frequency, although I can see arguments
    /// for other interpretations.
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuota }

    /// Commonwealth Electoral Act 1918, Section 273, (18)
    /// ```text
    /// Notwithstanding any other provision of this section, where the
    /// number of continuing candidates is equal to the number of
    /// remaining unfilled vacancies, those candidates shall be elected.
    /// ```
    /// This is not very helpful about WHEN this is checked. Other sections
    /// may or may not be relevant given the `Notwithstanding any other provision of this section`
    ///
    /// (13) is more explicit:
    ///
    /// Commonwealth Electoral Act 1918, Section 273, (13)
    /// ```text
    /// Where, after the counting of first preference votes or the transfer of
    /// surplus votes (if any) of elected candidates, no candidate has, or
    /// fewer than the number of candidates required to be elected have,
    /// received a number of votes equal to the quota:
    /// (a) the candidate who stands lowest in the poll must be excluded;
    /// or
    /// (b) if a bulk exclusion of candidates may be effected under
    /// subsection (13A), those candidates must be excluded;
    /// and the ballot papers of the excluded candidate or candidates must
    /// be distributed in accordance with subsection (13AA).
    /// ```
    /// This sounds as if the ballot papers must be distributed. Possible for all transfer values.
    ///
    /// Commonwealth Electoral Act 1918, Section 273, (13)
    /// ```text
    /// continuing candidate means a candidate not already elected or
    /// excluded from the count.
    /// ```
    /// This possibly implies that the ballot paper exclusion must be
    /// totally finished before section 17 or 18 applies.
    ///
    /// It seems to have been done differently in different years, so precedent is not much use.
    ///
    /// This is a matter of moderate importance, as it doesn't affect who is elected, just the particular
    /// timing. It could however change the order of election, so it is not insignificant.
    ///
    /// I am assigning it to be done generously, which is debatable.
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuota }

    /// Commonwealth Electoral Act 1918, Section 273, subsection (13)(b)
    /// ```text
    /// (b) if a bulk exclusion of candidates may be effected under
    ///     subsection (13A), those candidates must be excluded;
    /// ```
    fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { true }

    fn name() -> String { "Federal".to_string() }
}

/// The actual rules used by the AEC in 2019, based on reverse engineering their published
/// distribution of preferences transcripts.
///
/// Note that this is not possible to specify perfectly as the AEC considers their source
/// code secret and have persecuted people who requested it under FOI. There are often
/// multiple interpretations compatible with the actual outcome. I have tried to guess
/// the most plausible rules used, as close as possible to my interpretation of the legislation.
pub struct FederalRulesUsed2019 { }

impl PreferenceDistributionRules for FederalRulesUsed2019 {
    type Tally = usize ;
    type SplitByNumber = DoNotSplitByCountNumber;

    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue {
        TransferValue::from_surplus(surplus,ballots)
    }

    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> (usize, LostToRounding) {
        transfer_value.mul_rounding_down(ballots)
    }


    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }


    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }


    /// In 2019 QLD, in count 287, a surplus distribution, G. Rennick gets elected for achieving a quota.
    /// This leaves 2 candidates, and 1 vacancy. The 2 standing rules is not applied until
    /// count 288 when G. Rennick's excess is distributed.
    ///
    /// In 2019 VIC, J Hallam is eliminated, starting on count 362, leaving 2 candidates and
    /// 1 vacancy. The rule is not applied until count 367 when  the elimination is finished.
    ///
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }

    /// In 2019 NSW, count 429, K. McCulloch is excluded. This leaves 2 candidates, 2 vacancies.
    /// The elimination is aborted and no ballots are transferred in this count.
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterDeterminingWhoToExcludeButBeforeTransferringAnyPapers }

    /// Not done in TAS count 5, SA count 6, QLD 5, NSW 6.
    fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { false }

    fn name() -> String { "AEC2019".to_string() }
}

/// the actual rules used by the AEC in 2016, based on reverse engineering their published
/// distribution of preferences transcripts.
///
/// Note that this is not possible to specify perfectly as the AEC considers their source
/// code secret and have persecuted people who requested it under FOI. There are often
/// multiple interpretations compatible with the actual outcome. I have tried to guess
/// the most plausible rules used, as close as possible to my interpretation of the legislation.
pub struct FederalRulesUsed2016 { }

impl PreferenceDistributionRules for FederalRulesUsed2016 {
    type Tally = usize ;
    type SplitByNumber = DoNotSplitByCountNumber;

    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue {
        TransferValue::from_surplus(surplus,ballots)
    }

    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> (usize, LostToRounding) {
        transfer_value.mul_rounding_down(ballots)
    }


    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }

    /// In 2016, WA (with Rod Cullerton excluded because of bankruptcy and larceny), on count 49, there was a 3 way tie for elimination.
    /// M. Hercock, S. Fargher and H HENG all had 66 votes.
    /// The latest turn that they all had different tallies was turn 4, with 65, 61 and 63 respectively.
    /// So MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent means that S. Fargher should have been eliminated.
    /// Actually M. Hercock was eliminated. This may be because on round 41, they had tallies 65, 66 and 66 respectively.
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }


    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }


    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }


    /// In Queensland 2016, count 830, candidate R. McGarvie was excluded, leaving 2 candidates and 2 seats.
    /// The exclusion was carried out in full (11 counts), and C Ketter was discovered to have a quota, leaving 1 candidate (M Roberts) and 1 vacancy.
    /// This candidate was not elected until count 841, when C Ketter's surplus was distributed.
    ///
    /// A very similar thing happened in Victoria 2016, count 814, P. Bain was excluded, leaving 2 candidates and 2 seats.
    /// The exclusion was carried out in full (11 counts), and J Rice was discovered to have a quota, leaving 1 candidate (J Hume) and 1 vacancy.
    /// This candidate was not elected until count 825, when J Rice's surplus was distributed.
    ///
    /// A similar but slightly more complex thing happened in NSW 2016, count 1054. N. Hall was excluded, leaving 3 remaining candidates and 3 vacancies.
    /// The exclusion was carried out in full (10 counts), and two candidates, J Williams and B Burston were elected on quota.
    /// Two more surplus distributions were carried out, and on the last, D Leyonhjelm was elected.
    ///
    /// A different thing happened in WA 2016 (with Rod Cullerton excluded), ot count 535, K. Muir was excluded, leaving 2 candidates and 2 seats.
    /// The first step of the exclusion was performed, at the end of which the remaining 2 candidates were both declared elected.
    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExists }

    /// several occasions.
    fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { false }

    fn name() -> String { "AEC2016".to_string() }
}

/// the actual rules used by the AEC in 2013, based on reverse engineering their published
/// distribution of preferences transcripts.
///
/// Note that this is not possible to specify perfectly as the AEC considers their source
/// code secret and have persecuted people who requested it under FOI. There are often
/// multiple interpretations compatible with the actual outcome. I have tried to guess
/// the most plausible rules used, as close as possible to my interpretation of the legislation.
pub struct FederalRulesUsed2013 { }

impl PreferenceDistributionRules for FederalRulesUsed2013 {
    type Tally = usize ;
    type SplitByNumber = DoNotSplitByCountNumber;

    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue {
        TransferValue::from_surplus(surplus,ballots)
    }

    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> (usize, LostToRounding) {
        transfer_value.mul_rounding_down(ballots)
    }


    fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }
    fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent }

    /// In 2013 NSW, count 25, T. Dean was eliminated in a 4 way tie for 17. All candidates had 17 since count 1 other than T. Dean who had 16.
    /// This may be coincidence - the EC could have then decided it with MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent , or it could have been an application of MethodOfTieResolution::AnyDifferenceIsADiscriminator
    fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { MethodOfTieResolution::AnyDifferenceIsADiscriminator }


    fn finish_all_counts_in_elimination_when_all_elected() -> bool { false }
    fn finish_all_surplus_distributions_when_all_elected() -> bool { false }


    /// In SA, count 228, B. Day is elected on quota, leaving 2 candidates 1 seat. S. Birmingham is not elected until the next count, 229.
    fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }


    fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExists }

    /// several occasions, e.g ACT.
    fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { true }

    fn name() -> String { "AEC2013".to_string() }
}



#[cfg(test)]
mod tests {
    use std::fs::File;
    use stv::election_data::ElectionData;
    use stv::compare_rules::CompareRules;
    use crate::{FederalRulesUsed2013, FederalRulesUsed2016, FederalRules, FederalRulesUsed2019};
    use stv::compare_transcripts::DifferenceBetweenTranscripts::{DifferentCandidatesElected, CandidatesOrderedDifferentWay,Same};
    use stv::compare_transcripts::DifferentCandidateLists;
    use stv::ballot_metadata::CandidateIndex;

    #[test]
    fn example() -> anyhow::Result<()>{
        let data : ElectionData = serde_json::from_reader(File::open("../examples/MultipleExclusionOrdering.stv")?)?;
        let comparer = CompareRules{ dir: "tests".to_string() };
        let (comparisons,comp) = comparer.compute_dataset::<usize,FederalRulesUsed2013,FederalRulesUsed2016,FederalRulesUsed2019,FederalRules>(&data)?;

        for i in 0..comparisons.len() {
            println!("{} : {}",comparisons[i],comp.results[i]);
        }
        let index = |n1:&str,n2:&str| comparisons.iter().position(|c|&c.rule1==n1 && &c.rule2==n2).unwrap();
        assert_eq!(comp.results[index("AEC2016","AEC2013")],DifferentCandidatesElected(DifferentCandidateLists{ list1: vec![CandidateIndex(0),CandidateIndex(2),CandidateIndex(3),CandidateIndex(4),CandidateIndex(5),CandidateIndex(6)], list2: vec![CandidateIndex(0),CandidateIndex(1),CandidateIndex(6),CandidateIndex(5),CandidateIndex(4),CandidateIndex(3)] }));
        assert_eq!(comp.results[index("AEC2019","AEC2013")],DifferentCandidatesElected(DifferentCandidateLists{ list1: vec![CandidateIndex(0),CandidateIndex(2),CandidateIndex(6),CandidateIndex(5),CandidateIndex(4),CandidateIndex(3)], list2: vec![CandidateIndex(0),CandidateIndex(1),CandidateIndex(6),CandidateIndex(5),CandidateIndex(4),CandidateIndex(3)] }));
        assert_eq!(comp.results[index("AEC2019","AEC2016")],CandidatesOrderedDifferentWay(DifferentCandidateLists{ list1: vec![CandidateIndex(0),CandidateIndex(2),CandidateIndex(6),CandidateIndex(5),CandidateIndex(4),CandidateIndex(3)], list2: vec![CandidateIndex(0),CandidateIndex(2),CandidateIndex(3),CandidateIndex(4),CandidateIndex(5),CandidateIndex(6)] }));
        assert_eq!(comp.results[index("Federal","AEC2013")],Same);
        Ok(())
    }
}
