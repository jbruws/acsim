//! Handlers for boards

use actix_multipart::form::MultipartForm;
use actix_web::{get, http::StatusCode, post, web, HttpResponse, Responder};

use crate::html_proc;
use crate::routes::*;

/// Responder for boards
#[get("/{board}")]
pub async fn board(
    data: web::Data<ApplicationState<'_>>,
    info: web::Path<PathInfo>,
    page_data: web::Query<QueryOptions>,
) -> impl Responder {
    if !data.config.boards.contains_key(&info.board) {
        // we will have to manually format and send the response
        return HttpResponse::Ok().body(
            data.formatter
                .format_into_error(StatusCode::NOT_FOUND)
                .await,
        );
    }
    let client = data.db_client.lock().await;
    let mut inserted_msg = String::from("");

    let mut current_page = page_data.page.unwrap_or(1);
    if current_page == 0 {
        current_page = 1;
    }

    // Restoring messages from DB
    for row in client
        .get_messages(&info.board, current_page, data.config.page_limit as i64)
        .await
        .unwrap()
        .into_iter()
    {
        inserted_msg.push_str(
            data.formatter
                .format_into_message(
                    html_proc::BoardMessageType::Message,
                    row,
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
        return web::Redirect::to("/error?error_code=404").see_other();
    }

    let client = data.db_client.lock().await;
    let filepath_collection = process_files(&form.files).await;

    // getting time
    let since_epoch = html_proc::since_epoch();

    let trimmed_author = form.author.trim();
    let trimmed_message = form.message.trim();

    // if fits, run some checks and push new message into DB and vector
    if trimmed_author.len() < MAX_AUTHOR_LENGTH
        && !trimmed_message.is_empty()
        && trimmed_message.len() < MAX_MESSAGE_LENGTH
    {
        let filtered_author = match trimmed_author.len() {
            0 => "Anonymous".to_string(),
            _ => data.formatter.filter_tags(trimmed_author).await,
        };
        let filtered_msg = data.formatter.filter_tags(trimmed_message).await;

        // checking for banned words
        if contains_banned_words(&filtered_author).await
            || contains_banned_words(&filtered_msg).await
        {
            return web::Redirect::to("/error?error_code=403").see_other();
        }

        // Checking against the last message (to prevent spam)
        if let Ok(last_msg) = client.get_last_message(&info.board).await {
            if last_msg.msg == filtered_msg {
                return web::Redirect::to("/error?error_code=403").see_other();
            }
        }

        client
            .insert_to_messages(
                &info.board,
                since_epoch,
                &filtered_author,
                &filtered_msg,
                &filepath_collection,
                since_epoch,
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
