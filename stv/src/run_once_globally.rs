// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! This deals with the problem of the same thing getting executed twice simultaneously,
//! causing excess resource usage and possibly deadlocks or interference.



use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Mutex;
use futures::future::Shared;
use futures::FutureExt;

pub struct RunOnceController<K,V>
where K:  Clone+Eq+Hash,
V:Clone,
{
    map : Mutex<HashMap<K,Shared<Pin<Box<dyn Future<Output=V>+Send>>>>>
}

impl <K:Clone+Eq+Hash,V:Clone> Default for RunOnceController<K,V> {
    fn default() -> Self {
        RunOnceController{ map: Mutex::new(Default::default()) }
    }
}

type SendablePinnedBoxedFuture<V> = Pin<Box<dyn Future<Output = V> + Send>>;

impl<K,V> RunOnceController<K,V>
    where
        K: Eq,
        K: Hash,
        K: Clone,
        V : Clone,
{
    /// Get an element (well, a future for the element) for the provided calculation.
    /// Non-blocking, as long as calculation returns immediately (which it does for async functions)
    ///
    /// Makes sure that there are not two things computing the same thing
    /// simultaneously. In particular, if called twice with the same
    /// argument, then the first will start computing. If the first has finished
    /// key before the second is called, then the second will also compute. However
    /// if the second is called before the first is finished, it will return a cloned
    /// copy of the first one's output.
    ///
    /// So the point is to stop the same calculation being done simultaneously.
    /// This works well if the calculation itself is cached... with the same key.
    /// In this case, the caching is only done once, which would otherwise be a waste.
    ///
    ///# Example
    ///
    ///```
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicI32, Ordering};
    /// use stv::run_once_globally::RunOnceController;
    /// let work_counter = Arc::new(AtomicI32::new(0));
    /// let once = RunOnceController::default();
    /// use futures::executor::block_on;
    ///
    /// let f1 = once.get(&7,||async { 1 });
    /// assert_eq!(1,block_on(f1));
    /// let f2 = once.get(&7,||async { 2 });
    /// assert_eq!(2,block_on(f2));
    /// /// make sure one thread starts before the second ends
    /// let f1 = once.get(&7,||async { async_std::task::sleep(std::time::Duration::from_millis(5)).await; 1 });
    /// let f2 = once.get(&7,||async { async_std::task::sleep(std::time::Duration::from_millis(5)).await; 2 });
    /// let (r1,r2) = block_on(async{futures::join!(f1,f2)}) ;
    /// assert_eq!(r1,r2); // they may both be 1 (probably) or both be 2 depending which started first.
    ///```
    pub async fn get<F,FR>(&self,argument:&K,calculation : F) -> V
        where
            F : FnOnce() -> FR,
            FR : Future<Output = V>,
            FR : Send,
            FR : 'static,
    {
        let (accessor,disposal) = {
            let mut cache = self.map.lock().unwrap();
            match cache.get(argument) {
                Some(res) => (res.clone(),None),
                None => {
                    let pref : FR = calculation();
                    let f : SendablePinnedBoxedFuture<V> = Box::pin(pref);
                    let fs : Shared<SendablePinnedBoxedFuture<V>> = f.shared();
                    let res = fs.clone();
                    cache.insert(argument.clone(),fs);
                    //println!("SequenceCache just added value. Length now {}",cache.len());
                    (res,Some(argument))
                }
            }
        };
        let res = accessor.await;
        if let Some(finished_with_arg) = disposal {
            let mut cache = self.map.lock().unwrap();
            cache.remove(&finished_with_arg);
        }
        res
    }
}
