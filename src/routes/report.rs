//! Handler for message reporting

use crate::routes::ApplicationState;
use actix_web::{get, web, HttpResponse, Responder};

/// Options that specify reported messages
#[derive(serde::Deserialize)]
struct ReportQueryOptions {
    id: i64,
    subid: Option<i64>,
}

/// Unified handler for reporting messages and submessages
#[get("/report")]
pub async fn report_msg(
    data: web::Data<ApplicationState<'_>>,
    page_data: web::Query<ReportQueryOptions>,
) -> impl Responder {
    let client = data.db_client.lock().await;
    let message_type = match page_data.subid {
        Some(_v) => "submsg",
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
