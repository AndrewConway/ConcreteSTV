// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

mod find_election;
mod cache;

use std::path::PathBuf;
use actix_files::NamedFile;
use actix_web::{HttpServer, middleware, web};
use actix_web::web::Json;
use actix_web::{get, post};
use actix_web::http::header::{ContentDisposition, DispositionParam, DispositionType};
use main_app::rules::{PossibleTranscripts, Rules, RulesDetails};
use statistics::correlations::{CorrelationDendrogramsAndSVD, CorrelationOptions, SquareMatrix};
use stv::find_vote::{FindMyVoteQuery, FindMyVoteResult};
use statistics::intent_table::{IntentTable, IntentTableOptions};
use statistics::mean_preference::MeanPreferences;
use statistics::who_got_votes::WhoGotVotes;
use stv::ballot_metadata::{CandidateIndex, ElectionMetadata, NumberOfCandidates};
use stv::errors_btl::ObviousErrorsInBTLVotes;
use stv::tie_resolution::TieResolutionsMadeByEC;
use crate::cache::cache_json;
use crate::find_election::{ALL_ELECTIONS_AS_LIST, ElectionInfo, ElectionsOfOneType, FoundElection};
use serde::{Serialize,Deserialize};

#[get("/get_all_contests.json")]
async fn get_all_contests() -> Json<Result<Vec<ElectionsOfOneType>,String>> {
    let contests : &anyhow::Result<Vec<ElectionsOfOneType>> = &ALL_ELECTIONS_AS_LIST;
    Json(match contests {
        Ok(res) => Ok(res.clone()),
        Err(e) => Err(e.to_string()),
    })
}

#[get("/{name}/{year}/{electorate}/metadata.json")]
async fn get_metadata(election : web::Path<FoundElection>) -> Json<Result<ElectionMetadata,String>> {
    cache_json("metadata.json",&election.spec,||election.metadata()).await
}

#[get("/{name}/{year}/{electorate}/info.json")]
async fn get_info(election : web::Path<FoundElection>) -> Json<Result<ElectionInfo,String>> {
    cache_json("simple.json",&election.spec,||election.get_info()).await
}

#[get("/{name}/{year}/{electorate}/MeanPreferences.json")]
async fn get_mean_preferences(election : web::Path<FoundElection>) -> Json<Result<MeanPreferences,String>> {
    async fn get_mean_preferences_uncached(election : &web::Path<FoundElection>) -> Result<MeanPreferences,String> {
        Ok(MeanPreferences::compute(&election.data().await?))
    }
    cache_json("MeanVotes.json",&election.spec,||get_mean_preferences_uncached(&election)).await
}


#[get("/{name}/{year}/{electorate}/IntentTable.json")]
async fn get_intent_table(election : web::Path<FoundElection>,options : web::Query<IntentTableOptions>) -> Json<Result<IntentTable,String>> {
    async fn get_intent_table_uncached(election : &web::Path<FoundElection>,options : &web::Query<IntentTableOptions>) -> Result<IntentTable,String> {
        Ok(IntentTable::compute(&election.data().await?,options))
    }
    cache_json("IntentTable.json",&(election.spec.clone(),options.0.clone()),||get_intent_table_uncached(&election,&options)).await
}

#[get("/{name}/{year}/{electorate}/Correlation.json")]
async fn get_correlation(election : web::Path<FoundElection>,options : web::Query<CorrelationOptions>) -> Json<Result<CorrelationDendrogramsAndSVD,String>> {
    async fn get_correlation_uncached(election : &web::Path<FoundElection>,options : &web::Query<CorrelationOptions>) -> Result<CorrelationDendrogramsAndSVD,String> {
        let correlation = SquareMatrix::compute_correlation_matrix(&election.data().await?,&options).to_distance_matrix();
        Ok(CorrelationDendrogramsAndSVD::new(correlation)?)
    }
    cache_json("Correlation.json",&(election.spec.clone(),options.0.clone()),||get_correlation_uncached(&election,&options)).await
}

#[get("/{name}/{year}/{electorate}/WhoGotVotes.json")]
async fn get_who_got_votes(election : web::Path<FoundElection>) -> Json<Result<WhoGotVotes,String>> {
    async fn get_who_got_votes_uncached(election : &web::Path<FoundElection>) -> Result<WhoGotVotes,String> {
        Ok(WhoGotVotes::compute(&election.data().await?))
    }
    cache_json("WhoGotVotes.json",&election.spec,||get_who_got_votes_uncached(&election)).await
}

#[get("/{name}/{year}/{electorate}/RepeatedNumbers.json")]
async fn get_find_btl_errors(election : web::Path<FoundElection>) -> Json<Result<ObviousErrorsInBTLVotes,String>> {
    async fn get_find_btl_errors_uncached(election : &web::Path<FoundElection>) -> Result<ObviousErrorsInBTLVotes,String> {
        election.loader.find_btl_errors(election.electorate()).map_err(|e|e.to_string())
    }
    cache_json("WhoGotVotes.json",&election.spec,||get_find_btl_errors_uncached(&election)).await
}



#[post("/{name}/{year}/{electorate}/find_my_vote")]
async fn find_my_vote(election : web::Path<FoundElection>,query:web::Json<FindMyVoteQuery>) -> Json<Result<FindMyVoteResult,String>> {
    Json(election.loader.find_my_vote(election.electorate(),&query).map_err(|e|e.to_string()))
}

#[get("/{name}/{year}/{electorate}/data.stv")]
async fn get_data(election : web::Path<FoundElection>) -> std::io::Result<NamedFile> {
    let cached_path = election.data().await.map_err(|e|std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?.metadata.name.cache_file_name();
    let file = NamedFile::open(cached_path)?;
    let filename = election.spec.name.to_string()+" "+&election.spec.year+" "+&election.spec.electorate+".stv";
    Ok(file
        .use_last_modified(true)
        .set_content_disposition(ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![DispositionParam::Filename(filename)],
        }))
}

#[get("/rules.json")]
async fn get_rules() -> Json<Vec<RulesDetails>> {
    Json(RulesDetails::list())
}

#[derive(Serialize,Deserialize,Clone)]
pub struct RecountQuery {
    /// Candidates who are usually excluded, e.g. if they died on the election day or were ruled ineligible to stand. Looking at you 2016.
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub excluded : Vec<CandidateIndex>,
    pub candidates_to_be_elected : NumberOfCandidates,
    #[serde(flatten)]
    pub tie_resolutions : TieResolutionsMadeByEC,
    pub rules : Rules,
    /// if none, use all votes. Otherwise only use ones specified in here. "" means votes not assigned a type.
    pub vote_types : Option<Vec<String>>,
}

#[post("/{name}/{year}/{electorate}/recount")]
async fn recount(election : web::Path<FoundElection>,query:web::Json<RecountQuery>) -> Json<Result<PossibleTranscripts,String>> {
    async fn recount_uncached(election : &web::Path<FoundElection>,query:&RecountQuery) -> Result<PossibleTranscripts,String> {
        let vote_types : Option<&[String]> = if let Some(vt) = &query.vote_types { Some(vt) } else { None };
        Ok(query.rules.count(&election.data().await?,query.candidates_to_be_elected,&query.excluded.iter().cloned().collect(),&query.tie_resolutions,vote_types,false))
    }
    cache_json("recount",&(election.spec.clone(),query.clone()),||recount_uncached(&election,&query)).await
}



/// find the path containing web resources, static web files that will be served.
/// This is usually in the directory `WebResources` but the program may be run from
/// other directories. To be as robust as possible it will try likely possibilities.
fn find_web_resources() -> PathBuf {
    let rel_here = std::path::Path::new(".").canonicalize().expect("Could not resolve path .");
    for p in rel_here.ancestors() {
        let pp = p.join("WebResources");
        if pp.is_dir() {return pp;}
        let pp = p.join("webserver/WebResources");
        if pp.is_dir() {return pp;}
    }
    panic!("Could not find WebResources. Please run in a directory containing it.")
}

/// find the path containing the viewer, static web files that will be served.
/// This is usually in the directory `docs` but the program may be run from
/// other directories. To be as robust as possible it will try likely possibilities.
fn find_viewer_resources() -> PathBuf {
    let rel_here = std::path::Path::new(".").canonicalize().expect("Could not resolve path .");
    for p in rel_here.ancestors() {
        let pp = p.join("docs");
        if pp.is_dir() {return pp;}
    }
    panic!("Could not find docs. Please run in a directory containing it.")
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // check whether everything is working before starting the web server. Don't want to find out in the middle of a transaction.
    println!("Running webserver on http://localhost:8999 stop with control C.");
    HttpServer::new(move|| {
        actix_web::App::new()
            .wrap(middleware::Compress::default())
            .service(get_all_contests)
            .service(get_metadata)
            .service(get_info)
            .service(get_mean_preferences)
            .service(get_intent_table)
            .service(get_correlation)
            .service(get_who_got_votes)
            .service(get_find_btl_errors)
            .service(find_my_vote)
            .service(get_data)
            .service(get_rules)
            .service(recount)
            .service(actix_files::Files::new("/{a}/{b}/{c}/", find_web_resources().join("ContestDirectory")).use_last_modified(true).use_etag(true).index_file("index.html"))
            .service(actix_files::Files::new("/Viewer/", find_viewer_resources()).use_last_modified(true).use_etag(true))
            .service(actix_files::Files::new("/", find_web_resources().join("RootDirectory")).use_last_modified(true).use_etag(true).index_file("index.html"))
    })
        .bind("0.0.0.0:8999")?
        .run()
        .await?;
    Ok(())
}