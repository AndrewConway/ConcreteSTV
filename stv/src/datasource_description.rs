// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.


//! A description of elections that are known about.



use std::borrow::Cow;
use crate::parse_util::{FileFinder, RawDataSource};
use serde::{Deserialize,Serialize};

pub trait ElectionDataSource {
    // the name of the election. E.g. "Federal Senate"
    fn name(&self) -> Cow<'static, str> ;
    /// the name of the electoral commission that administers it.
    fn ec_name(&self) -> Cow<'static, str> ;
    /// the url of the electoral commission that administers it.
    fn ec_url(&self) -> Cow<'static, str> ;
    /// the years that it works for
    fn years(&self) -> Vec<String>;
    /// something that will load data given a year from the above list.
    fn get_loader_for_year(&self,year:&str,finder:&FileFinder) -> anyhow::Result<Box<dyn RawDataSource+Send+Sync>>;
}

/// Description of the copyright of the electoral data used.
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Copyright {
    /// The statement they would like users to make
    pub statement : Option<Cow<'static, str>>,
    /// A page describing the copyright.
    pub url : Option<Cow<'static, str>>,
    /// The license name, if any.
    pub license_name : Option<Cow<'static, str>>,
    /// A url to the details of the license
    pub license_url : Option<Cow<'static, str>>,
}

/// The counting rules that should apply to a set of elections.
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct AssociatedRules {
    /// The rules that were actually used.
    pub rules_used : Option<Cow<'static, str>>,
    /// The rules I recommend using.
    pub rules_recommended : Option<Cow<'static, str>>,
    /// A comment, if needed.
    pub comment : Option<Cow<'static, str>>,
    /// A list of reports (urls).
    pub reports : Vec<Cow<'static, str>>,
}