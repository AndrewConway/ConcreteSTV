
use serde::Serialize;
use serde::Deserialize;

#[derive(Clone,Debug,Serialize,Deserialize,Ord, PartialOrd, Eq, PartialEq,Hash)]
pub struct TransferValue(num::rational::BigRational);

