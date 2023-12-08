//! Handlers for boards

use actix_multipart::form::MultipartForm;
use actix_web::{get, post, web, HttpResponse, Responder};

use crate::html_proc;
use crate::routes::process_files;
use crate::routes::ApplicationState;
use crate::routes::MsgForm;
use crate::routes::PathInfo;
use crate::routes::QueryOptions;

use crate::db_control::{MessageRow, SubmessageRow};

/// Responder for boards
#[get("/{board}")]
pub async fn board(
    data: web::Data<ApplicationState<'_>>,
    info: web::Path<PathInfo>,
    page_data: web::Query<QueryOptions>,
) -> impl Responder {
    if !data.config.boards.contains_key(&info.board) {
        return HttpResponse::Ok().body("Does not exist");
    }
    let client = data.db_client.lock().await;
    let mut inserted_msg = String::from("");

    let mut current_page = page_data.page.unwrap_or(1);
    if current_page == 0 {
        current_page = 1;
    }

    // Restoring messages from DB
    for row in client
        .get_messages(
            &info.board,
            current_page as i64,
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
                    row,
                    &info.board,
                    &current_page.to_string(),
                    None,
                )
                .await
                .as_str(),
        );
    }

    let link_queries = page_data.into_inner().get_neighbour_pages();

    HttpResponse::Ok().body(
        data.formatter
            .format_into_board(
                &data.config,
                &info.board.to_string(),
                &inserted_msg,
                &link_queries.0.to_string(),
                &link_queries.1.to_string(),
            )
            .await,
    )
}

/// Message handling logic for boards
#[post("/{board}")]
pub async fn board_process_form(
    form: MultipartForm<MsgForm>,
    info: web::Path<PathInfo>,
    data: web::Data<ApplicationState<'_>>,
) -> impl Responder {
    const MAX_AUTHOR_LENGTH: usize = 250;
    const MAX_MESSAGE_LENGTH: usize = 4000;

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
    if trimmed_author.len() < MAX_AUTHOR_LENGTH
        && !trimmed_message.is_empty()
        && trimmed_message.len() < MAX_MESSAGE_LENGTH
    {
        let filtered_author = match trimmed_author.len() {
            0 => "Anonymous".to_string(),
            _ => data.formatter.filter_tags(trimmed_author).await,
        };
        let filtered_msg = data.formatter.filter_tags(trimmed_message).await;

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
