//! Module containing common functions and structs
//! used in handling user requests

use crate::db_control;
use crate::html_proc;
use crate::BoardConfig;
use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use serde::Deserialize;
use std::fmt;
use std::path::Path;
use std::ops::Not;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod board;
pub mod catalog;
pub mod dashboard;
pub mod error;
pub mod index;
pub mod report;
pub mod topic;
pub mod disambiguation;

/// Multipart form template for sending messages with file attachments
#[derive(MultipartForm)]
pub struct MsgForm {
    message: Text<String>,
    author: Text<String>,
    sage: Option<Text<String>>, // what has my life come to
    #[multipart(limit = "50 MiB", rename = "files[]")]
    files: Vec<TempFile>,
}

/// Information about board URL
#[derive(Deserialize)]
pub struct PathInfo {
    board: String,
    message_num: Option<i64>,
}

/// Options for query parameters in board URLs
#[derive(Deserialize)]
pub struct QueryOptions {
    page: Option<i64>,
    search_string: Option<String>,
}

impl fmt::Display for QueryOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut reassembled_query = String::new();
        if let Some(page_num) = self.page {
            reassembled_query.push_str(&format!("page={}&", page_num))
        }
        if let Some(search_string) = &self.search_string {
            reassembled_query.push_str(&format!("search_string={}&", search_string))
        }
        write!(f, "?{}", reassembled_query)
    }
}

impl QueryOptions {
    fn get_neighbour_pages(&self) -> (QueryOptions, QueryOptions) {
        let current_page = match self.page {
            Some(s) if s > 0 => s,
            Some(_) => 1,
            None => 1,
        };
        (
            QueryOptions {
                page: Some(current_page - 1),
                search_string: self.search_string.clone(),
            },
            QueryOptions {
                page: Some(current_page + 1),
                search_string: self.search_string.clone(),
            },
        )
    }
}

/// File categories that can be sent by users
#[derive(PartialEq)]
pub enum FileType {
    Image,
    Video,
    Invalid,
}

impl Not for FileType {
    type Output = bool;

    fn not(self) -> Self::Output {
        matches!(self, FileType::Invalid)
    }
}

/// Container for essential parts of the web app, such as a database client and config file
pub struct ApplicationState<'a> {
    pub db_client: Arc<Mutex<db_control::DatabaseWrapper>>,
    pub formatter: Arc<html_proc::HtmlFormatter<'a>>,
    pub config: Arc<BoardConfig>,
}

/// Function for checking a string for banned words
pub async fn contains_banned_words(checked: &str) -> bool {
    // maybe i ought to replace some of those with include_str! TODO
    // you know, to avoid opening and reading the file each time a message is sent
    let check_lower = checked.to_lowercase();
    let raw_banlist = std::fs::read_to_string("./data/banlist.yaml")
        .unwrap_or_else(|_| panic!("Can't read ./data/banlist.yaml. Is it there?"));
    let banlist: Vec<String> = serde_yaml::from_str(&raw_banlist).unwrap();
    for word in banlist {
        if check_lower.contains(&word) {
            return true;
        }
    }
    false
}

/// Validates images sent by users using libmagic
pub fn valid_file(image: &str) -> FileType {
    if image.is_empty() {
        return FileType::Invalid;
    }

    let mut image_fs_path = image.to_string();
    image_fs_path = image_fs_path[..image_fs_path.len()].to_string(); // path ends with "\" for some reason

    // libmagic image validation
    let cookie = magic::Cookie::open(magic::cookie::Flags::ERROR).unwrap();
    let database = Default::default();
    let cookie = cookie.load(&database).unwrap();
    let file_type = cookie.file(&image_fs_path);

    if let Ok(raw_type) = file_type {
        if Path::new(&image_fs_path).exists() {
            if raw_type.contains("image data") {
                return FileType::Image;
            } else if raw_type.contains("MP4 Base Media") || raw_type.contains("WebM") {
                return FileType::Video;
            }
        }
    }

    FileType::Invalid
}


/// Handler for files in multipart forms
pub async fn process_files(files: &Vec<TempFile>) -> String {
    let mut filepath_collection = String::from("");
    for (i, item) in files.iter().enumerate() {
        // only process the first 4 files, delete the rest
        let f = &item;
        let temp_file_path = f.file.path();
        if i > 4 {
            let remove_excess_status = std::fs::remove_file(temp_file_path);
            if remove_excess_status.is_err() {
                log::error!(
                    "Failed to delete unwanted (excess) file: {}",
                    temp_file_path.display()
                );
            }
            continue;
        }
        // test to see if it is an actual image/video
        if !valid_file(temp_file_path.to_str().unwrap()) {
            continue;
        }
        let orig_name = f
            .file_name
            .as_ref()
            .expect("no file name")
            .split('.')
            .collect::<Vec<&str>>();
        let new_name = rand::random::<u64>().to_string();
        let new_filepath = PathBuf::from(format!("data/user_images/{}.{}", new_name, orig_name[1]));
        let copy_status = std::fs::copy(temp_file_path, new_filepath.clone());
        let remove_status = std::fs::remove_file(temp_file_path);

        if copy_status.is_err() {
            log::error!(
                "Failed to move file {} to {}",
                temp_file_path.display(),
                &new_filepath.display()
            );
        }
        if remove_status.is_err() {
            log::error!("Failed to delete file: {}", temp_file_path.display());
        }

        filepath_collection.push_str(new_filepath.to_str().unwrap());
        filepath_collection.push(';');
    }
    filepath_collection
}
