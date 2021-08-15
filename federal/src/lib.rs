use stv::preference_distribution::PreferenceDistributionRules;
use stv::ballot_pile::{BallotPaperCount, DoNotSplitByCountNumber};
use stv::transfer_value::{TransferValue, LostToRounding};

pub mod parse;

pub struct FederalRules {
}

impl PreferenceDistributionRules for FederalRules {
    type Tally = usize;
    type SplitByNumber = DoNotSplitByCountNumber;

    fn make_transfer_value(surplus: usize, ballots: BallotPaperCount) -> TransferValue {
        TransferValue::from_surplus(surplus,ballots)
    }

    fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> (usize, LostToRounding) {
        transfer_value.mul_rounding_down(ballots)
    }
}

