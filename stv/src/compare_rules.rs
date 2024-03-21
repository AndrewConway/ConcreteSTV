// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use crate::preference_distribution::{PreferenceDistributionRules, WhenToDoElectCandidateClauseChecking, TransferValueMethod, SurplusTransferMethod, LastParcelUse};
use crate::election_data::ElectionData;
use crate::distribution_of_preferences_transcript::{Transcript, TranscriptWithMetadata};
use std::fs::File;
use std::path::PathBuf;
use crate::ballot_metadata::ElectionMetadata;
use crate::compare_transcripts::{DifferenceBetweenTranscripts, compare_transcripts};
use serde::{Serialize,Deserialize};
use std::fmt::{Debug, Display, Formatter};
use crate::ballot_pile::BallotPaperCount;
use crate::transfer_value::TransferValue;
use crate::tie_resolution::MethodOfTieResolution;
use std::marker::PhantomData;
use std::str::FromStr;
use num::BigRational;
use crate::official_dop_transcript::CanConvertToF64PossiblyLossily;
use crate::random_util::Randomness;

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct CompareRules {
    pub dir : String,
}

/// Which rules are being compared here?
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct RuleComparisonDefinition {
    pub rule1 : String,
    pub rule2 : String,
}

impl Display for RuleComparisonDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{} vs {}",self.rule1,self.rule2)
    }
}

/// Comparisons on a single set of electoral data
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct CompareRulesOneDataset {
    dataset : ElectionMetadata,
    /// results are in same order as datasets in the CompareRulesResults
    pub results : Vec<DifferenceBetweenTranscripts>,
}

/// Comparisons of various rules on various data.
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct CompareRulesResults {
    comparisons : Vec<RuleComparisonDefinition>,
    datasets : Vec<CompareRulesOneDataset>,
}

impl CompareRules {

    fn directory(&self) -> PathBuf { PathBuf::from(&self.dir) }

    fn save<T:?Sized + Serialize>(&self,data:&T,name:&str) -> anyhow::Result<()> {
        let dir = self.directory();
        std::fs::create_dir_all(&dir)?;
        let file = File::create(dir.join(name))?;
        serde_json::to_writer(file,data)?;
        Ok(())
    }

    fn compute<Rules:PreferenceDistributionRules>(&self,data:&ElectionData) -> anyhow::Result<Transcript<Rules::Tally>> {
        let transcript = data.distribute_preferences::<Rules>(&mut Randomness::ReverseDonkeyVote);
        let transcript = TranscriptWithMetadata{ metadata: data.metadata.clone(), transcript };
        let name = data.metadata.name.identifier()+"_"+&Rules::name()+".transcript";
        self.save(&transcript,&name)?;
        Ok(transcript.transcript)
    }

    pub fn compare_alternate_rules_2_of_1<R:PreferenceDistributionRules>(&self,transcript:&Transcript<R::Tally>,data:&ElectionData,comparisons : &mut Vec<RuleComparisonDefinition>,results :&mut Vec<DifferenceBetweenTranscripts>) -> anyhow::Result<()> {
        if R::when_to_check_if_just_two_standing_for_shortcut_election()!=WhenToDoElectCandidateClauseChecking::AfterDeterminingWhoToExcludeButBeforeTransferringAnyPapers {
            struct AltRule<R:PreferenceDistributionRules> {
                junk : PhantomData<R>,
            }
            impl <R:PreferenceDistributionRules> PreferenceDistributionRules for AltRule<R> {
                type Tally = R::Tally;
                type SplitByNumber = R::SplitByNumber;

                fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { R::use_last_parcel_for_surplus_distribution() }
                fn transfer_value_method() -> TransferValueMethod { R::transfer_value_method() }
                fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { R::convert_tally_to_rational(tally) }
                fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { R::convert_rational_to_tally_after_applying_transfer_value(rational) }
                fn make_transfer_value(surplus: Self::Tally, ballots: BallotPaperCount) -> TransferValue { R::make_transfer_value(surplus,ballots) }
                fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> Self::Tally { R::use_transfer_value(transfer_value,ballots) }

                fn surplus_distribution_subdivisions() -> SurplusTransferMethod { R::surplus_distribution_subdivisions() }
                fn sort_exclusions_by_transfer_value() -> bool { R::sort_exclusions_by_transfer_value() }

                fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { R::resolve_ties_elected_one_of_last_two() }
                fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { R::resolve_ties_elected_by_quota() }
                fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { R::resolve_ties_elected_all_remaining() }
                fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { R::resolve_ties_choose_lowest_candidate_for_exclusion() }

                fn check_elected_if_in_middle_of_surplus_distribution() -> bool { R::check_elected_if_in_middle_of_surplus_distribution() }
                fn check_elected_if_in_middle_of_exclusion() -> bool { R::check_elected_if_in_middle_of_exclusion() }
                fn finish_all_counts_in_elimination_when_all_elected() -> bool { R::finish_all_counts_in_elimination_when_all_elected() }
                fn finish_all_surplus_distributions_when_all_elected() -> bool { R::finish_all_surplus_distributions_when_all_elected() }
                fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterDeterminingWhoToExcludeButBeforeTransferringAnyPapers }
                fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { R::when_to_check_if_all_remaining_should_get_elected() }

                fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { R::when_to_check_if_top_few_have_overwhelming_votes() }

                fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { R::should_eliminate_multiple_candidates_federal_rule_13a() }
                fn count_set_aside_due_to_transfer_value_limit_as_rounding() -> bool { R::count_set_aside_due_to_transfer_value_limit_as_rounding() }
                fn name() -> String { R::name()+"_Earliest1of2" }

            }
            let alt_transcript = self.compute::<AltRule<R>>(data)?;
            let diff = compare_transcripts(transcript,&alt_transcript);
            comparisons.push(RuleComparisonDefinition{ rule1: transcript.rules.clone(), rule2: alt_transcript.rules.clone() });
            results.push(diff);
        }
        if R::when_to_check_if_just_two_standing_for_shortcut_election()!=WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing {
            struct AltRule<R:PreferenceDistributionRules> {
                junk : PhantomData<R>,
            }
            impl <R:PreferenceDistributionRules> PreferenceDistributionRules for AltRule<R> {
                type Tally = R::Tally;
                type SplitByNumber = R::SplitByNumber;
                fn use_last_parcel_for_surplus_distribution() -> LastParcelUse { R::use_last_parcel_for_surplus_distribution() }
                fn transfer_value_method() -> TransferValueMethod { R::transfer_value_method() }
                fn convert_tally_to_rational(tally: Self::Tally) -> BigRational { R::convert_tally_to_rational(tally) }
                fn convert_rational_to_tally_after_applying_transfer_value(rational: BigRational) -> Self::Tally { R::convert_rational_to_tally_after_applying_transfer_value(rational) }
                fn make_transfer_value(surplus: Self::Tally, ballots: BallotPaperCount) -> TransferValue { R::make_transfer_value(surplus,ballots) }
                fn use_transfer_value(transfer_value: &TransferValue, ballots: BallotPaperCount) -> Self::Tally { R::use_transfer_value(transfer_value,ballots) }

                fn surplus_distribution_subdivisions() -> SurplusTransferMethod { R::surplus_distribution_subdivisions() }
                fn sort_exclusions_by_transfer_value() -> bool { R::sort_exclusions_by_transfer_value() }

                fn resolve_ties_elected_one_of_last_two() -> MethodOfTieResolution { R::resolve_ties_elected_one_of_last_two() }
                fn resolve_ties_elected_by_quota() -> MethodOfTieResolution { R::resolve_ties_elected_by_quota() }
                fn resolve_ties_elected_all_remaining() -> MethodOfTieResolution { R::resolve_ties_elected_all_remaining() }
                fn resolve_ties_choose_lowest_candidate_for_exclusion() -> MethodOfTieResolution { R::resolve_ties_choose_lowest_candidate_for_exclusion() }

                fn check_elected_if_in_middle_of_surplus_distribution() -> bool { R::check_elected_if_in_middle_of_surplus_distribution() }
                fn check_elected_if_in_middle_of_exclusion() -> bool { R::check_elected_if_in_middle_of_exclusion() }
                fn when_to_check_if_top_few_have_overwhelming_votes() -> WhenToDoElectCandidateClauseChecking { R::when_to_check_if_top_few_have_overwhelming_votes() }

                fn finish_all_counts_in_elimination_when_all_elected() -> bool { R::finish_all_counts_in_elimination_when_all_elected() }
                fn finish_all_surplus_distributions_when_all_elected() -> bool { R::finish_all_surplus_distributions_when_all_elected() }
                fn when_to_check_if_just_two_standing_for_shortcut_election() -> WhenToDoElectCandidateClauseChecking { WhenToDoElectCandidateClauseChecking::AfterCheckingQuotaIfNoUndistributedSurplusExistsAndExclusionNotOngoing }
                fn when_to_check_if_all_remaining_should_get_elected() -> WhenToDoElectCandidateClauseChecking { R::when_to_check_if_all_remaining_should_get_elected() }
                fn should_eliminate_multiple_candidates_federal_rule_13a() -> bool { R::should_eliminate_multiple_candidates_federal_rule_13a() }
                fn count_set_aside_due_to_transfer_value_limit_as_rounding() -> bool { R::count_set_aside_due_to_transfer_value_limit_as_rounding() }
                fn name() -> String { R::name()+"_Latest1of2" }
            }
            let alt_transcript = self.compute::<AltRule<R>>(data)?;
            let diff = compare_transcripts(transcript,&alt_transcript);
            comparisons.push(RuleComparisonDefinition{ rule1: transcript.rules.clone(), rule2: alt_transcript.rules.clone() });
            results.push(diff);
        }
        Ok(())
    }

    pub fn compare_alternate_rules<R:PreferenceDistributionRules>(&self,transcript:&Transcript<R::Tally>,data:&ElectionData,comparisons : &mut Vec<RuleComparisonDefinition>,results :&mut Vec<DifferenceBetweenTranscripts>) -> anyhow::Result<()> {
        self.compare_alternate_rules_2_of_1::<R>(transcript,data,comparisons,results)?;
        Ok(())
    }

    /// This should be more general, rather than restricted to 4 rules.
    pub fn compute_dataset<CommonTally:PartialEq+Clone+FromStr+Display+Debug+CanConvertToF64PossiblyLossily,R1,R2,R3,R4>(&self,data:&ElectionData) -> anyhow::Result<(Vec<RuleComparisonDefinition>,CompareRulesOneDataset)>
    where
        R1: PreferenceDistributionRules<Tally=CommonTally>,
        R2: PreferenceDistributionRules<Tally=CommonTally>,
        R3: PreferenceDistributionRules<Tally=CommonTally>,
        R4: PreferenceDistributionRules<Tally=CommonTally>,
    {
        let mut comparisons : Vec<RuleComparisonDefinition> = vec![];
        let mut results = vec![];
        let transcripts : Vec<Transcript<CommonTally>> = vec![
            self.compute::<R1>(data)?,
            self.compute::<R2>(data)?,
            self.compute::<R3>(data)?,
            self.compute::<R4>(data)?,
        ];
        self.compare_alternate_rules::<R1>(&transcripts[0],data,&mut comparisons,&mut results)?;
        self.compare_alternate_rules::<R2>(&transcripts[1],data,&mut comparisons,&mut results)?;
        self.compare_alternate_rules::<R3>(&transcripts[2],data,&mut comparisons,&mut results)?;
        self.compare_alternate_rules::<R4>(&transcripts[3],data,&mut comparisons,&mut results)?;
        for i in 0..transcripts.len() {
            for j in 0..i {
                let diff = compare_transcripts(&transcripts[i],&transcripts[j]);
                comparisons.push(RuleComparisonDefinition{ rule1: transcripts[i].rules.clone(), rule2: transcripts[j].rules.clone() });
                println!("{} vs {} : {}",transcripts[i].rules,transcripts[j].rules,diff);
                results.push(diff);
            }
        }
        Ok((comparisons,CompareRulesOneDataset{ dataset: data.metadata.clone(), results }))
    }

    pub fn compare_datasets<CommonTally:PartialEq+Clone+FromStr+Display+Debug+CanConvertToF64PossiblyLossily,R1,R2,R3,R4,I>(&self,data_iterator:I) -> anyhow::Result<CompareRulesResults>
        where
            R1: PreferenceDistributionRules<Tally=CommonTally>,
            R2: PreferenceDistributionRules<Tally=CommonTally>,
            R3: PreferenceDistributionRules<Tally=CommonTally>,
            R4: PreferenceDistributionRules<Tally=CommonTally>,
            I : IntoIterator<Item = anyhow::Result<ElectionData>>,
    {
        let mut comparisons : Vec<RuleComparisonDefinition> = vec![];
        let mut datasets : Vec<CompareRulesOneDataset> = vec![];
        for data in data_iterator {
            let data=data?;
            let (comparison,dataset) = self.compute_dataset::<CommonTally,R1,R2,R3,R4>(&data)?;
            if datasets.is_empty() { comparisons=comparison};
            datasets.push(dataset);
        }
        let res = CompareRulesResults{ comparisons, datasets };
        self.save(&res,"Comparison.json")?;
        res.pretty_print();
        Ok(res)
    }
}

impl CompareRulesResults {
    /// write out in a sort of tabulated form.
    pub fn pretty_print(&self) {
        for dataset in &self.datasets {
            println!("\n{}\n",dataset.dataset.name.human_readable_name());
            for i in 0..self.comparisons.len() {
                println!("{} : {}",self.comparisons[i],dataset.results[i]);
            }
        }
    }
}
