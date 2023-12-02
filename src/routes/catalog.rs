//! Handler for board post catalogs

use actix_web::{get, web, HttpResponse, Responder};

use crate::html_proc;
use crate::routes::ApplicationState;
use crate::routes::PathInfo;
use crate::routes::QueryOptions;

/// Responder for board catalogs
#[get("{board}/catalog")]
pub async fn board_catalog(
    data: web::Data<ApplicationState<'_>>,
    page_data: web::Query<QueryOptions>,
    info: web::Path<PathInfo>,
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

    let catalog_messages;
    if let Some(search_string) = &page_data.search_string {
        catalog_messages = client
            .search_messages(
                &info.board,
                (current_page - 1) * data.config.page_limit as i64,
                data.config.page_limit as i64,
                search_string,
            )
            .await
            .unwrap();
    } else {
        catalog_messages = client
            .get_messages(
                &info.board,
                current_page as i64,
                data.config.page_limit as i64,
            )
            .await
            .unwrap()
    }

    // Restoring messages from DB
    for row in catalog_messages.into_iter() {
        inserted_msg.push_str(
            data.formatter
                .format_into_message(
                    html_proc::BoardMessageType::CatalogMessage,
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
            .format_into_catalog(
                &info.board,
                &inserted_msg,
                &link_queries.0.to_string(),
                &link_queries.1.to_string(),
            )
            .await,
    )
}
