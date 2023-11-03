//! Handlers for pages for individual posts

use actix_multipart::form::MultipartForm;
use actix_web::{get, post, web, HttpResponse, Responder};

use crate::html_proc;
use crate::routes::ApplicationState;
use crate::routes::MsgForm;
use crate::routes::PathInfo;
use crate::routes::QueryOptions;
use crate::routes::process_files;

/// Responder for individual topics/threads
#[get("{board}/topic/{message_num}")]
pub async fn topic(
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
pub async fn topic_process_form(
    data: web::Data<ApplicationState<'_>>,
    form: MultipartForm<MsgForm>,
    page_data: web::Query<QueryOptions>,
    info: web::Path<PathInfo>,
) -> impl Responder {
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
    if trimmed_author.len() < 254 && !trimmed_message.is_empty() && trimmed_message.len() < 4094 {
        let filtered_author = match trimmed_author.len() {
            0 => "Anonymous".to_string(),
            _ => data.formatter.filter_tags(trimmed_author).await,
        };
        let filtered_msg = data.formatter.filter_tags(trimmed_message).await;
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
        "/{}/topic/{}{}",
        info.board,
        message_num,
        page_data.into_inner()
    ))
    .see_other()
}
