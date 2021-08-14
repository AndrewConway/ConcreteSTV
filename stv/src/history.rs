//! The transcript of results

use serde::Serialize;
use serde::Deserialize;

/// The index of a count. 0 means the first. This is different from the human readable
/// count, which may be more complex and have sub-counts as well.
#[derive(Copy,Clone,Debug,Serialize,Deserialize,Ord, PartialOrd, Eq, PartialEq,Hash)]
pub struct CountIndex(pub(crate) usize);



