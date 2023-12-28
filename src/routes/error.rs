//! Handler for error pages

use actix_web::{get, web, HttpResponse, Responder};

use crate::routes::ApplicationState;
use crate::routes::ErrorQuery;

/// Returns the error page with appropriate error displayed
#[get("/error")]
pub async fn error_page(
    data: web::Data<ApplicationState<'_>>,
    q: web::Query<ErrorQuery>,
) -> impl Responder {
    let ecode_unwrapped = q.error_code.unwrap_or(500);
    HttpResponse::Ok().body(data.formatter.format_into_error(actix_web::http::StatusCode::from_u16(ecode_unwrapped.try_into().unwrap()).unwrap()).await)
}
