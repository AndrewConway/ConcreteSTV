use crate::ballot_metadata::CandidateIndex;
use crate::distribution_of_preferences_transcript::{Transcript, DecisionMadeByEC};
use std::collections::{HashSet, HashMap};
use std::hash::Hash;
use anyhow::anyhow;

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
}

/// Sometimes you need tie resolution to distinguish all candidates (e.g. for order elected),
/// sometimes only to single out a particular subset (e.g. elimination of 1 lowest candidate).
/// This specifies how precise one needs to be.
#[derive(Debug,Clone,Copy)]
pub enum TieResolutionGranularityNeeded {
    /// Require a unique collection of all people
    Total,
    /// Require the lowest provided number to be separated from the remainder.
    LowestSeparated(usize)
}

impl MethodOfTieResolution {
    /// sort tied_candidates low to high based upon the given method of tie resolution.
    /// If the method does not resolve it, return a DecisionMadeByEC object.
    pub fn resolve<Tally:Clone+Hash+Ord>(self,tied_candidates: &mut [CandidateIndex],transcript:  &Transcript<Tally>,granularity:TieResolutionGranularityNeeded) -> Option<DecisionMadeByEC> {
        let resolved = match self {
            MethodOfTieResolution::None => false,
            MethodOfTieResolution::RequireHistoricalCountsToBeAllDifferent => resolve_ties_require_all_different(tied_candidates,transcript),
            MethodOfTieResolution::AnyDifferenceIsADiscriminator => resolve_ties_any_different(tied_candidates,transcript,granularity),
        };
        if resolved { None } else { Some(DecisionMadeByEC{ affected: tied_candidates.to_vec() }) }
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
/// introduce added complexity until then.
pub struct TieResolutionsMadeByEC {
    pub resolutions : Vec<Vec<CandidateIndex>>
}

impl Default for TieResolutionsMadeByEC {
    fn default() -> Self { TieResolutionsMadeByEC{resolutions:vec![]}}
}

impl TieResolutionsMadeByEC {
    /// Simple constructor that checks to see that a candidate is not repeated which would cause later bugs and would be ambiguous in any case.
    pub fn new(resolutions : Vec<Vec<CandidateIndex>>) -> anyhow::Result<Self> {
        for decision in &resolutions {
            let mut ordered = decision.clone();
            ordered.sort_by_key(|c|c.0);
            ordered.dedup();
            if ordered.len()!=decision.len() {
                return Err(anyhow!("Tie resolutions {} contain at least one repeated candidate",decision.iter().map(|c|c.to_string()).collect::<Vec<_>>().join(",")));
            }
        }
        Ok(TieResolutionsMadeByEC{resolutions})
    }
    /// Sort tied_candidates appropriately (low to high)
    pub fn resolve(&self,tied_candidates: &mut [CandidateIndex]) {
        for decision in &self.resolutions {
            let deemed_order : Vec<CandidateIndex> = decision.iter().filter(|&c|tied_candidates.contains(c)).cloned().collect();
            if deemed_order.len()==tied_candidates.len() {
                tied_candidates.copy_from_slice(&deemed_order);
                return;
            }
        }
        tied_candidates.sort_by_key(|c|c.0);
    }
}


/// Sort candidates low to high based on some prior period when they each had a different tally.
/// Return true iff ties are resolved.
fn resolve_ties_require_all_different<Tally:Clone+Eq+Hash+Ord>(tied_candidates: &mut [CandidateIndex],transcript:  &Transcript<Tally>) -> bool {
    for count in transcript.counts.iter().rev() {
        let mut observed = HashSet::new();
        for candidate in tied_candidates.iter() {
            observed.insert(count.status.tallies.candidate[candidate.0].clone());
        }
        if observed.len()==tied_candidates.len() { // All different!
            tied_candidates.sort_by_key(|candidate|count.status.tallies.candidate[candidate.0].clone());
            return true;
        }
    }
    false
}


/// Sort candidates low to high based on the tie resolution rules.
/// Return true iff ties are resolved to the required granularity.
fn resolve_ties_any_different<Tally:Clone+Eq+Hash+Ord>(tied_candidates: &mut [CandidateIndex],transcript:  &Transcript<Tally>,granularity:TieResolutionGranularityNeeded) -> bool {
    //println!("Resolve ties any different between {}",tied_candidates.iter().map(|c|c.to_string()).collect::<Vec<_>>().join(","));
    for count in transcript.counts.iter().rev() {
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
                        TieResolutionGranularityNeeded::Total => {ok=ok&&resolve_ties_any_different(who,transcript,granularity)}  // could optimize to start at count currently up to.
                        TieResolutionGranularityNeeded::LowestSeparated(loc) if loc>upto && loc<upto+who.len() => {ok=ok&&resolve_ties_any_different(who,transcript,TieResolutionGranularityNeeded::LowestSeparated(loc-upto))}
                        TieResolutionGranularityNeeded::LowestSeparated(_) => {} // granularity means we don't care.
                    }
                }
                tied_candidates[upto..upto+who.len()].copy_from_slice(who);
                upto+=who.len();
            }
            //println!("Solution is : {}",tied_candidates.iter().map(|c|c.to_string()).collect::<Vec<_>>().join(","));
            return ok;
        }
    }
    false
}