// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::hash::Hash;
use std::collections::HashSet;

/// A utility to see if all values passing by are the same value.
pub struct DetectUnique<T:Eq> {
    seen_multiple : bool,
    res : Option<T>,
}

impl <T:Eq> Default for DetectUnique<T> {
    fn default() -> Self { DetectUnique{seen_multiple:false, res:None}}
}
impl <T:Eq> DetectUnique<T> {
    /// observe a value passing by
    pub fn add(&mut self,v:T) {
        if !self.seen_multiple {
            match &self.res {
                None => { self.res=Some(v); }
                Some(existing) => {
                    if *existing!=v {
                        self.res=None;
                        self.seen_multiple=true;
                    }
                }
            }
        }
    }
    /// clear, and return the unique object if it is one.
    pub fn take(&mut self) -> Option<T> { self.res.take() }
}


/// A little utility to collect all unique values that pass by.
pub struct CollectAll<T:Eq+Hash+Ord> {
    all : HashSet<T>
}

impl <T:Eq+Hash+Ord> Default for CollectAll<T> {
    fn default() -> Self { CollectAll{ all: HashSet::default() } }
}

impl <T:Eq+Hash+Ord>  CollectAll<T> {
    pub fn add(&mut self,t:T) { self.all.insert(t); }

    /// clear and return a sorted list of unique elements.
    pub fn take(&mut self) -> Vec<T> {
        let mut res : Vec<T> = self.all.drain().collect();
        res.sort();
        res.dedup();
        res
    }
}

impl<T:Eq+Hash+Ord> Extend<T> for CollectAll<T>
{
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) { self.all.extend(iter); }

}

impl<'a, T> Extend<&'a T> for CollectAll<T>
    where
        T: 'a + Eq + Hash + Copy +Ord,
{
    #[inline]
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().cloned());
    }

}
