//! Handler for redirecting message links to appropriate boards

use crate::routes::ApplicationState;
use actix_web::{get, web, Responder};

/// Query option containing msgid-submsgid pair as a string
#[derive(serde::Deserialize)]
struct IdPairQueryOptions {
    idpair: String,
}

/// Redirects message links to appropriate boards
#[get("/to_msg")]
pub async fn to_msg(
    data: web::Data<ApplicationState<'_>>,
    query: web::Query<IdPairQueryOptions>,
) -> impl Responder {
    let client = data.db_client.lock().await;
    if query.idpair.contains('.') {
        // if both message and submessage are specified
        let parts: Vec<Result<i64, _>> =
            query.idpair.split('.').map(|x| x.parse::<i64>()).collect();
        if parts[0].is_err() || parts[1].is_err() {
            return web::Redirect::to("/error?error_code=500").see_other();
        }
        // unwrapping both values after confirming there are no Err's
        let parts: Vec<i64> = parts.into_iter().map(|x| x.unwrap()).collect();
        let msg = client.get_single_submessage(parts[0], parts[1]).await;
        match msg {
            Ok(row) => web::Redirect::to(format!("{}/topic/{}#{}", row.board, parts[0], parts[1]))
                .see_other(),
            Err(_) => web::Redirect::to("/error?error_code=404").see_other(),
        }
    } else {
        let msgid = query.idpair.parse::<i64>();
        if msgid.is_err() {
            return web::Redirect::to("/error?error_code=500").see_other();
        }
        let msgid = msgid.unwrap();
        let msg = client.get_single_message(msgid).await;
        match msg {
            Ok(row) => web::Redirect::to(format!("{}/topic/{}", row.board, msgid)).see_other(),
            Err(_) => web::Redirect::to("/error?error_code=404").see_other(),
        }
    }
}
