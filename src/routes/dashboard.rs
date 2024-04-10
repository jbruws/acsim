//! Handlers for admin dashboard and login page

use crate::routes::ApplicationState;
use actix_web::{get, post, web, HttpResponse, Responder};

/// Query params for dashboard page switching
#[derive(serde::Deserialize)]
struct DashboardQueryOptions {
    flagged_type: Option<String>,
}

/// Container for query parameters regarding deleted messages
#[derive(serde::Deserialize)]
struct DeletionQueryOptions {
    msgid: i64,
    submsgid: Option<i64>,
}

/// Form used to send admin login credentials
#[derive(serde::Deserialize)]
struct LoginForm {
    password: String,
}

/// Handler for admin dashboard
#[get("/dashboard")]
pub async fn view_dashboard(
    data: web::Data<ApplicationState<'_>>,
    session: actix_session::Session,
    query: web::Query<DashboardQueryOptions>,
) -> impl Responder {
    let logged_in = match session.get::<bool>("logged_in") {
        Ok(opt) => opt.unwrap_or(false),
        Err(_) => false,
    };

    if !logged_in {
        return HttpResponse::Ok().body(data.formatter.format_into_login().await);
    }

    let client = data.db_client.lock().await;

    let flagged_msg_block: String = match &query.flagged_type {
        Some(n) => match n.as_str() {
            "msg" => {
                let msg_vec = client.get_flagged_messages().await;
                let mut result = "".to_string();
                if let Ok(v) = msg_vec {
                    for i in v {
                        let msgid = i.msgid;
                        result.push_str(
                            &data
                                .formatter
                                .format_into_message(
                                    crate::html_proc::BoardMessageType::Message,
                                    i,
                                    "1",
                                    None,
                                )
                                .await,
                        );
                        result.push_str(
                            format!("<a href=\"/delete?msgid={}\">Delete</a>\n", msgid).as_str(),
                        );
                        result.push('\n');
                    }
                }
                result
            }
            _ => {
                let msg_vec = client.get_flagged_submessages().await;
                let mut result = "".to_string();
                if let Ok(v) = msg_vec {
                    for i in v {
                        let parent_msg = i.parent_msg;
                        let submsg_id = i.submsg_id;
                        result.push_str(&data.formatter.format_into_submessage(i).await);
                        result.push_str(
                            format!(
                                "<a href=\"/delete?msgid={}&submsgid={}\">Delete</a>\n",
                                parent_msg, submsg_id
                            )
                            .as_str(),
                        );
                        result.push('\n');
                    }
                }
                result
            }
        },
        None => {
            let mut board_raw: Vec<(String, i64, i64, i64, i64)> = Vec::new();
            for i in data.config.boards.keys() {
                let count_msg = client.count_messages(i).await.unwrap_or(0);
                let count_submsg = client.count_board_submessages(i).await.unwrap_or(0);
                let rate = client.get_posting_rate(i, 3600).await.unwrap_or(0);
                board_raw.push((
                    String::from(i),
                    count_msg,
                    count_submsg,
                    count_msg + count_submsg,
                    rate,
                ));
            }
            data.formatter.format_into_board_data(board_raw).await
        }
    };

    HttpResponse::Ok().body(
        data.formatter
            .format_into_dashboard(flagged_msg_block)
            .await,
    )
}

/// Handler for processing login credentials
#[post("/dashboard")]
pub async fn login_page(
    data: web::Data<ApplicationState<'_>>,
    session: actix_session::Session,
    form: web::Form<LoginForm>,
) -> impl Responder {
    // the password is stored as a hash. no im not hashing it client-side.
    let hashed = sha256::digest(&form.password);
    if hashed == data.config.admin_password {
        let session_insert_result = session.insert("logged_in", true);
        match session_insert_result {
            Ok(_) => log::info!("Admin successfully authorized"),
            Err(_) => log::error!("Failed to authorize admin"),
        }
        web::Redirect::to("/dashboard").see_other()
    } else {
        web::Redirect::to("/error?error_code=403").see_other()
    }
}

/// Handler for flagged message deletion
#[get("/delete")]
pub async fn delete_msg(
    data: web::Data<ApplicationState<'_>>,
    session: actix_session::Session,
    query: web::Query<DeletionQueryOptions>,
) -> impl Responder {
    if let Ok(Some(logged_in)) = session.get::<bool>("logged_in") {
        if !logged_in {
            return web::Redirect::to("/error?error_code=403").see_other();
        }
    } else {
        return web::Redirect::to("/error?error_code=403").see_other();
    }
    let client = data.db_client.lock().await;
    if let Some(submsgid) = query.submsgid {
        let row = client
            .get_single_submessage(query.msgid, submsgid)
            .await
            .unwrap();
        let image_paths = row.image.split(';').collect::<Vec<&str>>();
        purge_images(image_paths);
        client.delete_submsg(query.msgid, submsgid).await;
        web::Redirect::to("/dashboard?flagged_type=submsg").see_other()
    } else {
        let row = client.get_single_message(query.msgid).await.unwrap();
        let submsg_vec = client.get_submessages(query.msgid).await;
        if let Ok(v) = submsg_vec {
            for i in v {
                let image_paths_sub = i.image.split(';').collect::<Vec<&str>>();
                purge_images(image_paths_sub);
            }
        }
        let image_paths = row.image.split(';').collect::<Vec<&str>>();
        purge_images(image_paths);
        client.delete_msg(query.msgid).await;
        web::Redirect::to("/dashboard?flagged_type=msg").see_other()
    }
}

/// Removes all specified file paths
fn purge_images(paths: Vec<&str>) {
    if paths == Vec::from([""]) {
        return ();
    }
    for i in paths {
        match std::fs::remove_file(std::path::Path::new(&format!(
            "{}/{}",
            env!("CARGO_MANIFEST_DIR"),
            i
        ))) {
            Ok(_) => log::debug!("Deleted media file: {}", i),
            Err(_) => log::error!("Media file deletion failed: {}", i),
        };
    }
}
