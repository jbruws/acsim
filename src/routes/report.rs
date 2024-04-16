//! Handler for message reporting

use crate::routes::ApplicationState;
use actix_web::{get, post, web, HttpResponse, Responder};

/// Query params that specify reported messages
#[derive(serde::Deserialize)]
struct ReportQueryOptions {
    id: i64,
    subid: Option<i64>,
}

/// Form for report confirmation captcha
#[derive(serde::Deserialize)]
struct ReportCaptchaForm {
    captcha_answer: String,
    captcha_hash: String,
    id: i64,
    subid: Option<i64>,
}

/// Unified handler for reporting messages and submessages
#[get("/report")]
pub async fn report_msg(
    data: web::Data<ApplicationState<'_>>,
    page_data: web::Query<ReportQueryOptions>,
) -> impl Responder {
    let captcha_hash =
        sha256::digest(crate::routes::create_new_captcha(data.config.captcha_num_limit).await);
    HttpResponse::Ok().body(
        data.formatter
            .format_into_report_captcha("".to_string(), captcha_hash, page_data.id, page_data.subid)
            .await,
    )
}

/// Handler for receiving captcha answer
#[post("/report")]
pub async fn report_process_captcha(
    data: web::Data<ApplicationState<'_>>,
    form: web::Form<ReportCaptchaForm>,
) -> impl Responder {
    if form.captcha_hash.clone() != sha256::digest(form.captcha_answer.clone()) {
        return HttpResponse::Ok().body(
            data.formatter
                .format_into_error(actix_web::http::StatusCode::FORBIDDEN)
                .await,
        );
    }

    // delete captcha image after usage
    crate::routes::delete_captcha_image(form.captcha_answer.to_string()).await;

    let client = data.db_client.lock().await;
    let message_type = match form.subid {
        Some(_v) => "submsg",
        None => "msg",
    };
    client
        .insert_to_flagged(message_type.to_string(), form.id, form.subid)
        .await;
    HttpResponse::Ok().body(data.formatter.format_into_report_accepted().await)
}
