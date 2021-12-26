// Copyright 2021 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::path::{PathBuf, Path};
use std::fs::File;
use std::thread::sleep;
use std::time::Duration;
use std::sync::Mutex;
use once_cell::sync::OnceCell;
use crate::parse_util::file_to_string;

/// Helper for downloading a URL to a local file equivalent.
/// Tries to make this an obvious mirror of the URL.
/// This is not possible for URLs ending in / (or the root dir); these have index.html added on.
pub struct CacheDir {
    base : PathBuf,
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
    pub fn get_or_download(&self,url:&str) -> anyhow::Result<File> {
        self.get_or_download_with_file_suffix(url,None)
    }
    /// Get the path a file should be stored to.
    pub fn get_file_path_with_extension(&self,url:&str,suffix:Option<&str>) -> PathBuf {
        let url_path = url.trim_start_matches("https://").trim_start_matches("http://").to_string();
        let url_path_with_extension : String = if let Some(suffix) = suffix { url_path+suffix } else { url_path };
        self.file(&url_path_with_extension)
    }
    /// Download a url using Reqwest, and store. Add a suffix before storing, if suffix is not None.
    pub fn get_or_download_with_file_suffix(&self,url:&str,suffix:Option<&str>) -> anyhow::Result<File> {
        let file = self.get_file_path_with_extension(&url,suffix);
        //if file_without_extension.exists() { std::fs::rename(&file_without_extension,&file)?; } // update already downloaded files
        match File::open(&file) {
            Ok(f) => Ok(f),
            Err(_) => {
                // need to download it,
                println!("Downloading {} with reqwest",url);
                Self::rate_limit();
                let contents = reqwest::blocking::get(url)?.bytes()?;
                if let Some(p) = file.parent() {
                    std::fs::create_dir_all(p)?;
                }
                std::fs::write(&file,contents)?;
                Ok(File::open(&file)?)
            }
        }
    }

    /// Download a url using Reqwest, and return as a string.
    pub fn get_or_download_string(&self,url:&str) -> anyhow::Result<String> {
        let mut file = self.get_or_download(url)?;
        file_to_string(&mut file)
    }
}

