// Copyright 2022-2024 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.



use std::sync::Arc;
use anyhow::anyhow;
use once_cell::sync::Lazy;
use federal::parse::FederalDataSource;
use nsw::parse_lge::NSWLGEDataSource;
use stv::datasource_description::{AssociatedRules, Copyright, ElectionDataSource};
use serde::{Deserialize,Serialize};
use examples::example_datasource::ExampleDataSource;
use nsw::parse_lc::NSWLCDataSource;
use statistics::simple_statistics::SimpleStatistics;
use stv::ballot_metadata::ElectionMetadata;
use stv::election_data::ElectionData;
use stv::parse_util::{FileFinder, RawDataSource};
use stv::run_once_globally::RunOnceController;

fn get_all_elections_with_redistributable_data() -> Vec<Box<dyn ElectionDataSource+Sync+Send>>{
    vec![Box::new(FederalDataSource{}),Box::new(NSWLCDataSource{}),Box::new(NSWLGEDataSource{}),Box::new(ExampleDataSource{})]
}

static GREAT_ELECTION_LIST: Lazy<Vec<Box<dyn ElectionDataSource+Sync+Send>>> = Lazy::new(get_all_elections_with_redistributable_data);
static FILE_FINDER: Lazy<FileFinder> = Lazy::new(||FileFinder::find_ec_data_repository());

pub static ALL_ELECTIONS_AS_LIST : Lazy<anyhow::Result<Vec<ElectionsOfOneType>>> = Lazy::new(get_all_elections_as_list);

fn get_all_elections_as_list() -> anyhow::Result<Vec<ElectionsOfOneType>> {
    let mut res = vec![];
    for source in GREAT_ELECTION_LIST.iter() {
        res.push(ElectionsOfOneType::new(source,&FILE_FINDER)?);
    }
    Ok(res)
}

/// Information similar to that found in ElectionDataSource but usable outside.
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ElectionsOfOneType {
    pub name : String,
    /// the name of the electoral commission that administers it.
    pub ec_name : String,
    /// the url of the electoral commission that administers it.
    pub ec_url : String,
    /// the years that are available
    pub years : Vec<ElectionsOfOneTypeAndYear>,
}

impl ElectionsOfOneType {
    fn new(source:&Box<dyn ElectionDataSource+Sync+Send>,finder:&FileFinder) -> anyhow::Result<ElectionsOfOneType> {
        let mut years = vec![];
        for year in source.years() {
            let electorates = source.get_loader_for_year(&year,finder)?.all_electorates();
            years.push(ElectionsOfOneTypeAndYear{year,electorates})
        }
        Ok(ElectionsOfOneType{
            name: source.name().to_string(),
            ec_name: source.ec_name().to_string(),
            ec_url: source.ec_url().to_string(),
            years,
        })
    }
}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ElectionsOfOneTypeAndYear {
    pub year : String,
    pub electorates : Vec<String>,
}


#[derive(Debug,Serialize,Deserialize,Clone,Eq,PartialEq,Hash)]
pub struct TextElectionSpecification {
    pub name : String,
    pub year : String,
    pub electorate : String,
}

#[derive(Deserialize)]
#[serde(try_from="TextElectionSpecification")]
/// A structure that can be parsed from a name/year/electorate path.
pub struct FoundElection {
    pub spec : TextElectionSpecification,
    pub source : &'static Box<dyn ElectionDataSource+Sync+Send>,
    pub loader : Arc<Box<dyn RawDataSource+Sync+Send>>,
}

impl TryFrom<TextElectionSpecification> for FoundElection {
    type Error = anyhow::Error;

    fn try_from(spec: TextElectionSpecification) -> Result<Self, Self::Error> {
        if let Some(source) = GREAT_ELECTION_LIST.iter().find(|source|source.name()==spec.name.as_str()) {
            let loader = Arc::new(source.get_loader_for_year(&spec.year,&FILE_FINDER)?);
            if loader.all_electorates().contains(&spec.electorate) {
                Ok(FoundElection{spec,source,loader})
            } else {
                Err(anyhow!("Could not find electorate named {}",spec.electorate))
            }
        } else {
            Err(anyhow!("Could not find election named {}",spec.name))
        }
    }
}

// control how the election is loaded.
static RUN_ONCE_CONTROLLER: Lazy<RunOnceController<TextElectionSpecification,Result<ElectionData,String>>> = Lazy::new(||RunOnceController::default());

impl FoundElection {
    pub fn electorate(&self) -> &str { self.spec.electorate.as_str() }
    pub async fn data(& self) -> Result<ElectionData,String> {
        let loader = self.loader.clone();
        let electorate = self.electorate().to_string();
        RUN_ONCE_CONTROLLER.get(&self.spec.clone(),||async move{
            loader.load_cached_data(&electorate).map_err(|e|e.to_string())
        }).await
        // self.loader.load_cached_data(self.electorate())
    }
    pub async fn metadata(&self) -> Result<ElectionMetadata,String> {
        if self.loader.can_load_full_data(self.electorate()) { Ok(self.data().await?.metadata) } // this gets any EC decisions deduced in the full set
        else { self.loader.read_raw_metadata(self.electorate()).map_err(|e|e.to_string()) }
    }
    pub async fn get_info(&self) -> Result<ElectionInfo,String> {
        let simple = if self.loader.can_load_full_data(self.electorate()) {
            Some(SimpleStatistics::new(&self.data().await?))
        } else { None };
        Ok(ElectionInfo{
            simple,
            ec_name: self.source.ec_name().to_string(),
            ec_url: self.source.ec_url().to_string(),
            copyright: self.loader.copyright(),
            rules: self.loader.rules(self.electorate()),
            can_read_raw_markings : self.loader.can_read_raw_markings(),
        })
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ElectionInfo {
    simple : Option<SimpleStatistics>,
    /// the name of the electoral commission that administers it.
    pub ec_name : String,
    /// the url of the electoral commission that administers it.
    pub ec_url : String,
    pub copyright : Copyright,
    pub rules : AssociatedRules,
    pub can_read_raw_markings : bool,
}
