// Copyright 2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Some utility routines using pseudo-random numbers.


use rand::RngCore;
use rand::distributions::{Distribution, Uniform};

/// Make a boolean array of length len such that num_true of them are true.
/// ```
/// use rand::thread_rng;
/// use stv::random_util::make_array_with_some_randomly_true;
/// let a4_10 = make_array_with_some_randomly_true(10,4,& mut thread_rng());
/// assert_eq!(10,a4_10.len());
/// assert_eq!(4,a4_10.iter().filter(|v|**v).count());
/// let a7_10 = make_array_with_some_randomly_true(10,7,& mut thread_rng());
/// assert_eq!(10,a7_10.len());
/// assert_eq!(7,a7_10.iter().filter(|v|**v).count());
/// ```
pub fn make_array_with_some_randomly_true<R:RngCore + ?Sized>(len:usize,num_true:usize,rng:&mut R) -> Vec<bool> {
    let inverse = num_true>len/2;
    let mut res = vec![inverse;len];
    let mut togo = if inverse {len-num_true} else {num_true};
    let uniform = Uniform::from(0..len);
    while togo>0 {
        let pos = uniform.sample(rng);
        if res[pos]==inverse { res[pos]=!inverse; togo-=1; }
    }
    res
}