// Copyright 2023-2025 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Some utility routines using pseudo-random numbers.


use rand::distr::{Distribution, Uniform};
use rand::prelude::SliceRandom;
use crate::ballot_metadata::CandidateIndex;


/// There is need of randomness in a variety of situations in STV counting
/// * For tie resolution
/// * For the NSW randomized algorithms, for selecting which votes are the surplus.
///
/// This defines how that randomness is generated.
pub enum Randomness {
    /// Resolve ties by favouring candidates who are lower down on the ballot. This seems to have been done by the AEC some years, but may have been coincidence.
    /// Resolve the NSW random selection of excess by choosing them chronologically from the start.
    ReverseDonkeyVote,
    /// Use a pseudo random number generator.
    PRNG(rand_chacha::ChaCha20Rng)
}


impl Randomness {
    /// If all else fails, resolve draws randomly.
    pub fn resolve(&mut self,tied_candidates: &mut [CandidateIndex]) {
        // Sort by reverse donkey vote. Necessary even if using random sorting so that the same PRNG seed produces the same results (the order may be different because Hash table ordering is not guaranteed to be repeatable).
        tied_candidates.sort_by_key(|c|c.0);
        match self {
            Randomness::ReverseDonkeyVote => {}
            Randomness::PRNG(prng) => { tied_candidates.shuffle(prng); }
        }
    }

    /// Make a boolean array of length len such that num_true of them are true.
    /// If the randomness is ReverseDonkeyVote, take the first n.
    /// ```
    /// use rand::SeedableRng;
    /// let mut prng = stv::random_util::Randomness::PRNG(rand_chacha::ChaCha20Rng::seed_from_u64(1));
    /// let a4_10 = prng.make_array_with_some_randomly_true(10,4);
    /// assert_eq!(10,a4_10.len());
    /// assert_eq!(4,a4_10.iter().filter(|v|**v).count());
    /// let a7_10 = prng.make_array_with_some_randomly_true(10,7);
    /// assert_eq!(10,a7_10.len());
    /// assert_eq!(7,a7_10.iter().filter(|v|**v).count());
    /// let mut donkey = stv::random_util::Randomness::ReverseDonkeyVote;
    /// let a4_6 = donkey.make_array_with_some_randomly_true(6,4);
    /// assert_eq!(vec![true,true,true,true,false,false],a4_6);
    /// ```
    pub fn make_array_with_some_randomly_true(&mut self,len:usize,num_true:usize) -> Vec<bool> {
        assert!(num_true<=len);
        match self {
            Randomness::ReverseDonkeyVote => {
                let mut res = vec![false;len];
                for i in 0..num_true { res[i]=true; }
                res
            }
            Randomness::PRNG(prng) => {
                let inverse = num_true>len/2;
                let mut res = vec![inverse;len];
                let mut togo = if inverse {len-num_true} else {num_true};
                if togo>0 && len>0 { // if len is zero, then the Uniform creator below will crash.
                    let uniform = Uniform::new(0,len).expect("len = 0");
                    while togo>0 {
                        let pos = uniform.sample(prng);
                        if res[pos]==inverse { res[pos]=!inverse; togo-=1; }
                    }
                }
                res
            }
        }
    }

}

use rand::SeedableRng;
impl From<Option<u64>> for Randomness {
    fn from(value: Option<u64>) -> Self {
        match value {
            None => Randomness::ReverseDonkeyVote,
            Some(_) => Randomness::PRNG(rand_chacha::ChaCha20Rng::seed_from_u64(1)),
        }
    }
}