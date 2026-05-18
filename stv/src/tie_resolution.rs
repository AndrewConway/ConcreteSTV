// Copyright 2021-2026 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use crate::ballot_metadata::{CandidateIndex, NumberOfCandidates};
use crate::distribution_of_preferences_transcript::{CountIndex, Transcript};
use std::collections::{HashSet, HashMap};
use std::hash::Hash;
use serde::{Serialize,Deserialize};
use anyhow::anyhow;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use crate::ballot_pile::BallotPaperCount;
use crate::compare_transcripts::DeltasInCandidateLists;
use crate::random_util::Randomness;

#[derive(Debug,Clone,Copy)]
pub enum MethodOfTieResolution {
    None,
    /// Require that at some prior point *all* the counts were different
    /// ```text
    /// Commonwealth Electoral Act 1918, Section 273, 20(b) extract
    /// if any 2 or more of
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
    RequireHistoricalCountsToBeAllDifferent,
    /// Another approach is that whenever X has a higher count than Y, Y is considered below X.
    /// That is, whenever there are at least 2 different values, all with the lower values go before all with the higher values.
    /// This is equivalent to always sorting by tally, and actually seems the most reasonable choice as far as I am concerned.
    /// Of course, that is not necessarily what is legislated.
    AnyDifferenceIsADiscriminatorGiveUpIfNotFullSolution,
    /// Like AnyDifferenceIsADiscriminatorGiveUpIfNotFullSolution, except instead of giving up if not a full solution,
    /// use the NZ PRNs. See clauses 41 to 48 of
    /// [Schedule 1A (New  Zealand method of counting single transferable votes), Local Electoral Regulations 2001, (SR 2001/145), Version as at 1 July 2025](https://www.legislation.govt.nz/secondary-legislation/pco-drafted/2001/145/en/latest/#DLM57125)
    ///
    /// Note that I am not confident of my implementation as I consider the legislation to be ambiguous in a few places:
    /// * Between clause 43 and 44 are there 4 or 5 values discarded?
    /// * How does the timing of clause 23 actually work?
    /// * It appears that in practice and extra copy of clause 13 is inserted immediately after clause 6, which upsets the timing of everthing thereafter, which matters because of the PRNs inverting each iteration.
    AnyDifferenceIsADiscriminatorUseNZPRNsIfNotFullSolution,
    /// Like AnyDifferenceIsADiscriminatorUseNZPRNsIfNotFullSolution but include a variety of changes to the
    /// algorithm signposted in https://github.com/Conservatory/openstv/blob/master/openstv/MethodPlugins/MeekNZSTV.py
    /// as probably being used in the official software (although I can't check without access or sufficient examples).
    /// * Line 77: clause 42 has the 1000 in the formula for z replaced by 10,000.
    /// * Line 90: an extra mod 10,000 is inserted into the rc computation in clause 43. Seems plausible otherwise why have the 10,000 in the inversion?
    /// * Line 92: an extra inversion is done. (this could be the 4 vs 5 debate ambiguity listed above)
    AnyDifferenceIsADiscriminatorUseApocryphalNZPRNsIfNotFullSolution,
    /// This is an even better version of AnyDifferenceIsADiscriminatorGiveUpIfNotFullSolution. The difference is that if AnyDifferenceIsADiscriminatorGiveUpIfNotFullSolution fails
    /// to solve everything, then a draw is done as if the countback was totally nullified. However,
    /// AnyDifferenceIsADiscriminator only does a draw amongst the candidates that AnyDifferenceIsADiscriminator could not distinguish.
    AnyDifferenceIsADiscriminator,
    /// Like RequireHistoricalCountsToBeAllDifferent, but ignore sub-transfers in the middle
    /// of a poly-transfer. E.g. in an exclusion where there are different transfer values
    /// transferred in different sub-counts, ignore all the subcounts other than the one where
    /// it is finished.
    RequireHistoricalCountsToBeAllDifferentOnlyConsideringCountsWhereAnActionIsFinished,
    /// Like AnyDifferenceIsADiscriminatorGiveUpIfNotFullSolution but only consider major counts like RequireHistoricalCountsToBeAllDifferentOnlyConsideringCountsWhereAnActionIsFinished
    AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinishedGiveUpIfNotFullSolution,
    /// Like AnyDifferenceIsADiscriminator but only consider major counts like RequireHistoricalCountsToBeAllDifferentOnlyConsideringCountsWhereAnActionIsFinished
    AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinished,
    /// In between AnyDifferenceIsADiscriminatorGiveUpIfNotFullSolution and RequireHistoricalCountsToBeAllDifferent,
    /// 
    /// As described in the Federal House of Representatives tie resolution, 
    /// Commonwealth Electoral Act 1918, Section 275, (9)
    /// ```text
    /// If, on any count other than the final count:
    /// (a) 2 or more candidates (lowest ranking candidates) have an
    ///     equal number of votes; and
    /// (b) one of them has to be excluded;
    /// the candidate to be excluded is the candidate with less votes than
    /// any of the other lowest ranking candidates at the last count at
    /// which one of those candidates had less votes than any of the others,
    /// but, if there has been no such count, the Divisional Returning
    /// Officer must decide by lot which of them is to be excluded
    /// ```
    /// 
    /// This is a slightly more general version of this - if there are 2 lowest needed, then it is required
    /// that the 2 lowest are both lower than the third lowest.
    RequireHistoricalUniqueLowest,
}

/// Sometimes you need tie resolution to distinguish all candidates (e.g. for order elected),
/// sometimes only to single out a particular subset (e.g. elimination of 1 lowest candidate).
/// This specifies how precise one needs to be.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum TieResolutionGranularityNeeded {
    /// Require a unique collection of all people
    Total,
    /// Require the lowest provided number to be separated from the remainder.
    LowestSeparated(usize)
}

impl MethodOfTieResolution {
    /// sort tied_candidates low to high based upon the given method of tie resolution.
    /// If the method does not resolve it, return a DecisionMadeByEC object.
    pub fn resolve<'a,Tally:Clone+Hash+Ord+Display+FromStr+Debug>(self,tied_candidates: &'a mut [CandidateIndex],transcript:  &Transcript<Tally>,granularity:TieResolutionGranularityNeeded,number_of_candidates: NumberOfCandidates,number_of_vacancies:NumberOfCandidates,valid_papers:BallotPaperCount) -> Vec<(&'a mut [CandidateIndex],TieResolutionGranularityNeeded)> {
        let resolved = match self {
            MethodOfTieResolution::None => false,
            MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent => resolve_ties_require_all_different(tied_candidates,transcript,false),
            MethodOfTieResolution::AnyDifferenceIsADiscriminatorGiveUpIfNotFullSolution => resolve_ties_any_different_give_up_if_cant_do_everything(tied_candidates, transcript, granularity, false),
            MethodOfTieResolution::AnyDifferenceIsADiscriminatorUseNZPRNsIfNotFullSolution => {
                let solved_by_aafd = resolve_ties_any_different_give_up_if_cant_do_everything(tied_candidates, transcript, granularity, false);
                if !solved_by_aafd { // attempt to solve using NZ PRN method
                    resolve_using_NZPRN(tied_candidates,transcript,number_of_candidates,number_of_vacancies,valid_papers,false);
                }
                true
            }
            MethodOfTieResolution::AnyDifferenceIsADiscriminatorUseApocryphalNZPRNsIfNotFullSolution => {
                let solved_by_aafd = resolve_ties_any_different_give_up_if_cant_do_everything(tied_candidates, transcript, granularity, false);
                if !solved_by_aafd { // attempt to solve using NZ PRN method
                    resolve_using_NZPRN(tied_candidates,transcript,number_of_candidates,number_of_vacancies,valid_papers,true);
                }
                true
            }
            MethodOfTieResolution::AnyDifferenceIsADiscriminator => return resolve_ties_any_different(tied_candidates, transcript, granularity, false),
            MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferentOnlyConsideringCountsWhereAnActionIsFinished => resolve_ties_require_all_different(tied_candidates,transcript,true),
            MethodOfTieResolution::AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinishedGiveUpIfNotFullSolution => resolve_ties_any_different_give_up_if_cant_do_everything(tied_candidates, transcript, granularity, true),
            MethodOfTieResolution::AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinished => return resolve_ties_any_different(tied_candidates, transcript, granularity, true),
            MethodOfTieResolution::RequireHistoricalUniqueLowest => resolve_ties_require_unique_minimum_granularity(tied_candidates, transcript, granularity, false),
        };
        if resolved { vec![] } else { vec![(tied_candidates,granularity)] }
    }
}

/// Resolve ties using the NZ PRNG, either the legislated or apocryphal versions. See 
/// MethodOfTieResolution::AnyDifferenceIsADiscriminatorUseApocryphalNZPRNsIfNotFullSolution for
/// what apocryphal means in this context.
/// 
/// Always succeeds. Unless there are too many candidates (10,001 is definitely too many for
/// apocryphal. Note that the NZ legislation will fail in this case as clause 46 would be impossible to fulfil. 
#[allow(non_snake_case)] // allow NZPRN acronym.
fn resolve_using_NZPRN<Tally: Clone + Hash + Ord + Display + FromStr + Debug>(tied_candidates: &mut [CandidateIndex], transcript:  &Transcript<Tally>, number_of_candidates: NumberOfCandidates, number_of_vacancies:NumberOfCandidates, valid_papers:BallotPaperCount, apocryphal:bool) {
    let mut prng = NZPRNG::new(number_of_candidates,number_of_vacancies,valid_papers,apocryphal);
    let prns = prng.get_all_prns(number_of_candidates,apocryphal); // gets a sufficient number of PRNs.
    tied_candidates.sort_by_key(|c|prns[c.0]);
    if transcript.counts.len()%2==1 { tied_candidates.reverse()} // reverse the order of PRNs each count.
}
/// In order to perfectly match the results of an Electoral Commission, it is necessary to have
/// the identical decisions made. These are handled by providing an explicit list.
///
/// This holds such information.
///
/// A tie between C1,C2 and C3 is broken by the first list of candidates provided that includes
/// all the candidates. The relative order in this list is the relative order of the candidates
/// in the new list. (low to high)
///
/// If nothing matches, then a candidate with a smaller index (earlier on the paper generally)
/// will be put before (generally a worse position) than a candidate with a smaller index.
/// This seems to be what many ECs do in practice.
///
/// Is it possible that the same set of candidates will need two different ties resolutions?
/// This seems unlikely since tie resolutions tend to result in at least one candidate being
/// elected or eliminated, at which point they are unlikely to be relevant. However, in Federal
/// rules, this situation is technically possible, since if multiple candidates get elected
/// in the same count (e.g. over quota), their order of election is covered by rule 20(b),
/// but their order of elimination is covered by (basically identical) rule 22. Both allow
/// the EC to make a decision, and it would be conceivable for them to be different decisions.
/// If an EC ever perversely decides to do this, I guess I will need to support it. But no need to
/// introduce added complexity until then
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct TieResolutionsMadeByEC {
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub tie_resolutions : Vec<TieResolutionAtom>
}

impl Default for TieResolutionsMadeByEC {
    fn default() -> Self { TieResolutionsMadeByEC{tie_resolutions:vec![]}}
}

#[derive(Serialize,Deserialize,Debug,Clone,Eq,PartialEq)]
#[serde(untagged)]
#[serde(from = "TieResolutionAtomWithBackwardsCompatibility")]
pub enum TieResolutionAtom {
    /// Old style, list candidates in order of increasing favour. Useful for 3 way ties on order of election, should that ever happen. Should possibly deprecate.
    IncreasingFavour(Vec<CandidateIndex>),
    /// New preferred style.
    ExplicitDecision(TieResolutionExplicitDecisionInCount)
}

impl From<TieResolutionAtomWithBackwardsCompatibility> for TieResolutionAtom {
    fn from(value: TieResolutionAtomWithBackwardsCompatibility) -> Self {
        match value {
            TieResolutionAtomWithBackwardsCompatibility::IncreasingFavour(decision) => TieResolutionAtom::IncreasingFavour(decision),
            TieResolutionAtomWithBackwardsCompatibility::ExplicitDecision(decision) => TieResolutionAtom::ExplicitDecision(decision),
            TieResolutionAtomWithBackwardsCompatibility::OldExplicitDecision(decision) =>
                TieResolutionAtom::ExplicitDecision(TieResolutionExplicitDecisionInCount{ decision: TieResolutionExplicitDecision { increasing_favour: vec![decision.disfavoured,decision.favoured], usage: None }, came_up_in: None }),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
/// This structure is solely used to allow backwards compatibility for reading files using ObsoleteTieResolutionExplicitDecisionInCount.
/// It is not perfect backwards compatibility - came_up_in is not carried on, but that is actually good as for all versions in which it was output, it was ignored by ConcreteSTV.
enum TieResolutionAtomWithBackwardsCompatibility {
    /// Very old style, list candidates in order of increasing favour. Useful for 3 way ties on order of election, should that ever happen.
    IncreasingFavour(Vec<CandidateIndex>),
    /// New preferred style.
    ExplicitDecision(TieResolutionExplicitDecisionInCount),
    // Old version of TieResolutionExplicitDecisionInCount, kept for backwards compatibility with .stv files.
    OldExplicitDecision(ObsoleteTieResolutionExplicitDecisionInCount),
}

/// Kept solely for backwards compatibility. Deprecated. Never created any more.
#[derive(Deserialize)]
struct ObsoleteTieResolutionExplicitDecisionInCount {
    /// the candidate(s) that got the better result from the EC's decision. Order is not meaningful.
    favoured : Vec<CandidateIndex>,
    /// the candidate(s) that got the worse result from the EC's decision. Order is not meaningful.
    disfavoured : Vec<CandidateIndex>,
    /// if this came up in an official election, list the round it came up in.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    #[allow(dead_code)]
    came_up_in : Option<String>,
}


#[derive(Serialize,Deserialize,Debug,Clone,Eq,PartialEq)]
pub struct TieResolutionExplicitDecisionInCount {
    #[serde(flatten)]
    pub decision : TieResolutionExplicitDecision,
    /// if this came up in an official election, list the round it came up in.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub came_up_in : Option<CountIndex>,
}

#[derive(Serialize,Deserialize,Debug,Clone,Eq,PartialEq)]
pub struct TieResolutionExplicitDecision {
    /// More general alternative to disfavoured and favoured.
    /// increasing_favour[0] are the candidates least favoured by the EC (got the worst result). Order withing this sub array doesn't matter.
    /// increasing_favour[1] are the candidates more favoured by the EC.
    /// increasing_favour.last are the candidates most favoured by the EC (got the best result).
    pub increasing_favour: Vec<Vec<CandidateIndex>>,
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub usage : Option<TieResolutionUsage>,
}

/// Where a tie resolution was performed.
#[derive(Serialize,Deserialize,Debug,Clone,Copy,Eq,PartialEq)]
pub enum TieResolutionUsage {
    Exclusion,
    OrderElected,
    ShortcutWinner,
    OrderSurplusDistributed, // usually OrderElected, unless this is present earlier in the list.
    RoundingUp, // For NSW stochastic
}

impl FromStr for TieResolutionUsage {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Exclusion" => Ok(TieResolutionUsage::Exclusion),
            "OrderElected" => Ok(TieResolutionUsage::OrderElected),
            "ShortcutWinner" => Ok(TieResolutionUsage::ShortcutWinner),
            "OrderSurplusDistributed" => Ok(TieResolutionUsage::OrderSurplusDistributed),
            "RoundingUp" => Ok(TieResolutionUsage::RoundingUp),
            _ => Err("no such tie resolution usage"),
        }
    }
}
impl TieResolutionExplicitDecision {
    /// make a decision from the common case of two lists of candidates, one favoured over the other.
    pub fn two_lists(disfavoured:Vec<CandidateIndex>,favoured:Vec<CandidateIndex>) -> Self {
        TieResolutionExplicitDecision {
            increasing_favour: vec![disfavoured,favoured],
            usage: None,
        }
    }
    /// make a decision given a final ordering of candidates and a given granularity and usage.
    pub fn from_resolution(resolved_order:&[CandidateIndex],granularity:TieResolutionGranularityNeeded,usage:TieResolutionUsage) -> Self {
        match granularity {
            TieResolutionGranularityNeeded::Total => {
                TieResolutionExplicitDecision {
                    increasing_favour: resolved_order.iter().map(|c|vec![*c]).collect(),
                    usage: Some(usage),
                }
            }
            TieResolutionGranularityNeeded::LowestSeparated(disfavoured) => {
                TieResolutionExplicitDecision{
                    increasing_favour: vec![
                        resolved_order[..disfavoured].to_vec(),resolved_order[disfavoured..].to_vec()
                    ],
                    usage: Some(usage),
                }
            }
        }
    }
    /// The AEC seemed to some years resolve all decisions by reverse donkey vote.
    /// That is candidate i is favoured over j iff i>j
    /// See if this is one.
    pub fn is_reverse_donkey_vote(&self) -> bool {
        let mut highest_seen : Option<usize> = None;
        for candidates in &self.increasing_favour {
            if let Some(should_be_lower) =  highest_seen.take() {
                if let Some(lowest) = candidates.iter().map(|c|c.0).min() {
                    if should_be_lower>lowest { return false; }
                }
            }
            if let Some(highest) = candidates.iter().map(|c|c.0).max() {
                highest_seen=Some(highest)
            }
        }
        true
    }

    /// If the decision can be represented as a set of favoured and disfavoured candidates, extract them.
    /// Returns (disfavoured,favoured)
    fn extract_disfavoured_and_favoured(&self) -> Option<(&[CandidateIndex],&[CandidateIndex])> {
        if self.increasing_favour.len()==2 { Some((&self.increasing_favour[0], &self.increasing_favour[1]))}
        else { None }
    }

    /// See if a different decision here could explain different people being excluded.
    /// the different result is summarized in excluded_deltas where list1 is the desired candidate(s) to exclude, and list2 contains the candidate(s) excluded by this decision
    /// If such a different decision exists, return it.
    pub fn could_a_different_decision_have_caused_different_candidates_to_be_excluded(&self,excluded_deltas : &DeltasInCandidateLists) -> Option<TieResolutionExplicitDecision> {
        match self.usage {
            None | Some(TieResolutionUsage::Exclusion) => {
                if let Some((disfavoured,favoured)) = self.extract_disfavoured_and_favoured() {
                    // check that my decision favoured everyone kept just in mine and disfavoured everyone excluded just in mine
                    if excluded_deltas.list1only.iter().all(|candidate_excluded_only_in_official|favoured.contains(candidate_excluded_only_in_official))
                        &&  excluded_deltas.list2only.iter().all(|candidate_excluded_only_in_my|disfavoured.contains(candidate_excluded_only_in_my)) { // well, that would explain it.
                        let favoured = favoured.iter().filter(|&w|!excluded_deltas.list1only.contains(w)).chain(excluded_deltas.list2only.iter()).cloned().collect::<Vec<_>>();
                        let disfavoured = disfavoured.iter().filter(|&w|!excluded_deltas.list2only.contains(w)).chain(excluded_deltas.list1only.iter()).cloned().collect::<Vec<_>>();
                        Some(TieResolutionExplicitDecision { increasing_favour: vec![disfavoured, favoured], usage: self.usage })
                    } else { None }
                } else { None }
            },
            _ => None,
        }
    }

    /// Get the total number of candidates mentioned.
    pub fn num_candidates_mentioned(&self) -> usize {
        self.increasing_favour.iter().map(|v|v.len()).sum()
    }
    pub fn mentions_exactly_these_candidates(&self,candidates:&[CandidateIndex]) -> bool {
        candidates.len()==self.num_candidates_mentioned() && candidates.iter().all(|c|self.increasing_favour.iter().any(|v|v.contains(c)))
    }
}

impl Display for TieResolutionExplicitDecision {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut had_something = false;
        write!(f,"Chose ")?;
        for candidates in &self.increasing_favour {
            if had_something { write!(f," < ")? }
            else { had_something = true; }
            write!(f,"{:?}",candidates)?;
        }
        Ok(())
    }
}



impl TieResolutionsMadeByEC {
    /// Simple constructor that checks to see that a candidate is not repeated which would cause later bugs and would be ambiguous in any case.
    pub fn new(tie_resolutions : Vec<Vec<CandidateIndex>>) -> anyhow::Result<Self> {
        for decision in &tie_resolutions {
            let mut ordered = decision.clone();
            ordered.sort_by_key(|c|c.0);
            ordered.dedup();
            if ordered.len()!=decision.len() {
                return Err(anyhow!("Tie resolutions {} contain at least one repeated candidate",decision.iter().map(|c|c.to_string()).collect::<Vec<_>>().join(",")));
            }
        }
        let tie_resolutions = tie_resolutions.into_iter().map(|v|TieResolutionAtom::IncreasingFavour(v)).collect();
        Ok(TieResolutionsMadeByEC{tie_resolutions})
    }
    /// Sort tied_candidates appropriately (low to high), and then return a description of what was done.
    /// If all else fails, use randomness.
    pub fn resolve(&self, tied_candidates: &mut [CandidateIndex], granularity: TieResolutionGranularityNeeded,usage:TieResolutionUsage,current_count:CountIndex,randomness:&mut Randomness) -> TieResolutionExplicitDecision {
        self.resolve_work(tied_candidates,granularity,usage,current_count,randomness);
        TieResolutionExplicitDecision::from_resolution(tied_candidates,granularity,usage)
    }
    fn resolve_work(&self, tied_candidates: &mut [CandidateIndex], granularity: TieResolutionGranularityNeeded,usage:TieResolutionUsage,current_count:CountIndex,randomness:&mut Randomness)  {
        // println!("Trying to resolve {:?}. There are {} tie resolutions given.",tied_candidates,self.tie_resolutions.len());
        for atom in &self.tie_resolutions {
            match atom {
                TieResolutionAtom::IncreasingFavour(decision) => {
                    let deemed_order : Vec<CandidateIndex> = decision.iter().filter(|&c|tied_candidates.contains(c)).cloned().collect();
                    if deemed_order.len()==tied_candidates.len() {
                        tied_candidates.copy_from_slice(&deemed_order);
                        return;
                    }
                    if granularity==TieResolutionGranularityNeeded::LowestSeparated(1) && decision.len()==2 && deemed_order.len()==2 {
                        // This is sufficient. One will be excluded and this should not re-arise.
                        // This is a bit of a hack introduced before TieResolutionExplicitDecision which how handles this case more elegantly and expressively.
                        let last = decision[0]; // this is least favoured candidate, so should go at the start of the list, which is in ascending order.
                        let order_with_last_first = [last].into_iter().chain(tied_candidates.iter().cloned().filter(|&c|c!=last)).collect::<Vec<_>>();
                        tied_candidates.copy_from_slice(&order_with_last_first);
                        return;
                    }
                }
                TieResolutionAtom::ExplicitDecision(TieResolutionExplicitDecisionInCount{decision, came_up_in, }) => {
                    let appropriate_usage = match decision.usage {
                        None => true,
                        Some(TieResolutionUsage::Exclusion) => usage==TieResolutionUsage::Exclusion,
                        Some(TieResolutionUsage::OrderElected) => usage==TieResolutionUsage::OrderElected || usage==TieResolutionUsage::OrderSurplusDistributed,
                        Some(TieResolutionUsage::ShortcutWinner) => usage==TieResolutionUsage::ShortcutWinner,
                        Some(TieResolutionUsage::OrderSurplusDistributed) => usage==TieResolutionUsage::OrderSurplusDistributed,
                        Some(TieResolutionUsage::RoundingUp) => usage==TieResolutionUsage::RoundingUp,
                    };
                    let appropriate_time = match came_up_in {
                        None => true,
                        Some(s) => *s==current_count,
                    };
                    let appropriate_division = match granularity {
                        TieResolutionGranularityNeeded::Total => decision.increasing_favour.iter().all(|v|v.len()==1),
                        TieResolutionGranularityNeeded::LowestSeparated(num_low) => decision.increasing_favour.len()==2 && decision.increasing_favour[0].len()==num_low
                    };
                    // println!("Found {:?} appropriate usage : {appropriate_usage} appropriate time : {appropriate_time} appropriate division : {appropriate_division}",decision);
                    if appropriate_usage && appropriate_time && appropriate_division && decision.mentions_exactly_these_candidates(tied_candidates) { // this decision is perfect for this particular case.
                        // load tied_candidates from flattened decision.increasing_favour.
                        // println!("Found ideal decision.");
                        let mut upto = 0;
                        for v in &decision.increasing_favour {
                            tied_candidates[upto..upto+v.len()].copy_from_slice(v);
                            upto+=v.len();
                        }
                        assert_eq!(upto,tied_candidates.len());
                        return;
                    }
                }
            }
        }
        // If all else fails, we need to do a draw.
        randomness.resolve(tied_candidates);
    }
}


/// Sort candidates low to high based on some prior period when they each had a different tally.
/// Return true iff ties are resolved.
fn resolve_ties_require_all_different<Tally:Clone+Eq+Hash+Ord+Display+FromStr+Debug>(tied_candidates: &mut [CandidateIndex],transcript:  &Transcript<Tally>,just_consider_major_counts:bool) -> bool {
    for count in transcript.counts.iter().rev() {
        if count.reason_completed || !just_consider_major_counts {
            let mut observed = HashSet::new();
            for candidate in tied_candidates.iter() {
                observed.insert(count.status.tallies.candidate[candidate.0].clone());
            }
            if observed.len()==tied_candidates.len() { // All different!
                tied_candidates.sort_by_key(|candidate|count.status.tallies.candidate[candidate.0].clone());
                return true;
            }
        }
    }
    false
}

/// Sort candidates low to high based on a countback where any difference is used as much as possible.
/// Return remaining need for resolution, if any.
fn resolve_ties_any_different<'a,Tally:Clone+Eq+Hash+Ord+Display+FromStr+Debug>(tied_candidates: &'a mut [CandidateIndex],transcript:  &Transcript<Tally>,granularity:TieResolutionGranularityNeeded,just_consider_major_counts:bool) -> Vec<(&'a mut [CandidateIndex],TieResolutionGranularityNeeded)> {
    //println!("Resolve ties any different between {}",tied_candidates.iter().map(|c|c.to_string()).collect::<Vec<_>>().join(","));
    let mut res = vec![];
    for count in transcript.counts.iter().rev() {if count.reason_completed || !just_consider_major_counts {
        let mut observed : HashMap<Tally,Vec<CandidateIndex>> = HashMap::new();
        for candidate in tied_candidates.iter() {
            observed.entry(count.status.tallies.candidate[candidate.0].clone()).or_insert_with(||vec![]).push(*candidate);
        }
        if observed.len()>1 { // at least 1 different.
            //println!("Broken into {} groups",observed.len());
            let mut tallies : Vec<Tally> = observed.keys().cloned().collect();
            tallies.sort();
            let mut upto : usize = 0;
            let mut remaining_tied_candidates = tied_candidates;
            for tally in tallies {
                let who = observed.get(&tally).unwrap();
                let (candidates_with_this_tally,candidates_with_higher_tally) = remaining_tied_candidates.split_at_mut(who.len());
                remaining_tied_candidates = candidates_with_higher_tally;
                candidates_with_this_tally.copy_from_slice(who);
                if who.len()>1 {
                    match granularity {
                        TieResolutionGranularityNeeded::Total => {res.extend(resolve_ties_any_different(candidates_with_this_tally,transcript,granularity,just_consider_major_counts)) }  // could optimize to start at count currently up to.
                        TieResolutionGranularityNeeded::LowestSeparated(loc) if loc>upto && loc<upto+who.len() => {res.extend(resolve_ties_any_different(candidates_with_this_tally,transcript,TieResolutionGranularityNeeded::LowestSeparated(loc-upto),just_consider_major_counts))}
                        TieResolutionGranularityNeeded::LowestSeparated(_) => {} // granularity means we don't care.
                    }
                }
                upto+=who.len();
            }
            //println!("Solution is : {}",tied_candidates.iter().map(|c|c.to_string()).collect::<Vec<_>>().join(","));
            return res;
        }
    }}
    vec![(tied_candidates,granularity)]
}


/// Sort candidates low to high based on a countback where any difference is used as much as possible.
/// Return true iff ties are resolved to the required granularity.
/// Like resolve_ties_any_different_work but give up if there are any problems.
fn resolve_ties_any_different_give_up_if_cant_do_everything<Tally:Clone+Eq+Hash+Ord+Display+FromStr+Debug>(tied_candidates: &mut [CandidateIndex],transcript:  &Transcript<Tally>,granularity:TieResolutionGranularityNeeded,just_consider_major_counts:bool) -> bool {
    resolve_ties_any_different(tied_candidates,transcript,granularity,just_consider_major_counts).is_empty()
}

/// Sort candidates low to high based on a countback where one finds the most recent 
/// count where the required granularity are all strictly lower than the next highest. (if any)
/// 
/// Return true iff ties are resolved to the required granularity.
fn resolve_ties_require_unique_minimum_granularity<Tally:Clone+Eq+Hash+Ord+Display+FromStr+Debug>(tied_candidates: &mut [CandidateIndex],transcript:  &Transcript<Tally>,granularity:TieResolutionGranularityNeeded,just_consider_major_counts:bool) -> bool {
    let unique_min = match granularity {
        TieResolutionGranularityNeeded::LowestSeparated(n) if n < tied_candidates.len() => n, // most common case
        TieResolutionGranularityNeeded::LowestSeparated(n) if n == tied_candidates.len() => return true, // umm, why did we need tie resolution? 
        TieResolutionGranularityNeeded::Total if tied_candidates.len() ==2 => 1, // can only produce complete granularity if there are exactly 2 to select
        _ => return false, 
    };
    assert!(unique_min>0);
    assert!(unique_min<tied_candidates.len());
    for count in transcript.counts.iter().rev() {if count.reason_completed || !just_consider_major_counts {
        let mut tallies : Vec<Tally> = tied_candidates.iter().map(|candidate|count.status.tallies.candidate[candidate.0].clone()).collect();
        tallies.sort_unstable();
        if tallies[unique_min-1]<tallies[unique_min] { // we have unique losers!
            let mut found_losers = 0;
            for i in 0..tied_candidates.len() {
                if count.status.tallies.candidate[tied_candidates[i].0].clone()<tallies[unique_min] { // this candidate is a loser. Move them to spot found_losers.
                    tied_candidates.swap(i,found_losers);
                    found_losers+=1;
                }
            }
            assert_eq!(found_losers, unique_min);
            return true
        }
    }}
    false
}

/// New Zealand PRNG method.
/// This structure is the generator for rcs for the following legislation:
///
/// ### PRNG method
/// 41. Allocate a unique pseudo-random whole number (a PRN number) for each candidate at each stage of the counting.
/// 42. To generate PRNs, calculate x, y, and z using the following formulae:
///
///   x = c+5
///   y = n
///   z = (v + 1 000 (v rem 10)) rem 30 323
///
///   where—
///     - **c** is the number of candidates
///     - **n** is the number of vacancies
///     - **v** is the total number of valid voting documents
///     - **rem** is the remainder operator such that a rem b gives the remainder of dividing whole number a by whole number b.
///
/// 43. Generate a random whole number rc using the following formulae:
///
///    x = (171x) rem 30 269
///    y = (172y) rem 30 307
///    z = (170z) rem 30 323
///    rc = (10 000x) div 30 269 + (10 000y) div 30 307 + (10 000z) div 30 323
///
/// where—
///     - **rc** is a pseudo-random number
///     - **div** is the integer division operator such that a div b gives the whole number quotient of dividing whole number a by whole number b.
///
/// 44. Repeat the step in clause 43 four times, discarding the first 4 values of rc.
/// 45. Assign the current value of rc to the first candidate.
/// 46. Repeat the step in clause 43 until a pseudo-random number r results that is distinct from all previous pseudo random numbers assigned to candidates. Assign rc to the next candidate.
/// 47. Repeat the step in clause 43 until all candidates have been assigned a pseudo-random number.
/// 48. For the second and subsequent steps, replace the pseudo-random number for each candidate with the candidate’s PRN at the previous step subtracted from 10 000.
struct NZPRNG {
    x : u32,
    y : u32,
    z : u32,
}

impl NZPRNG {
    /// initialize a new PRNG using paragraph 42.
    /// - **number_of_candidates** is the number of candidates
    /// - **number_of_vacancies** is the number of vacancies
    /// - **valid** is the total number of valid voting documents
    fn new(number_of_candidates: NumberOfCandidates,number_of_vacancies:NumberOfCandidates,valid:BallotPaperCount,apocryphal:bool) -> Self {
        let x = (number_of_candidates.0+5) as u32;
        let y = number_of_vacancies.0 as u32;
        let z = (valid.0+(if apocryphal {10000} else {1000})*(valid.0%10)) as u32;
        Self { x, y, z }
    }

    /// Get a new rc using rule 43
    fn get_next_rc(&mut self,apocryphal:bool) -> u32 {
        self.x = (self.x*171) % 30269;
        self.y = (self.y*172) % 30307;
        self.z = (self.z*170) % 30323;
        let rc = ((10000*self.x)%30269)+((10000*self.y)%30307)+((10000*self.z)%30323);
        if apocryphal {10000-(rc%10000)} else {rc}
    }

    /// Get PRNs for all candidates using rules 44 to 47.
    fn get_all_prns(&mut self,number_of_candidates: NumberOfCandidates,apocryphal:bool) -> Vec<u32> {
        // Note that it is not entirely clear whether rule 43 should be applied after rule 42 and before rule 43, or if 43 is just definitional for rules 44 to 46. I interpret it as the latter but this is just my guess.
        for _ in 0..4 { self.get_next_rc(apocryphal); }  // 44. Repeat the step in clause 43 four times, discarding the first 4 values of rc.
        let mut res = vec![];
        let mut set = HashSet::new();
        for _ in 0..number_of_candidates.0 {
            let mut rc = self.get_next_rc(apocryphal);
            while set.contains(&rc) { rc=self.get_next_rc(apocryphal); }
            res.push(rc);
            set.insert(rc);
        }
        res
    }
}