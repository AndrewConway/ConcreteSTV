// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

//! Cache to disk JSON responses to web calls




use std::fmt;
use std::future::Future;
use actix_web::web::Json;
use serde::{Serialize,Deserialize};
use serde::de::DeserializeOwned;

const CACHE_DIR: &str = "./Cache/WebJSON";

/*
/// Return f(), or a previously cached copy of it, stored with a key based on name and the Json serialization of args.
/// NOTE if called while still evaluating f(), could evaluate f() multiple times.
pub async fn cache<F,R,A>(name:&str,args:&A,f:F) -> Result<R,String>
    where F:FnOnce()->anyhow::Result<R>,
          R:Serialize+DeserializeOwned,
          A:Serialize
{
    let key = name.to_string()+&serde_json::to_string(args).map_err(|_e|"Internal error - could not serialize arguments".to_string())?;

    match cacache::read(CACHE_DIR,&key).await {
        Ok(bytes) => {
            let deserialized : R = serde_json::from_slice(&bytes).map_err(|_e|"Internal error - could not deserialize cached value".to_string())?;
            Ok(deserialized)
        }
        Err(_) => {
            let computed = f().map_err(|e|e.to_string())?;
            let serialized = serde_json::to_vec(&computed).map_err(|_e|"Internal error - could not serialize cached value".to_string())?;
            cacache::write(CACHE_DIR,&key,&serialized).await.map_err(|_e|"Internal error - could not cache value".to_string())?;
            Ok(computed)
        }
    }
}*/

/// Return f(), or a previously cached copy of it, stored with a key based on name and the Json serialization of args.
/// NOTE if called while still evaluating f(), could evaluate f() multiple times. So not ideal, but only a waste of performance in a special case.
pub async fn cache_async<F,R,A,Fut,E>(name:&str,args:&A,f:F) -> Result<R,String>
    where F:FnOnce()-> Fut,
          Fut: Future<Output=Result<R,E>>,
          E : ToString,
          R:Serialize+DeserializeOwned,
          A:Serialize
{
    let key = name.to_string()+&serde_json::to_string(args).map_err(|_e|"Internal error - could not serialize arguments".to_string())?;

    match cacache::read(CACHE_DIR,&key).await {
        Ok(bytes) => {
            let deserialized : R = serde_json::from_slice(&bytes).map_err(|e|{
                println!("Internal Error {}. Failed to deserialize {}",e,String::from_utf8_lossy(&bytes));
                // let try2 : Result<TranscriptWithMetadata<isize>,_> = serde_json::from_slice(&bytes);
                // if let Err(err) = try2 {
                //     println!("try2 error : {}",err);
                // }
                format!("Internal error - could not deserialize cached value ({})",e)
            })?;
            Ok(deserialized)
        }
        Err(_) => {
            let computed = f().await.map_err(|e|e.to_string())?;
            let serialized = serde_json::to_vec(&computed).map_err(|_e|"Internal error - could not serialize cached value".to_string())?;
            cacache::write(CACHE_DIR,&key,&serialized).await.map_err(|_e|"Internal error - could not cache value".to_string())?;
            Ok(computed)
        }
    }
}
/*
pub async fn cache_json_sync<F,R,A>(name:&str,args:&A,f:F) -> Json<Result<R,String>>
    where F:FnOnce()->anyhow::Result<R>,
          R:Serialize+DeserializeOwned,
          A:Serialize
{
    Json(cache(name,args,f).await)
}*/

pub async fn cache_json<F,R,A,Fut,E>(name:&str,args:&A,f:F) -> Json<Result<R,String>>
    where F:FnOnce()-> Fut,
          Fut: Future<Output=Result<R,E>>,
          E : ToString,
          R:Serialize+DeserializeOwned,
          A:Serialize
{
    Json(cache_async(name,args,f).await)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StringError(String);

impl fmt::Debug for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}
impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
}
impl std::error::Error for StringError {}

impl From<String> for StringError {
    fn from(v: String) -> Self { StringError(v) }
}
