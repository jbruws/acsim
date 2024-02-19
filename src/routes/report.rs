//! Handlers for message reporting

use actix_multipart::form::MultipartForm;
use actix_web::{get, http::StatusCode, post, web, HttpResponse, Responder};

use crate::html_proc;
use crate::routes::process_files;
use crate::routes::ApplicationState;
use crate::routes::MsgForm;
use crate::routes::ReportQueryOptions;

/// Unified handler for reporting messages and submessages
#[get("/report")]
pub async fn report_msg(
    data: web::Data<ApplicationState<'_>>,
    page_data: web::Query<ReportQueryOptions>,
) -> impl Responder {
    let client = data.db_client.lock().await;
    let message_type = match page_data.subid {
        Some(v) => "submsg",
        None => "msg",
    };
    client
        .insert_to_flagged(message_type.to_string(), page_data.id, page_data.subid)
        .await;
    let submsg_optional = match page_data.subid {
        Some(n) => format!("#{}", n),
        None => "".to_string(),
    };
    let backlink = format!("{}{}", page_data.id, submsg_optional);
    HttpResponse::Ok().body(data.formatter.format_into_report(backlink).await)
}
