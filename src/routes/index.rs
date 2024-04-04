//! Handler for the main page (index)

use actix_web::{get, web, HttpResponse, Responder};

use crate::routes::ApplicationState;

/// Responder for site root
#[get("/")]
pub async fn root(data: web::Data<ApplicationState<'_>>) -> impl Responder {
    let mut board_links = Vec::new();
    for (board_name, desc) in &data.config.boards {
        board_links.push((board_name, desc));
    }

    HttpResponse::Ok().body(
        data.formatter
            .format_into_root(&data.config.site_name, board_links)
            .await,
    )
}
