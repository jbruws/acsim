//! Module containing functions responsible for actual
//! handling of HTTP requests

use crate::db_control;
use crate::html_proc;
use crate::BoardConfig;
use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use serde::Deserialize;
use std::fmt;
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

/// Form used to send messages and images
#[derive(MultipartForm)]
pub struct MsgForm {
    message: Text<String>,
    author: Text<String>,
    sage: Option<Text<String>>, // what has my life come to
    #[multipart(limit = "50 MiB", rename = "files[]")]
    files: Vec<TempFile>,
}

/// Information about URL path
#[derive(Deserialize)]
pub struct PathInfo {
    board: String,
    message_num: Option<i64>,
}

/// Options that can be specified in query strings in URL
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

/// Struct containing various components of the application
pub struct ApplicationState<'a> {
    pub db_client: Arc<Mutex<db_control::DatabaseWrapper>>,
    pub formatter: Arc<html_proc::HtmlFormatter<'a>>,
    pub config: Arc<BoardConfig>,
}

/// Function for handling files in multipart forms
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
        if !html_proc::valid_file(temp_file_path.to_str().unwrap()) {
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
