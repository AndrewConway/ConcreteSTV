// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use crate::ballot_metadata::CandidateIndex;
use crate::distribution_of_preferences_transcript::{CountIndex, Transcript};
use std::collections::{HashSet, HashMap};
use std::hash::Hash;
use serde::{Serialize,Deserialize};
use anyhow::anyhow;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use crate::compare_transcripts::DeltasInCandidateLists;

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
    AnyDifferenceIsADiscriminator,
    /// Like RequireHistoricalCountsToBeAllDifferent, but ignore sub-transfers in the middle
    /// of a poly-transfer. E.g. in an exclusion where there are different transfer values
    /// transferred in different sub-counts, ignore all the subcounts other than the one where
    /// it is finished.
    RequireHistoricalCountsToBeAllDifferentOnlyConsideringCountsWhereAnActionIsFinished,
    /// Like AnyDifferenceIsADiscriminator but only consider major counts like RequireHistoricalCountsToBeAllDifferentOnlyConsideringCountsWhereAnActionIsFinished
    AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinished,
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
    pub fn resolve<'a,Tally:Clone+Hash+Ord+Display+FromStr+Debug>(self,tied_candidates: &'a mut [CandidateIndex],transcript:  &Transcript<Tally>,granularity:TieResolutionGranularityNeeded) -> Option<(&'a mut [CandidateIndex],TieResolutionGranularityNeeded)> {
        let resolved = match self {
            MethodOfTieResolution::None => false,
            MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent => resolve_ties_require_all_different(tied_candidates,transcript,false),
            MethodOfTieResolution::AnyDifferenceIsADiscriminator => resolve_ties_any_different(tied_candidates,transcript,granularity,false),
            MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferentOnlyConsideringCountsWhereAnActionIsFinished => resolve_ties_require_all_different(tied_candidates,transcript,true),
            MethodOfTieResolution::AnyDifferenceIsADiscriminatorOnlyConsideringCountsWhereAnActionIsFinished => resolve_ties_any_different(tied_candidates,transcript,granularity,true),
        };
        // TODO deal with tie resolution that partially solves the problem.
        if resolved { None } else { Some((tied_candidates,granularity)) }
    }
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

#[derive(Serialize,Deserialize,Debug,Clone,Copy,Eq,PartialEq)]
pub enum TieResolutionUsage {
    Exclusion,
    OrderElected,
    ShortcutWinner,
    OrderSurplusDistributed, // usually OrderElected, unless this is present.
    RoundingUp, // For NSW stochastic
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
    pub fn resolve(&self, tied_candidates: &mut [CandidateIndex], granularity: TieResolutionGranularityNeeded,usage:TieResolutionUsage,current_count:CountIndex) -> TieResolutionExplicitDecision {
        self.resolve_work(tied_candidates,granularity,usage,current_count);
        TieResolutionExplicitDecision::from_resolution(tied_candidates,granularity,usage)
    }
    fn resolve_work(&self, tied_candidates: &mut [CandidateIndex], granularity: TieResolutionGranularityNeeded,usage:TieResolutionUsage,current_count:CountIndex)  {
        // println!("Trying to resolve {:?}",tied_candidates);
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
                        Some(TieResolutionUsage::OrderElected) => usage==TieResolutionUsage::Exclusion || usage==TieResolutionUsage::OrderSurplusDistributed,
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
                    if appropriate_usage && appropriate_time && appropriate_division && decision.mentions_exactly_these_candidates(tied_candidates) { // this decision is perfect for this particular case.
                        // load tied_candidates from flattened decision.increasing_favour.
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
        // If all else fails, we need to do a draw. For repeatability, do reverse donkey vote ATM. TODO make an option allowing randomness.
        tied_candidates.sort_by_key(|c|c.0);
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


/// Sort candidates low to high based on the tie resolution rules.
/// Return true iff ties are resolved to the required granularity.
fn resolve_ties_any_different<Tally:Clone+Eq+Hash+Ord+Display+FromStr+Debug>(tied_candidates: &mut [CandidateIndex],transcript:  &Transcript<Tally>,granularity:TieResolutionGranularityNeeded,just_consider_major_counts:bool) -> bool {
    //println!("Resolve ties any different between {}",tied_candidates.iter().map(|c|c.to_string()).collect::<Vec<_>>().join(","));
    for count in transcript.counts.iter().rev() {if count.reason_completed || !just_consider_major_counts {
        let mut observed : HashMap<Tally,Vec<CandidateIndex>> = HashMap::new();
        for candidate in tied_candidates.iter() {
            observed.entry(count.status.tallies.candidate[candidate.0].clone()).or_insert_with(||vec![]).push(*candidate);
        }
        if observed.len()>1 { // at least 1 different.
            //println!("Broken into {} groups",observed.len());
            let mut tallies : Vec<Tally> = observed.keys().cloned().collect();
            tallies.sort();
            let mut ok = true;
            let mut upto : usize = 0;
            for tally in tallies {
                let who = observed.get_mut(&tally).unwrap();
                if who.len()>1 {
                    match granularity {
                        TieResolutionGranularityNeeded::Total => {ok=ok&&resolve_ties_any_different(who,transcript,granularity,just_consider_major_counts)}  // could optimize to start at count currently up to.
                        TieResolutionGranularityNeeded::LowestSeparated(loc) if loc>upto && loc<upto+who.len() => {ok=ok&&resolve_ties_any_different(who,transcript,TieResolutionGranularityNeeded::LowestSeparated(loc-upto),just_consider_major_counts)}
                        TieResolutionGranularityNeeded::LowestSeparated(_) => {} // granularity means we don't care.
                    }
                }
                tied_candidates[upto..upto+who.len()].copy_from_slice(who);
                upto+=who.len();
            }
            //println!("Solution is : {}",tied_candidates.iter().map(|c|c.to_string()).collect::<Vec<_>>().join(","));
            return ok;
        }
    }}
    false
}