// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::fs::File;
use stv::ballot_metadata::{ElectionName, Candidate, CandidateIndex, PartyIndex, ElectionMetadata, DataSource, NumberOfCandidates};
use stv::ballot_paper::{RawBallotMarking, parse_marking, RawBallotMarkings, UniqueVoteBuilderMultipleTypes};
use std::collections::{HashMap};
use csv::{StringRecord, StringRecordsIntoIter};
use zip::ZipArchive;
use zip::read::ZipFile;
use anyhow::anyhow;
use stv::election_data::ElectionData;
use stv::distribution_of_preferences_transcript::{CountIndex, QuotaInfo};
use serde::Deserialize;
use stv::ballot_pile::BallotPaperCount;
use stv::datasource_description::{AssociatedRules, Copyright, ElectionDataSource};
use stv::official_dop_transcript::{candidate_elem, OfficialDistributionOfPreferencesTranscript, OfficialDOPForOneCount};
use stv::tie_resolution::TieResolutionsMadeByEC;
use stv::parse_util::{CandidateAndGroupInformationBuilder, skip_first_line_of_file, GroupBuilder, RawDataSource, MissingFile, FileFinder, RawBallotPaperMetadata, CanReadRawMarkings, read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions, add_atl_how_to_vote_to_metadata};
use crate::{FederalRulesUsed2013, FederalRulesUsed2016, FederalRulesUsed2019};
use crate::parse2013::{read_from_senate_group_voting_tickets_download_file2013, read_ticket_votes2013, read_btl_votes2013};

pub fn get_federal_data_loader_2013(finder:&FileFinder) -> FederalDataLoader {
    FederalDataLoader::new(finder,"2013",false,"https://results.aec.gov.au/17496/Website/SenateDownloadsMenu-17496-Csv.htm",17496)
}

pub fn get_federal_data_loader_2014(finder:&FileFinder) -> FederalDataLoader {
    FederalDataLoader::new(finder,"2014",false,"https://results.aec.gov.au/17875/Website/SenateDownloadsMenu-17875-csv.htm",17875)
}

pub fn get_federal_data_loader_2016(finder:&FileFinder) -> FederalDataLoader {
    FederalDataLoader::new(finder,"2016",true,"https://results.aec.gov.au/20499/Website/SenateDownloadsMenu-20499-Csv.htm",20499)
}

pub fn get_federal_data_loader_2019(finder:&FileFinder) -> FederalDataLoader {
    FederalDataLoader::new(finder,"2019",false,"https://results.aec.gov.au/24310/Website/SenateDownloadsMenu-24310-Csv.htm",24310)
}

pub fn get_federal_data_loader_2022(finder:&FileFinder) -> FederalDataLoader {
    FederalDataLoader::new(finder,"2022",false,"https://results.aec.gov.au/27966/Website/SenateDownloadsMenu-27966-Csv.htm",27966)
}

pub fn get_federal_data_loader_2025(finder:&FileFinder) -> FederalDataLoader {
    FederalDataLoader::new(finder,"2025",false,"https://results.aec.gov.au/",31496)
}



pub struct FederalDataSource {}

impl ElectionDataSource for FederalDataSource {
    fn name(&self) -> Cow<'static, str> { "Federal Senate".into() }
    fn ec_name(&self) -> Cow<'static, str> { "Australian Electoral Commission (AEC)".into() }
    fn ec_url(&self) -> Cow<'static, str> { "https://www.aec.gov.au/".into() }
    fn years(&self) -> Vec<String> { vec!["2013".to_string(),"2014".to_string(),"2016".to_string(),"2019".to_string(),"2022".to_string(),"2025".to_string()] }
    fn get_loader_for_year(&self,year: &str,finder:&FileFinder) -> anyhow::Result<Box<dyn RawDataSource+Send+Sync>> {
        match year {
            "2013" => Ok(Box::new(get_federal_data_loader_2013(finder))),
            "2014" => Ok(Box::new(get_federal_data_loader_2014(finder))),
            "2016" => Ok(Box::new(get_federal_data_loader_2016(finder))),
            "2019" => Ok(Box::new(get_federal_data_loader_2019(finder))),
            "2022" => Ok(Box::new(get_federal_data_loader_2022(finder))),
            "2025" => Ok(Box::new(get_federal_data_loader_2025(finder))),
            _ => Err(anyhow!("Not a valid year")),
        }
    }
}


pub struct FederalDataLoader {
    finder : FileFinder,
    archive_location : String,
    year : String,
    double_dissolution : bool,
    page_url : String,
    election_number : usize,
}

impl FederalDataLoader {
    /// This terrible cryptic monstrosity is a hand compiled collection of how to vote recommendations
    /// where I am aware of them from parties advising voters on how they recommend they vote.
    /// 
    /// These are generally a list of parties, with their own party as the first one. Parties are
    /// listed by AEC column (a letter starting from A). Each string is a single party's how to vote
    /// card with the party columns separated by whitespace.
    /// 
    /// These are not reported in any central database, but are collected by the national 
    /// library and also public reporting by the ABC, e.g.
    /// https://www.abc.net.au/news/elections/federal/2022/guide/senate-nsw-htv
    ///
    /// Unfortunately these come from my interpretation of images and 
    /// are very hard to test against a mistake. If anyone spots an error, 
    /// please report it!
    /// 
    /// Some parties have multiple recommendations, in which case all are included.
    /// 
    /// Some parties recommend voting for a small number of specific people (usually just the 
    /// party in question) and then filling our further by your choice, usually at least 6
    /// due to the AEC's statement to voters to do so. These are recorded as the small number of
    /// specific parties.
    ///
    /// One party one time (liberal, ACT, 2022) just listed themselves without recommending
    /// other ATL votes. This is also listed as just that one party.
    /// 
    /// Some parties recommend voting themselves first, and then recommend weakly a set of next
    /// preferences in a particular order. These weak recommendations are included.
    /// 
    /// Some parties recommend voting themselves first, and then list a set of suggested
    /// next preferences not in a particular order. These unordered recommendations are not
    /// included.
    /// 
    /// Some parties had errors in their How To Vote cards which have been fixed to my interpretation
    /// of what was intended. For instance in TAS 2016, One Nation recommended a third preference for
    /// group "AG - Shooters, Fishers and Farmers" by which I assume they meant group P (there is no
    /// group AG). 
    pub(crate) fn get_how_to_vote_cards(&self, state: &str) -> anyhow::Result<&[&str]> {
        Ok(match self.year.as_str() {
            "2013" | "2014" => &[], // formal tickets in use.
            "2016" => match state {
                "ACT" => &["A","C H J G B A","D I F A G E","E G","F I A G E C","G E B","H B J G C E"],
                "NSW" => &["C H D X AM AF","D J AG C AM X","F AF J C D AA","H C AF X J A","I AC AL AB AG N","J AF AM M D F","L AL AH K AJ AO AB N","M S AF H J C","N AL AN AB AG D","P","Q","R AL AG I N AN","S M AB H AM AD","T P B I V","W","AB AK AN A AL N","AD","AG AO K AN U AB","AH","AI","AK AB AN","AL R I L AB N","AM C J X S D","AN AB AK AG AL N AO E I"],
                "NT" => &["A G C E F D","D F B E C G","E G F B D A","F D B G E C"],
                "QLD" => &["A B H AK J D","D AK V AD AC I","E AG J","G T AA Q I AF","H AL U A AK D","I AC Q N T X","L","M AK V D U C","N X T I Q G","O","Q Y X AA N G","S Y AF I AH AA Q T AI AJ X G","T Y N I AC X G","U AL H AH AK D V AG W","V J H C B A","X N Y Q T I","Y T X AF Q S AA","Z","AC I T AK D G","AD","AE","AF T Y AA S N G","AJ O K AI AH AD","AK U J B D A","AL H U"],
                "SA" => &["B D U P F J","D U E P C B","E D U R C B","F","G","H N K M J Q","K N Q R V S","N M S Q O K","O N Q A S J","Q N M O S K","R P V U C E","S Q K N H M","T","U D P E B V"],
                "TAS" => &["A P T N S F","B C M S H L","C L S B R U","E M","F D P T A B","H U S R Q L","I D P A S N","J","L S C H Q B J R O","M","N D T P A M","P D I A N T S","Q L R C B","R Q C H U B","T S P N"],
                "VIC" => &["A","C I M X AK D","D AL A M AK E","E X AK C Q D","G","H R O AI P AF","I C M","J AK AL E D M","K","M AL C I AK D W AC N","O H R P AB A","R H O L AA AI AF","U AG A O AE AA","V","W","X AK AL E C Q","Y AI AG P AE A","Z AJ AK X AL M D","AF O H R AA A","AG AI O Y P AH","AH W AL AG O AB","AI Y H O AG R P","AJ","AK X E C Q D","AL AD X T Q M"],
                "WA" => &["B P Q R A X","B P Q R A F","B P Q R AB X","B P Q R AB F","C","D J S H B F","G J M O S N K D","H","J O N K S D","K O N U J D","N M K J S C","O S K U J D AA N L","P B W Q AB Z","Q B R W AB T A","R Q B AB L A","S K O M J D","W AB Q A T P X","X F W B Z AB","Z I F S AB B","AB B W T P Q"],
                _ => anyhow::bail!("Invalid state or territory {state}"),
            },
            "2019" => match state {
                "ACT" => &["A E F C G B","B G C F E A","D A E F UG G","E A UG C G F","F","G B C F E A"],
                "NSW" => &["A D P M O AA Z S N H","B","D Z P R AI M","E AE G AH F AC B W AD J","G Q K AC E J","J G AC T Q B","K G W Q B F AC J","M X AA P H M A I AI D","O N M H D A","P D H S M A","Q","R B S AI AE E","S X R H P U","X S H R P M","Y C AC N P S","Z D M AH AI J","AE AH AC Q G J","AF A AA S H G","AG","AH AE L AG E Q","AI AG M R D P"],
                "NT" => &["A C G B D E","B H F D A C","C A D F B H","E C I A G D","F","G E H D A I","H F B D A C","I E C G A B"],
                "QLD" => &["A D O I P C B S W V","B Q M Y W A","C D Q O W J","D C Q O P Y","E","G U T K H J","H K L G J N","I Y A D O W","J H G Q N R","M B W Y C O","N","O","R","S P O M I W V Q Z A C D","T H G N L J","U","X O P Q R A D","Z B Q A W H"],
                "SA" => &["A L B M F C K G","B A M L G E","C","D H I C J O A","E N M H F L","G K L N C D","I","J P I D O C","K G L M D O","L","O J P C D N","P J O C H G"],
                "TAS" => &["A","B E J N H A","C","D O F M I C","E B N A H P","F D I C","G J P L N D","I O D F L M","J H P C B A","K P A B E H","L","M","N E B A I L","O D I F C M"],
                "VIC" => &["A F P AB N I","D I Y AB A P","F A AB G D X","G N Q E P AB","I D L V X A U","J","K Q W P G V","L O I B M J","P AB E W F A","Q W G N E Y","R V O L U J T S X","U V M X I AD","V U X I M L","W A E AB AA P F Q AC G","X I V U O L","Y","AA AE AC AB G A","AB","AC P E AB AE AA G W D K F A","AE AA G AB Q A","AE AA G AB Q N E P W O Y F D A I L Z AD AC B K J R S X U T M H C"],
                "WA" => &["A P N K H U","B T R L G A","C U I K N M","D Q L F J G","E S R P C A","F D L J Q W","G D H L N U","H","I C U P M O","J","K M P C U A I","L T D V Q G","M","O A P K T D","P K A C U N","Q D J L F G","R K M E P O H I U N","S E P M A C","S E P M A N K H V R T I O U C W L J F B G Q D","U C H N P A","V"],
                _ => anyhow::bail!("Invalid state or territory {state}"),
            },
            "2022" => match state {
                "ACT" => &["A E H F G J","B","D","E F","H F E G A J","I","J A F B E H","K I C E F B"],
                "NSW" => &["A E V R H B","C N M H E B","D","E A L I H V","F","G T W N K U","G W S T N K","G S T W N K","H E V R A Q","I E L Q H V","K L H D J R","M T W V P G","O P W S T Q","Q","R H K A Q E V","S U T W O M","T W S M U G","U S P M T W","V E M A T H","W T M S O U"],
                "NT" => &["A B C D E F","B F D A C E","C H B D E A","D","E","G F H A B C","H C D E A B"],
                "QLD" => &["B J K P E Y","C","D","E U P Y Q C","I","J B P C Y U","L X A F N I","N X H W M A","P Y J D O B","Q G E D K Y","R W T X N O","S X R W N Q","T M O A H X R N","W R X N O S","X N W R T O","Y J P Q E G"],
                "SA" => &["A U S E J D","B D P R N H","C G E U A R","D P B I A K","E J S A G U","G C S E U R","H","K R H I D P","O","P I D B T R","Q F J S T U","S E A U Q G","T","U A Q S J E","V O J S Q P"],
                "TAS" => &["A","D M I H C J","E","H","I C H M E D","J K B F G L","K J N B M G","L K J M F B","M F D L K I","N G H J K B"],
                "VIC" => &["A I S K U E","C A T F J U","D L H W O C","E","F","G P O W C E","I K A U M E","K U H W I A","L W X P N G","M Z U A I K","O","P G W L O N","Q O P U A G","U Z I E A K","W L P R N G","X V N R P L","Y","Z M U J I K"],
                "WA" => &["A O H D S I","C L B M G Q","D A O L F T","E","F E V K D A","G N L B C Q","I","L C G Q N B","M J Q B I C G L","N G Q L M C","O A D E S T","P B C G N M J Q L T I K V F O S E","R B T L Q F","R B T L Q G","S A H K V D","U K G N A M"],
                _ => anyhow::bail!("Invalid state or territory {state}"),
            },
            "2025" => match state { 
                "ACT" => &["A","B","C A F B G D","D B G C A E","E","G B C A D E"],
                "NSW" => &["A R J O G D","B","E I H M F N","F E I H M N","G D R Q J A","H E F I M N","I E F H M N","J G D L K R","K","M E H I O P","N H E F I O","O","Q R L C J G A","R L G Q J A"],
                "NT" => &["A","B A H E C D","C E B A D G","D F H B A C","E B C A G D","F H G D A B","G"],
                "QLD" => &["A C P E M O J","B N H G F L","G S N B K Q","I Q N S G B","J P O M F H","L G Q E P C","M D R O P J","N G Q S I K","O M D C H P","P O M A C J","Q G S B L I","R","S G Q N I K"],
                "SA" => &["A P M J G K","C H P M A F","D N F K E B","E B N K O D","F I B D N C","G M A J F L","H C F P M J","J M A G P F","K E B N F A","L","M J A C P H","N I K E D F","O","P A G L H C"],
                "TAS" => &["A","B I H L G K","C G H B K J","D E A F G H","F D E G K A","I H L C J B","J L D A I F","K G F E L D"],
                "VIC" => &["A J P K N L","B C F D L Q","C F B M Q I","G","H J L O P R","I B Q C L M","J A O P K S","K P S H J A","L","M Q E C F I","N","O P J R H K","P O K S F J","Q M B E C I","R O E D Q K"],
                "WA" => &["B M H D Q R","C","D B F H G C","E","H A G B D M","I N O K P L","K P I O J L","L I N O F K","M H G B R Q","N J O F I G","O J N E I L","Q R M B G E","R Q M G F B"],
                _ => anyhow::bail!("Invalid state or territory {state}"),
            },
            _ => anyhow::bail!("Invalid year {}",self.year),
        })
    }
}

impl RawDataSource for FederalDataLoader {
    fn name(&self,state:&str) -> ElectionName {
        ElectionName{
            year: self.year.clone(),
            authority: "AEC".to_string(),
            name: "Federal Senate".to_string(),
            electorate: state.to_string(),
            modifications: vec![],
            comment: None,
        }
    }

    fn candidates_to_be_elected(&self,state:&str) -> NumberOfCandidates {
        NumberOfCandidates(
            if state=="ACT" || state=="NT" { 2 }
            else if self.double_dissolution { 12 }
            else { 6 }
        )
    }

    /// These are deduced by looking at the actual transcript of results.
    /// I have not included anything if all decisions are handled by the fallback "earlier on the ballot paper candidates are listed in worse positions".
    fn ec_decisions(&self,state:&str) -> TieResolutionsMadeByEC {
        match self.year.as_str() {
            "2013" => match state {
                "VIC" => TieResolutionsMadeByEC::new(vec![vec![CandidateIndex(54), CandidateIndex(23),CandidateIndex(85),CandidateIndex(88)]]).unwrap() , // 4 way tie at count 10.
                "NSW" => TieResolutionsMadeByEC::new( vec![vec![CandidateIndex(82),CandidateIndex(52),CandidateIndex(54)], vec![CandidateIndex(104),CandidateIndex(68),CandidateIndex(72)], vec![CandidateIndex(56),CandidateIndex(7)], vec![CandidateIndex(20),CandidateIndex(12),CandidateIndex(96)]]).unwrap() ,
                _ => Default::default(),
            },
            _ => Default::default(),
        }
    }

    /// These are due to a variety of events.
    fn excluded_candidates(&self,state:&str) -> Vec<CandidateIndex> {
        match self.year.as_str() {
            "2016" => match state {
                "SA" => vec![CandidateIndex(38)], // Bob Day was excluded because of indirect pecuniary interest.
                "WA" => vec![CandidateIndex(45)], // Rod Cullerton was excluded because of bankruptcy and larceny.
                _ => Default::default(),
            },
            _ => Default::default(),
        }
    }

    fn find_raw_data_file(&self,filename:&str) -> Result<PathBuf,MissingFile> {
        self.finder.find_raw_data_file(filename,&self.archive_location,&self.page_url)
    }
    fn all_electorates(&self) -> Vec<String> {
        match self.year.as_str() {
            "2014" => vec!["WA".to_string()], // The 2013 WA election was reheld in 2014 due to lost ballots.
            _ => vec!["ACT".to_string(),"NT".to_string(),"TAS".to_string(),"VIC".to_string(),"NSW".to_string(),"QLD".to_string(),"SA".to_string(),"WA".to_string()]
        }

    }

    fn read_raw_data(&self,state:&str) -> anyhow::Result<ElectionData> {
        match self.year.as_str() {
            "2013" | "2014" => self.read_raw_data2013(state),
            _ => {
                let mut builder = UniqueVoteBuilderMultipleTypes::default();
                let callback = |markings:&RawBallotMarkings,_meta:&[(&str,&str)]| {
                    let collection_point = _meta[1].1;
                    let vote_type = if collection_point.starts_with("PROVISIONAL") { Some("PROVISIONAL") }
                    else if collection_point.starts_with("PRE_POLL") { Some("PRE_POLL") }
                    else if collection_point.starts_with("POSTAL") { Some("POSTAL") }
                    else if collection_point.starts_with("ABSENT") { Some("ABSENT") }
                    else {None};
                    builder.add_vote(markings.interpret_vote(1,6),vote_type);
                };
                let metadata = self.iterate_over_raw_markings(state,callback)?;
                Ok(builder.into_election_data(metadata))
            }
        }
    }

    fn read_raw_data_best_quality(&self, electorate: &str) -> anyhow::Result<ElectionData> {
        match self.year.as_str() {
            "2013" => read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<FederalRulesUsed2013,Self>(self,electorate),
            "2014" => read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<FederalRulesUsed2013,Self>(self,electorate),
            "2016" => read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<FederalRulesUsed2016,Self>(self,electorate),
            "2019" => read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<FederalRulesUsed2019,Self>(self,electorate),
            "2022" => read_raw_data_checking_against_official_transcript_to_deduce_ec_resolutions::<FederalRulesUsed2019,Self>(self,electorate),
            "2025" => self.read_raw_data(electorate), // TODO check ec resolutions
            _ => Err(anyhow!("Invalid year {}",self.year)),
        }
    }

    fn read_raw_metadata(&self,state:&str) -> anyhow::Result<ElectionMetadata> {
        let mut builder = CandidateAndGroupInformationBuilder::default();
        if self.year=="2013" || self.year=="2014" { read_from_senate_group_voting_tickets_download_file2013(&mut builder,self.find_raw_data_file(&self.name_of_candidate_source_post_election())?.as_path(),state)?; }
        else if !self.can_load_full_data(state) { read_candidate_list_file_available_before_election2022(&mut builder,self.find_raw_data_file(&self.name_of_candidate_source_pre_election()?)?.as_path(),state)?; }
        else { read_from_senate_first_prefs_by_state_by_vote_typ_download_file2016(&mut builder,self.find_raw_data_file(&self.name_of_candidate_source_post_election())?.as_path(),state)?; }
        let vacancies = self.candidates_to_be_elected(state);
        let mut metadata = ElectionMetadata{
            name: self.name(state),
            candidates: builder.candidates.clone(),
            parties: builder.extract_parties(),
            source: vec![DataSource{
                url: self.page_url.clone(),
                files: vec![self.name_of_candidate_source_post_election()],
                comments: None
            }],
            results: None,
            vacancies: Some(vacancies),
            enrolment: None,
            secondary_vacancies: if vacancies==NumberOfCandidates(12) { Some(NumberOfCandidates(6)) } else {None},
            excluded: self.excluded_candidates(state),
            tie_resolutions : self.ec_decisions(state),
        };
        add_atl_how_to_vote_to_metadata(&mut metadata,self.get_how_to_vote_cards(state)?)?;
        Ok(metadata)
    }
    fn copyright(&self) -> Copyright {
        Copyright{
            statement: Some("© Commonwealth of Australia 2017".into()),
            url: Some("https://www.aec.gov.au/footer/Copyright.htm".into()),
            license_name: Some("Creative Commons Attribution 4.0 International Licence".into()),
            license_url: Some("https://creativecommons.org/licenses/by/4.0".into())
        }
    }

    fn rules(&self, _electorate: &str) -> AssociatedRules {
        match self.year.as_str() {
            "2013" => AssociatedRules{
                rules_used: Some("AEC2013".into()),
                rules_recommended: Some("FederalPre2021".into()),
                comment: None,
                reports: vec!["https://github.com/AndrewConway/ConcreteSTV/blob/main/reports/RecommendedAmendmentsSenateCountingAndScrutiny.pdf".into()]
            },
            "2014" => AssociatedRules{
                rules_used: Some("AEC2013".into()),
                rules_recommended: Some("FederalPre2021".into()),
                comment: None,
                reports: vec!["https://github.com/AndrewConway/ConcreteSTV/blob/main/reports/RecommendedAmendmentsSenateCountingAndScrutiny.pdf".into()]
            },
            "2016" => AssociatedRules{
                rules_used: Some("AEC2016".into()),
                rules_recommended: Some("FederalPre2021".into()),
                comment: None,
                reports: vec!["https://github.com/AndrewConway/ConcreteSTV/blob/main/reports/RecommendedAmendmentsSenateCountingAndScrutiny.pdf".into()]
            },
            "2019" => AssociatedRules{
                rules_used: Some("AEC2019".into()),
                rules_recommended: Some("FederalPre2021".into()),
                comment: None,
                reports: vec!["https://github.com/AndrewConway/ConcreteSTV/blob/main/reports/RecommendedAmendmentsSenateCountingAndScrutiny.pdf".into()]
            },
            "2022" => AssociatedRules{
                rules_used: Some("AEC2019".into()),
                rules_recommended: Some("FederalPost2021".into()),
                comment: Some(Cow::Borrowed("The AEC seems to me to have used the same rules as they used in 2019 (AEC2019). This is similar to my interpretation of the legislation (FederalPost2021) other than in Queensland, where on the last count the AEC did not distribute any votes. This did not change who was elected, or, in this case, the order.")),
                reports: vec![]
            },
            "2025" => AssociatedRules{
                rules_used: Some("AEC2019".into()),
                rules_recommended: Some("FederalPost2021".into()),
                comment: Some(Cow::Borrowed("Full results are not out yet, based on current data it is not clear whether the bug in AEC2019 was fixed or not since the situation didn't appear to arise. So I don't know whether the same buggy software was used as in 2019 and 2022 or not.")), // TODO update when data is released.
                reports: vec![]
            },
            _ => AssociatedRules{rules_used:None,rules_recommended:None,comment:None,reports:vec![]},
        }
    }
    fn can_read_raw_markings(&self) -> bool  { self.year=="2016" || self.year=="2019" || self.year=="2022" || self.year=="2025" } 
    fn can_load_full_data(&self,_state:&str) -> bool { true }

    fn read_official_dop_transcript(&self,metadata:&ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
        let filename = self.name_of_official_transcript_zip_file();
        let preferences_zip_file = self.find_raw_data_file(&filename)?;
        println!("Parsing {}",&preferences_zip_file.to_string_lossy());
        let mut zipfile = ZipArchive::new(File::open(preferences_zip_file)?)?;
        {
            for i in 0..zipfile.len() {
                let file = zipfile.by_index(i)?;
                if file.name().contains(&metadata.name.electorate) {
                    return read_official_dop_transcript_work(file,metadata);
                }
            }
            Err(anyhow!("Could not find file in zipfile for {}",&metadata.name.electorate))
        }
    }

}

impl CanReadRawMarkings for FederalDataLoader {
    fn iterate_over_raw_markings<F>(&self,state:&str,mut callback:F)  -> anyhow::Result<ElectionMetadata>
        where F:FnMut(&RawBallotMarkings,RawBallotPaperMetadata)
    {
        if self.year=="2013" { return Err(anyhow!("Iterating over raw btl preferences not supported.")); }
        let mut metadata = self.read_raw_metadata(state)?;
        let filename = self.name_of_vote_source(state);
        let preferences_zip_file = self.find_raw_data_file(&filename)?;
        println!("Parsing {}",&preferences_zip_file.to_string_lossy());
        metadata.source[0].files.push(filename);
        let mut parties_that_can_get_atls = vec![];
        for i in 0..metadata.parties.len() {
            if metadata.parties[i].atl_allowed { parties_that_can_get_atls.push(PartyIndex(i)); }
        }
        let mut zipfile = ZipArchive::new(File::open(preferences_zip_file)?)?;
        let num_atl_plus_num_btl_hint = metadata.candidates.len()+metadata.parties.len();
        for record in ParsedRawVoteIterator::new(&mut zipfile,num_atl_plus_num_btl_hint)? {
            let record=record?;
            // if &record.record[4]!="1" { continue; } // test just using batch 1.
            let markings = RawBallotMarkings::new(&parties_that_can_get_atls,&record.markings);
            callback(&markings,&[("Electorate",&record.record[record.electorate_column]),("Collection Point",&record.record[record.collection_column])]);
        }
        Ok(metadata)
    }

}
impl FederalDataLoader {


    pub fn new(finder:&FileFinder,year:&'static str,double_dissolution:bool,page_url:&'static str,election_number:usize) -> Self {
        FederalDataLoader {
            finder : finder.clone(),
            archive_location: "Federal/".to_string()+year,
            year: year.to_string(),
            double_dissolution,
            page_url: page_url.to_string(),
            election_number,
        }
    }

    fn name_of_candidate_source_post_election(&self) -> String {
        match self.year.as_str() {
            "2013" | "2014" => format!("SenateGroupVotingTicketsDownload-{}.csv",self.election_number),
            _ => format!("SenateFirstPrefsByStateByVoteTypeDownload-{}.csv",self.election_number),
        }
    }
    fn name_of_candidate_source_pre_election(&self) -> anyhow::Result<String> {
        match self.year.as_str() {
            "2022" | "2025" => Ok("senate-candidates.csv".to_string()),
            _ => Err(anyhow!("No pre election formats for year {}",self.year))
        }
    }

    fn name_of_vote_source(&self,state:&str) -> String {
        format!("aec-senate-formalpreferences-{}-{}.zip",self.election_number,state)
    }
    fn name_of_official_transcript_zip_file(&self) -> String {
        format!("SenateDopDownload-{}.zip",self.election_number)
    }


    fn read_raw_data2013(&self,state:&str) -> anyhow::Result<ElectionData> {
        let mut metadata = self.read_raw_metadata(state)?;
        let filename = format!("SenateUseOfGvtByGroupDownload-{}.csv",self.election_number);
        let preferences_zip_file = self.find_raw_data_file(&filename)?;
        println!("Parsing {}",&preferences_zip_file.to_string_lossy());
        metadata.source[0].files.push(filename);
        let ticket_votes = read_ticket_votes2013(&metadata,&preferences_zip_file,state,&self.year)?;
        let filename = format!("SenateStateBtlDownload-{}-{}.zip",self.election_number,state);
        let preferences_zip_file = self.find_raw_data_file(&filename)?;
        println!("Parsing {}",&preferences_zip_file.to_string_lossy());
        metadata.source[0].files.push(filename);
        let (btl,informal) = read_btl_votes2013(&metadata, &preferences_zip_file, 1)?; // The 2013 formality rules are quite complex. I am assuming the AEC has applied them already to all with a 1 vote. This is a dubious assumption as there are some without a 1 vote. However since we don't get all the informal votes, it is hard to check formality properly.
        Ok(ElectionData{ metadata, atl:ticket_votes, atl_types: vec![], atl_transfer_values: vec![], btl, btl_types: vec![], btl_transfer_values: vec![], informal })
    }

}


fn read_official_dop_transcript_work(file : ZipFile,metadata : &ElectionMetadata) -> anyhow::Result<OfficialDistributionOfPreferencesTranscript> {
    let mut reader = csv::ReaderBuilder::new().flexible(false).has_headers(true).from_reader(file);
    #[derive(Debug, Deserialize)]
    struct Record {
        #[serde(rename = "State")] _state: String,
        #[serde(rename = "No Of Vacancies")] vacancies: usize,
        #[serde(rename = "Total Formal Papers")] formal_papers: usize,
        #[serde(rename = "Quota")] quota : usize,
        #[serde(rename = "Count")] count : usize,
        #[serde(rename = "Ballot Position")] _ballot_position : usize,
        #[serde(rename = "Ticket")] _ticket : String,
        #[serde(rename = "Surname")] surname : String,
        #[serde(rename = "GivenNm")] given_name : String,
        #[serde(rename = "Papers")] papers_transferred : isize,
        #[serde(rename = "VoteTransferred")] votes_transferred : isize,
        #[serde(rename = "ProgressiveVoteTotal")] votes_total : usize,
        #[serde(rename = "Transfer Value")] transfer_value : f64,
        #[serde(rename = "Status")] status : String, // blank, Elected, Excluded
        #[serde(rename = "Changed")] changed : String, // True or blank.
        #[serde(rename = "Order Elected")] order_elected : usize,
        #[serde(rename = "Comment")] comment: Option<String>,
    }
    let lookup_names : HashMap<String,CandidateIndex> = metadata.get_candidate_name_lookup();
    let mut res = OfficialDistributionOfPreferencesTranscript::default();
    let mut last_count : usize = 0;
    let mut order_elected : HashMap<CandidateIndex,usize> = Default::default(); // value is order elected, which is not necessarily as encountered.
    let mut excluded_last : Vec<CandidateIndex> = vec![]; // transcript marks them as excluded the round before they are excluded in.
    let mut papers_came_from_counts : Option<Vec<CountIndex>> = None;
    for result in reader.deserialize() {
        let record : Record = result?;
        if last_count==0 {
            res.quota=Some(QuotaInfo{
                papers: BallotPaperCount(record.formal_papers),
                vacancies : NumberOfCandidates(record.vacancies),
                quota: record.quota as f64
            });
        }
        if record.count!=last_count {
            last_count=record.count;
            res.finished_count();
            res.count().excluded.extend(excluded_last.drain(..));
            res.count().papers_came_from_counts = papers_came_from_counts.take();
        }
        if record.transfer_value!=0.0 { res.count().transfer_value = Some(record.transfer_value) }
        if record.surname=="Exhausted" {
            res.count().paper_delta().exhausted= record.papers_transferred;
            res.count().vote_delta().exhausted= record.votes_transferred as f64;
            res.count().vote_total().exhausted= record.votes_total as f64;
        } else if record.surname=="Gain/Loss" {
            res.count().paper_delta().rounding= record.papers_transferred.into();
            res.count().vote_delta().rounding= (record.votes_transferred as f64).into();
            res.count().vote_total().rounding= (record.votes_total as f64).into();
        } else {
            let name = record.surname+", "+&record.given_name;
            match lookup_names.get(&name) {
                None => return Err(anyhow!("Could not find name {}",name)),
                Some(&candidate) => {
                    * candidate_elem(&mut res.count().paper_delta().candidate,candidate) = record.papers_transferred;
                    * candidate_elem(&mut res.count().vote_delta().candidate,candidate)= record.votes_transferred as f64;
                    * candidate_elem(&mut res.count().vote_total().candidate,candidate)= record.votes_total as f64;
                    if &record.changed=="True" {
                        match record.status.as_str() {
                            "Excluded" => excluded_last.push(candidate),
                            "Elected" => {
                                //println!("Elected {} at count {}",candidate,res.counts.len());
                                res.count().elected.push(candidate);
                                order_elected.insert(candidate,record.order_elected);
                                res.count().elected.sort_by_key(|c|order_elected.get(c));
                            }
                            _ => return Err(anyhow!("Could not understand status {}",record.status)),
                        }
                    }
                }
            }
        }
        if papers_came_from_counts.is_none() {
            if let Some(comment) = &record.comment {
                papers_came_from_counts = OfficialDOPForOneCount::extract_counts_from_comment(comment,"Preferences received at count(s) ",".")?;
                if papers_came_from_counts.is_none() {
                    papers_came_from_counts = OfficialDOPForOneCount::extract_counts_from_comment(comment,"papers are involved from count number(s) ",".")?;
                }
            }
        }
    }
    Ok(res)
}


/// the candidate information file doesn't list the place on the ticket.
/// the SenateFirstPrefsByStateByVoteTypeDownload file does, but it isn't available until after the election.
/// the file that is available before the election is not available well after the election :-)
/// so need to be able to parse both.
/// This format is used in 2016 and 2019
fn read_from_senate_first_prefs_by_state_by_vote_typ_download_file2016(builder: &mut CandidateAndGroupInformationBuilder,path:&Path,state:&str) -> anyhow::Result<()> {
    let mut rdr = csv::Reader::from_reader(skip_first_line_of_file(path)?);
    for result in rdr.records() {
        let record = result?;
        if state==&record[0] { // right state
            let group_id = &record[1]; // something like A, B, or UG
            let candidate_id = &record[2]; // something like 32847
            if candidate_id!="0" {
                let position_in_ticket = record[3].parse::<usize>()?; // 0, 1, ... 0 means a dummy id for the group ticket.
                if builder.parties.len()==0 || &builder.parties[builder.parties.len()-1].group_id != group_id {
                    builder.parties.push(GroupBuilder{name:record[5].to_string(), abbreviation:None, group_id:group_id.to_string(),ticket_id:if position_in_ticket==0 {Some(candidate_id.to_string())} else {None}, tickets: vec![]});
                }
                if position_in_ticket!=0 { // real candidate.
                    // self.candidate_by_id.insert(candidate_id.to_string(),CandidateIndex(self.candidates.len()));
                    builder.candidates.push(Candidate{
                        name: record[4].to_string(),
                        party: Some(PartyIndex(builder.parties.len()-1)),
                        position: Some(position_in_ticket),
                        ec_id: Some(candidate_id.to_string()),
                    })
                }
            }
        }
    }
    Ok(())
}

/// This reads the file format available before the election.
/// This is the format used in 2022.
/// A similar format was used in 2016 and 2019
fn read_candidate_list_file_available_before_election2022(builder: &mut CandidateAndGroupInformationBuilder,path:&Path,state:&str) -> anyhow::Result<()> {
    let mut rdr = csv::Reader::from_path(path)?;
    for result in rdr.records() {
        let record = result?;
        if state==&record[0] { // right state
            let group_id = &record[1]; // something like A, B, or UG
            let position_in_ticket = record[2].parse::<usize>()?; // 1,2.,,,
            if builder.parties.len()==0 || &builder.parties[builder.parties.len()-1].group_id != group_id {
                builder.parties.push(GroupBuilder{name:record[5].to_string(), abbreviation:None, group_id:group_id.to_string(),ticket_id:None, tickets: vec![]});
            }
            builder.candidates.push(Candidate{
                name: record[3].to_string()+", "+&record[4],
                party: Some(PartyIndex(builder.parties.len()-1)),
                position: Some(position_in_ticket),
                ec_id: None,
            });
        }
    }
    Ok(())
}



struct ParsedRawVoteIterator<'a> {
    electorate_column : usize,
    collection_column : usize,
    preferences_column : Option<usize>,
    num_atl_plus_num_btl_hint : usize,
    // reader : Reader<ZipFile<'a>>,
    records : StringRecordsIntoIter<ZipFile<'a>>
}


impl<'a> ParsedRawVoteIterator<'a> {
    /// the num_atl_plus_num_btl_hint is used for initial capacity of the vector - it only matters for performance, and if it is a few over that is fine,
    fn new(zipfile : &'a mut ZipArchive<File>,num_atl_plus_num_btl_hint:usize) -> anyhow::Result<Self> {
        let zip_contents = zipfile.by_index(0)?;
        let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(zip_contents);
        let headings = reader.headers()?;
        let electorate_column = if &headings[0]=="ElectorateNm" {0} else if &headings[1]=="Division" {1} else { return Err(anyhow!("Could not find a division heading"))};
        let collection_column = if &headings[1]=="VoteCollectionPointNm" {1} else if &headings[2]=="Vote Collection Point Name" {2} else {return Err(anyhow!("Could not find a collection point heading"))};
        let preferences_column = if &headings[5]=="Preferences" {Some(5)} else {None};
        let records = reader.into_records();
        Ok(ParsedRawVoteIterator {
            electorate_column,
            collection_column,
            preferences_column,
            num_atl_plus_num_btl_hint,
            records,
        })
    }
}

pub struct ParsedRawVote {
    pub markings : Vec<RawBallotMarking>,
    electorate_column : usize,
    collection_column : usize,
    record : StringRecord,
}

impl ParsedRawVote {
    pub fn metadata(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("Electorate".to_string(),self.record[self.electorate_column].to_string());
        map.insert("Collection Point".to_string(),self.record[self.collection_column].to_string());
        map
    }
}

impl <'a> Iterator for ParsedRawVoteIterator<'a> {
    type Item = Result<ParsedRawVote,csv::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.records.next() {
            Some(Ok(record)) => {
                if record[0].starts_with("---") { return self.next(); } // skip dummy heading "underlines" if there.
                let mut markings : Vec<RawBallotMarking> = Vec::with_capacity(self.num_atl_plus_num_btl_hint);
                match self.preferences_column {
                    Some(preferences_column) => { // preferences are all in 1 column, comma separated
                        for s in record[preferences_column].split(',') {
                            markings.push(parse_marking(s));
                        }
                    }
                    None => {
                        for i in 6..record.len() {
                            markings.push(parse_marking(&record[i]));
                        }
                    }
                }
                Some(Ok(ParsedRawVote{
                    markings,
                    electorate_column: self.electorate_column,
                    collection_column: self.collection_column,
                    record
                }))
            }
            None => None,
            Some(Err(e)) => Some(Err(e)),
        }
    }
}
