//! Handler for error pages

use actix_web::{get, web, HttpResponse, Responder};

use crate::routes::ApplicationState;
use crate::routes::ErrorQuery;

/// Returns the error page with appropriate error displayed
#[get("/error")]
pub async fn error_page(data: web::Data<ApplicationState<'_>>, q: web::Query<ErrorQuery>) -> impl Responder {
    let ecode_unwrapped = match q.error_code {
        Some(v) => v,
        None => 500,
    };
    HttpResponse::Ok().body(data.formatter.format_into_error(ecode_unwrapped).await)
}
