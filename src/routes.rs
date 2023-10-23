//! Module containing functions responsible for actual
//! handling of HTTP requests

// std
use std::path::PathBuf;
use std::sync::Arc;
// actix and serde
use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use actix_web::{get, post, web, HttpResponse, Responder};
use serde::Deserialize;
// misc
use rand::prelude::SliceRandom;
use tokio::sync::Mutex;
// crate
use crate::db_control;
use crate::html_proc;
use crate::BoardConfig;

/// Form used to send messages and images
#[derive(MultipartForm)]
struct MsgForm {
    message: Text<String>,
    author: Text<String>,
    #[multipart(limit = "128 MiB", rename = "files[]")]
    files: Vec<TempFile>,
}

/// Information about URL path
#[derive(Deserialize)]
struct PathInfo {
    board: String,
    message_num: Option<i64>,
}

/// Options that can be specified in query strings in URL
#[derive(Deserialize)]
struct QueryOptions {
    page: Option<i64>,
}

/// Struct containing various components of the application
pub struct ApplicationState<'a> {
    pub db_client: Arc<Mutex<db_control::DatabaseWrapper>>,
    pub formatter: Arc<html_proc::HtmlFormatter<'a>>,
    pub config: Arc<BoardConfig>,
}

/// Function for handling files in multipart forms
async fn process_files(files: &Vec<TempFile>) -> String {
    let mut filepath_collection = String::new();
    for (i, item) in files.iter().enumerate() {
        if i == 4 {
            break;
        }
        let f = &item;
        let temp_file_path = f.file.path();
        // test to see if it is an actual image
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
        let new_filepath = PathBuf::from(format!("user_images/{}.{}", new_name, orig_name[1]));
        let _copy_status = std::fs::copy(temp_file_path, new_filepath.clone());
        let _remove_status = std::fs::remove_file(temp_file_path);
        filepath_collection.push_str(new_filepath.to_str().unwrap());
        filepath_collection.push_str(";");
    }
    filepath_collection
}

/// Responder for site root (redirects to /b/ by default)
#[get("/")]
async fn root() -> impl Responder {
    web::Redirect::to("/b").see_other()
}

/// Responder for boards
#[get("/{board}")]
async fn board(
    data: web::Data<ApplicationState<'_>>,
    info: web::Path<PathInfo>,
    page_data: web::Query<QueryOptions>,
) -> impl Responder {
    if !data.config.boards.contains_key(&info.board) {
        return HttpResponse::Ok().body("Does not exist");
    }
    let client = data.db_client.lock().await;
    let mut inserted_msg = String::from("");

    let current_page = page_data.page.unwrap_or(1);

    // Restoring messages from DB
    for row in client
        .get_messages(
            &info.board,
            (current_page - 1) * data.config.page_limit as i64,
            data.config.page_limit as i64,
        )
        .await
        .unwrap()
        .into_iter()
    {
        inserted_msg.push_str(
            data.formatter
                .format_into_message(
                    html_proc::BoardMessageType::Message,
                    &info.board,
                    &row.get::<usize, i64>(0),        // message id
                    &html_proc::get_time(row.get(1)), // time of creation
                    &current_page.to_string(),
                    &row.get::<usize, String>(2), // author
                    &data
                        .formatter
                        .create_formatting(&row.get::<usize, String>(3))
                        .await, // message contents
                    &row.get::<usize, String>(4), // associated image
                )
                .await
                .as_str(),
        );
    }

    let mut board_links = String::new();
    for c in data.config.boards.keys() {
        board_links.push_str(&format!("<a href=\"/{}\">/{}/</a>\n ", c, c));
    }

    HttpResponse::Ok().body(
        data.formatter
            .format_into_board(
                &data.config.site_name,
                &info.board.to_string(),
                data.config
                    .boards
                    .get(&info.board)
                    .unwrap_or(&String::from("")),
                data.config
                    .taglines
                    .choose(&mut rand::thread_rng())
                    .unwrap(),
                &board_links,
                &inserted_msg,
                current_page,
            )
            .await,
    )
}

/// Message handling logic for boards
#[post("/{board}")]
async fn board_process_form(
    form: MultipartForm<MsgForm>,
    info: web::Path<PathInfo>,
    data: web::Data<ApplicationState<'_>>,
) -> impl Responder {
    if !data.config.boards.contains_key(&info.board) {
        return web::Redirect::to("/").see_other();
    }

    let client = data.db_client.lock().await;
    let filepath_collection = process_files(&form.files).await;

    // getting time
    let since_epoch = html_proc::since_epoch();

    let trimmed_author = form.author.trim();
    let trimmed_message = form.message.trim();

    // if fits, push new message into DB and vector
    if trimmed_author.len() < 254 && 0 != trimmed_message.len() && trimmed_message.len() < 4094 {
        let filtered_author = match trimmed_author.len() {
            0 => "Anonymous".to_string(),
            _ => data.formatter.filter_tags(&trimmed_author).await,
        };
        let filtered_msg = data.formatter.filter_tags(&trimmed_message).await;

        client
            .insert_to_messages(
                since_epoch,
                &filtered_author,
                &filtered_msg,
                &filepath_collection,
                since_epoch,
                &info.board,
            )
            .await;

        // after sending, get number of messages on the board
        let msg_count = client.count_messages(&info.board).await.unwrap();

        // delete a message if total message number is over the hard limit
        if msg_count > data.config.hard_limit.into() {
            client.delete_least_active(&info.board).await;
        }
    }

    web::Redirect::to(format!("/{}", info.board)).see_other()
}

/// Responder for individual topics/threads
#[get("{board}/topic/{message_num}")]
async fn topic(
    data: web::Data<ApplicationState<'_>>,
    info: web::Path<PathInfo>,
    page_data: web::Query<QueryOptions>,
) -> impl Responder {
    let message_num = info.message_num.unwrap_or(1);
    if !data.config.boards.contains_key(&info.board) {
        return HttpResponse::Ok().body("Does not exist");
    }

    let current_page = page_data.page.unwrap_or(1);

    let client = data.db_client.lock().await;
    let head_msg: String;
    let head_msg_data = client.get_single_message(message_num).await;
    if let Ok(d) = head_msg_data {
        head_msg = data
            .formatter
            .format_into_message(
                html_proc::BoardMessageType::ParentMessage,
                &info.board,
                &d.get::<usize, i64>(0),        // message id
                &html_proc::get_time(d.get(1)), // time of creation
                &current_page.to_string(),
                &d.get::<usize, String>(2), // author
                &data
                    .formatter
                    .create_formatting(&d.get::<usize, String>(3))
                    .await, // message contents
                &d.get::<usize, String>(4), // associated image
            )
            .await;
    } else {
        return HttpResponse::Ok().body("404 No Such Message Found");
    }
    let mut inserted_submsg = String::from("");
    let mut submessage_counter = 0;
    for row in client.get_submessages(message_num).await.unwrap() {
        submessage_counter += 1;
        inserted_submsg.push_str(
            data.formatter
                .format_into_message(
                    html_proc::BoardMessageType::Submessage,
                    &info.board,
                    &submessage_counter,              // ordinal number
                    &html_proc::get_time(row.get(1)), // time of creation
                    &current_page.to_string(),
                    &data
                        .formatter
                        .filter_tags(&row.get::<usize, String>(2))
                        .await, // author
                    &data
                        .formatter
                        .create_formatting(&row.get::<usize, String>(3))
                        .await, // message contents
                    &row.get::<usize, String>(4), // associated image
                )
                .await
                .as_str(),
        );
    }

    HttpResponse::Ok().body(
        data.formatter
            .format_into_topic(
                &data.config.site_name,
                &message_num.to_string(),
                &head_msg,
                &inserted_submsg,
                &info.board.to_string(),
            )
            .await,
    )
}

/// Message handling logic for topics/threads
#[post("{board}/topic/{message_num}")]
async fn topic_process_form(
    data: web::Data<ApplicationState<'_>>,
    form: MultipartForm<MsgForm>,
    page_data: web::Query<QueryOptions>,
    info: web::Path<PathInfo>,
) -> impl Responder {
    let message_num = info.message_num.unwrap_or(1);
    if !data.config.boards.contains_key(&info.board) {
        return web::Redirect::to(format!("{}/topic/{}", info.board, message_num)).see_other();
    }
    let current_page = page_data.page.unwrap_or(1);

    let client = data.db_client.lock().await;
    let filepath_collection = process_files(&form.files).await;

    // getting time
    let since_epoch = html_proc::since_epoch();

    let trimmed_author = form.author.trim();
    let trimmed_message = form.message.trim();

    // if fits, push new message into DB
    if trimmed_author.len() < 254 && 0 != trimmed_message.len() && trimmed_message.len() < 4094 {
        let filtered_author = match trimmed_author.len() {
            0 => "Anonymous".to_string(),
            _ => data.formatter.filter_tags(&trimmed_author).await,
        };
        let filtered_msg = data.formatter.filter_tags(&trimmed_message).await;
        client
            .insert_to_submessages(
                message_num,
                since_epoch,
                &filtered_author,
                &filtered_msg,
                &filepath_collection,
            )
            .await;

        // counting submessages for given message
        let submsg_count = client.count_submessages(message_num).await.unwrap();

        // if number of submessages is below the bumplimit, update activity of parent msg
        if submsg_count <= data.config.bumplimit.into() {
            client
                .update_message_activity(since_epoch, message_num)
                .await;
        }
    }
    web::Redirect::to(format!(
        "/{}/topic/{}?page={}",
        info.board, message_num, current_page
    ))
    .see_other()
}
