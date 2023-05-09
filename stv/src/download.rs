// Copyright 2021-2023 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::path::{PathBuf, Path};
use std::fs::File;
use std::io::Read;
use std::thread::sleep;
use std::time::Duration;
use std::sync::Mutex;
use anyhow::{anyhow, Context};
use once_cell::sync::OnceCell;
use crate::parse_util::{file_to_string, MissingFile};

/// Helper for downloading a URL to a local file equivalent.
/// Tries to make this an obvious mirror of the URL.
/// This is not possible for URLs ending in / (or the root dir); these have index.html added on.
pub struct CacheDir {
    base : PathBuf,
}

pub trait Downloader {
    // true return means file downloaded. False return means file not available yet but put in a queue to be downloaded some other time.
    fn download(file:&PathBuf,url:&str) -> anyhow::Result<bool>;
}

pub struct DownloadWithReqwest {}

impl Downloader for DownloadWithReqwest {
    fn download(file: &PathBuf, url: &str) -> anyhow::Result<bool> {
        println!("Downloading {} with reqwest",url);
        let contents = reqwest::blocking::get(url)?.bytes()?; // Client::builder().build()?.get(url).send()
        std::fs::write(&file,contents)?;
        Ok(true)
    }
}

impl CacheDir {
    fn rate_limit() {
        static DOWNLOAD_RATE_LIMIT_MUTEX: OnceCell<Mutex<()>> = OnceCell::new();
        let _lock = DOWNLOAD_RATE_LIMIT_MUTEX.get_or_init(||Mutex::new(())).lock().unwrap();
        sleep(Duration::from_millis(1000));
    }
    pub fn new<P:AsRef<Path>>(path:P) -> Self {
        let path : &Path = path.as_ref();
        CacheDir{ base: path.to_path_buf() }
    }

    /// Get where a file representing said path should be stored.
    pub fn file(&self,url_path:&str) -> PathBuf {
        let res = self.base.join(url_path);
        if url_path.chars().last().map(std::path::is_separator).unwrap_or(true) { res.join("index.html")} else {res}
    }

    /// Download a url using Reqwest, and store.
    /// If Ok(None) is returned, then this is not available now but may be later.
    pub fn get_or_download<D:Downloader>(&self,url:&str) -> anyhow::Result<Option<File>> {
        self.get_or_download_with_file_suffix::<D>(url,None)
    }
    /// Get the path a file should be stored to.
    pub fn get_file_path_with_extension(&self,url:&str,suffix:Option<&str>) -> PathBuf {
        let url_path = url.trim_start_matches("https://").trim_start_matches("http://").replace("%20"," ");
        let already_has_suffix = url_path.split('/').last().map(|s|s.contains('.')).unwrap_or(false);
        let url_path_with_extension : String = if already_has_suffix { url_path } else if let Some(suffix) = suffix { url_path+suffix } else { url_path };
        self.file(&url_path_with_extension)
    }
    /// Find the path to an existing file, or useful error if it doesn't exist.
    /// Don't try to download.
    pub fn find_raw_data_file_from_cache(&self,url:&str) -> Result<PathBuf,MissingFile> {
        let path = self.get_file_path_with_extension(url,None);
        if path.exists() { Ok(path) } else { Err(MissingFile{file_name:path.to_string_lossy().to_string(),where_to_get:url.to_string(),where_to_get_is_exact_url:true })}
    }
    /// Download a url using Reqwest, and store. Add a suffix before storing, if suffix is not None, unless the file already has a suffix.
    /// If Ok(None) is returned, then this is not available now but may be later.
    pub fn get_or_download_with_file_suffix<D:Downloader>(&self,url:&str,suffix:Option<&str>) -> anyhow::Result<Option<File>> {
        let file = self.get_file_path_with_extension(&url.replace("%20"," "),suffix);
        //if file_without_extension.exists() { std::fs::rename(&file_without_extension,&file)?; } // update already downloaded files
        match File::open(&file) {
            Ok(f) => Ok(Some(f)),
            Err(_) => {
                // need to download it,
                if let Some(p) = file.parent() {
                    std::fs::create_dir_all(p)?;
                }
                Self::rate_limit();
                if D::download(&file,url)? {
                    Ok(Some(File::open(&file)?))
                } else { Ok(None)}
            }
        }
    }

    /// Download a url using Reqwest, and return as a string.
    /// If Ok(None) is returned, then this is not available now but may be later.
    pub fn get_or_download_string<D:Downloader>(&self,url:&str) -> anyhow::Result<Option<String>> {
        if let Some(mut file) = self.get_or_download::<D>(url)? {
            Ok(Some(file_to_string(&mut file)?))
        } else {
            Ok(None)
        }
    }

    /// Get a previously cached URL as a file. Like get_or_download, except does not download.
    pub fn get_file(&self,url:&str) -> anyhow::Result<File> {
        let path = self.get_file_path_with_extension(url,None);
        File::open(&path).with_context(||format!("Looking for file at {} containing contents of {}",path.to_string_lossy(),url))
    }

    /// Get a previously cached URL as a string. Like get_or_download_string, except does not download.
    pub fn get_string(&self,url:&str) -> anyhow::Result<String> {
        file_to_string(&mut self.get_file(url)?)
    }

    /// Download a url using Reqwest, and return as a string.
    /// If Ok(None) is returned, then this is not available now but may be later.
    pub fn get_or_download_byte_array<D:Downloader>(&self,url:&str) -> anyhow::Result<Option<Vec<u8>>> {
        if let Some(mut file) = self.get_or_download::<D>(url)? {
            let mut bytes = Vec::new();
            file.read_to_end( &mut bytes)?;
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }

    /// Get a previously cached URL as a string. Like get_or_download_string, except does not download.
    pub fn get_byte_array(&self, url:&str) -> std::io::Result<Vec<u8>> {
        let path = self.get_file_path_with_extension(url,None);
        std::fs::read(path)
    }


    /// like get_or_download_string but windows_1252 encoding not utf-8
    pub fn get_or_download_string_windows_1252<D:Downloader>(&self,url:&str) -> anyhow::Result<Option<String>> {
        if let Some(bytes) = self.get_or_download_byte_array::<D>(url)? {
            let (cow,_,had_errors) = encoding_rs::WINDOWS_1252.decode(&bytes);
            if had_errors { return Err(anyhow!("Had errors decoding")) }
            Ok(Some(cow.to_string()))
        } else { Ok(None) }
    }

    /// Get a previously cached URL as a string. Like get_string, except windows_1252 encoding not utf-8
    pub fn get_string_windows_1252(&self,url:&str) -> anyhow::Result<String> {
        let bytes = self.get_byte_array(url)?;
        let (cow,_,had_errors) = encoding_rs::WINDOWS_1252.decode(&bytes);
        if had_errors { return Err(anyhow!("Had errors decoding")) }
        Ok(cow.to_string())
    }
}

