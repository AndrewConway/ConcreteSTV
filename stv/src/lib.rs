pub mod ballot_paper;
pub mod ballot_metadata;
pub mod election_data;
pub mod ballot_pile;
pub mod history;
pub mod transfer_value;
pub mod preference_distribution;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
