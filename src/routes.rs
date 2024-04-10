//! Module containing common functions and structs
//! used in handling user requests

use crate::db_control;
use crate::html_proc;
use crate::BoardConfig;
use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use serde::Deserialize;
use std::fmt;
use std::ops::Not;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod board;
pub mod catalog;
pub mod dashboard;
pub mod disambiguation;
pub mod error;
pub mod index;
pub mod report;
pub mod topic;

/// Multipart form template for sending messages with file attachments
#[derive(MultipartForm)]
pub struct MsgForm {
    message: Text<String>,
    author: Text<String>,
    sage: Option<Text<String>>, // what has my life come to
    #[multipart(limit = "50 MiB", rename = "files[]")]
    files: Vec<TempFile>,
    captcha_answer: Text<String>,
    captcha_hash: Text<String>,
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

/// Creates a captcha image, saves it to /tmp and returns the characters it contains
pub async fn create_new_captcha() -> String {
    let mut captcha = captcha::Captcha::new();
    captcha
        .add_chars(6)
        .apply_filter(captcha::filters::Noise::new(f32::max(
            rand::random::<f32>() * 0.5,
            0.3,
        )))
        .apply_filter(captcha::filters::Wave::new(
            f64::max(rand::random::<f64>() * 4.0, 2.0),
            f64::max(rand::random::<f64>() * 7.0, 4.0),
        ))
        .apply_filter(captcha::filters::Dots::new(15).min_radius(3).max_radius(10))
        .view(300, 150);
    let captcha_contents = captcha.chars_as_string();

    let captcha_save_path = format!(
        "./data/captcha/ACSIM_CAPTCHA_{}.png",
        sha256::digest(&captcha_contents)
    );
    let save_result = captcha.save(std::path::Path::new(&captcha_save_path));
    if save_result.is_err() {
        log::error!("Failed to save captcha: {}", &captcha_save_path);
    }
    captcha_contents
}

/// Deletes a used captcha
pub async fn delete_captcha_image(captcha_value: String) -> std::io::Result<()> {
    let path = format!(
        "./data/captcha/ACSIM_CAPTCHA_{}.png",
        sha256::digest(captcha_value)
    );
    std::fs::remove_file(path)
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
