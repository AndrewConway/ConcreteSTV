
use serde::Serialize;
use serde::Deserialize;
use num::{One, BigRational, BigInt, ToPrimitive};
use crate::ballot_pile::BallotPaperCount;

pub struct LostToRounding(pub f64);

#[derive(Clone,Debug,Serialize,Deserialize,Ord, PartialOrd, Eq, PartialEq,Hash)]
pub struct TransferValue(pub(crate) num::rational::BigRational);

impl TransferValue {
    pub fn one() -> Self { TransferValue(num::rational::BigRational::one())}
    pub fn new(numerator:BigInt,denominator:BigInt) -> Self {
        TransferValue(BigRational::new(numerator,denominator))
    }
    pub fn from_surplus(surplus:usize,denominator:BallotPaperCount) -> Self {
        TransferValue::new(BigInt::from(surplus),BigInt::from(denominator.0))
    }

    pub fn mul(&self,papers:BallotPaperCount) -> num::rational::BigRational {
        BigRational::new(self.0.numer().clone()*BigInt::from(papers.0),self.0.denom().clone())
    }

    pub fn mul_rounding_down(&self,papers:BallotPaperCount) -> (usize,LostToRounding) {
        let exact = self.mul(papers);
        let rounded_down = exact.numer().clone()/exact.denom().clone();
        let frac = (exact-rounded_down.clone()).to_f64().unwrap();
        (rounded_down.to_usize().unwrap(),LostToRounding(frac))
    }
}

