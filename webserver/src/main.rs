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
use statistics::mean_preference::MeanPreferences;
use stv::ballot_metadata::ElectionMetadata;
use crate::cache::cache_json;
use crate::find_election::{ALL_ELECTIONS_AS_LIST, ElectionInfo, ElectionsOfOneType, FoundElection};


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



#[post("/find_my_vote")]
async fn find_my_vote() -> Json<Result<String,String>> {
    let contests : anyhow::Result<String> = Ok("Federal".to_string());
    Json(contests.map_err(|e|e.to_string()))
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

#[actix_rt::main]
async fn main() -> anyhow::Result<()> {
    // check whether everything is working before starting the web server. Don't want to find out in the middle of a transaction.
    println!("Running webserver on http://localhost:8999 stop with control C.");
    HttpServer::new(move|| {
        actix_web::App::new()
            .service(get_all_contests)
            .service(get_metadata)
            .service(get_info)
            .service(get_mean_preferences)
            .service(find_my_vote)
            .service(get_data)
            .wrap(middleware::Compress::default())
            .service(actix_files::Files::new("/{a}/{b}/{c}/", find_web_resources().join("ContestDirectory")).use_last_modified(true).use_etag(true).index_file("index.html"))
            .service(actix_files::Files::new("/", find_web_resources().join("RootDirectory")).use_last_modified(true).use_etag(true).index_file("index.html"))
    })
        .bind("0.0.0.0:8999")?
        .run()
        .await?;
    Ok(())
}