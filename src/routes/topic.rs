//! Handlers for pages for individual posts

use actix_multipart::form::MultipartForm;
use actix_web::{get, http::StatusCode, post, web, HttpResponse, Responder};

use crate::html_proc;
use crate::routes::process_files;
use crate::routes::ApplicationState;
use crate::routes::MsgForm;
use crate::routes::PathInfo;
use crate::routes::QueryOptions;

/// Responder for individual topics/threads
#[get("{board}/topic/{message_num}")]
pub async fn topic(
    data: web::Data<ApplicationState<'_>>,
    info: web::Path<PathInfo>,
    page_data: web::Query<QueryOptions>,
) -> impl Responder {
    let message_num = info.message_num.unwrap_or(1);
    if !data.config.boards.contains_key(&info.board) {
        return HttpResponse::Ok().body(
            data.formatter
                .format_into_error(StatusCode::NOT_FOUND)
                .await,
        );
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
                d,
                &current_page.to_string(),
                None,
            )
            .await;
    } else {
        return HttpResponse::Ok().body(
            data.formatter
                .format_into_error(StatusCode::NOT_FOUND)
                .await,
        );
    }
    let mut inserted_submsg = String::from("");
    for row in client.get_submessages(message_num).await.unwrap() {
        inserted_submsg.push_str(
            data.formatter
                .format_into_submessage(
                    row,
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
pub async fn topic_process_form(
    data: web::Data<ApplicationState<'_>>,
    form: MultipartForm<MsgForm>,
    page_data: web::Query<QueryOptions>,
    info: web::Path<PathInfo>,
) -> impl Responder {
    const MAX_AUTHOR_LENGTH: usize = 250;
    const MAX_MESSAGE_LENGTH: usize = 4000;

    let message_num = info.message_num.unwrap_or(1);
    if !data.config.boards.contains_key(&info.board) {
        return web::Redirect::to(format!("{}/topic/{}", info.board, message_num)).see_other();
    }

    let client = data.db_client.lock().await;
    let filepath_collection = process_files(&form.files).await;

    // getting time
    let since_epoch = html_proc::since_epoch();

    let trimmed_author = form.author.trim();
    let trimmed_message = form.message.trim();

    // if fits, push new message into DB
    if trimmed_author.len() < MAX_AUTHOR_LENGTH
        && !trimmed_message.is_empty()
        && trimmed_message.len() < MAX_MESSAGE_LENGTH
    {
        let filtered_author = match trimmed_author.len() {
            0 => "Anonymous".to_string(),
            _ => data.formatter.filter_tags(trimmed_author).await,
        };
        let filtered_msg = data.formatter.filter_tags(trimmed_message).await;

        // Checking against the last message (to prevent spam)
        if let Ok(last_msg) = client.get_last_submessage(&message_num).await {
            if last_msg.submsg == filtered_msg {
                return web::Redirect::to("/error?error_code=403").see_other();
            }
        }

        let submsg_count = client.count_submessages(message_num).await.unwrap();

        client
            .insert_to_submessages(
                message_num,
                submsg_count + 1,
                &info.board,
                since_epoch,
                &filtered_author,
                &filtered_msg,
                &filepath_collection,
            )
            .await;

        if submsg_count < data.config.bumplimit.into() && form.sage.is_none() {
            client
                .update_message_activity(since_epoch, message_num)
                .await;
        }
    }
    web::Redirect::to(format!(
        "/{}/topic/{}{}",
        info.board,
        message_num,
        page_data.into_inner()
    ))
    .see_other()
}
