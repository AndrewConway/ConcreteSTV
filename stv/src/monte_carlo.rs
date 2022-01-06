// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! Some utilities useful for Monte-Carlo experiments.


use rand::Rng;

#[derive(Default,Clone)]
pub struct SampleWithReplacement<E> {
    elements : Vec<E>
}

impl <E:Clone> SampleWithReplacement<E> {
    /// add an element that could be chosen
    pub fn add(&mut self,e:E) {
        self.elements.push(e);
    }

    /// Add an element multiple times.
    pub fn add_multiple(&mut self,e:E,n:usize) {
        for _ in 0..n { self.add(e.clone()); }
    }

    /// Get a random element from this range.
    pub fn get(&self,rng:&mut impl Rng) -> E {
        self.elements[rng.gen_range(0..self.elements.len())].clone()
    }
}


