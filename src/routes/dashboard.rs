//! Handlers for admin dashboard and login page

use crate::routes::ApplicationState;
use actix_web::{get, post, web, HttpResponse, Responder};

/// Struct containing query options for dashboard
#[derive(serde::Deserialize)]
struct DashboardQueryOptions {
    flagged_type: Option<String>,
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

    println!("{}", session.get::<bool>("logged_in").unwrap().unwrap_or(false));

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
                        result.push_str(&data.formatter.format_into_message(crate::html_proc::BoardMessageType::Message, i, "1", None).await);
                        result.push('\n');
                    }
                }
                result
            },
            _ => { // anything other than 'msg' is treated as a submessage
                let msg_vec = client.get_flagged_submessages().await;
                let mut result = "".to_string();
                if let Ok(v) = msg_vec {
                    for i in v {
                        result.push_str(&data.formatter.format_into_submessage(i).await);
                        result.push('\n');
                    }
                }
                result
            },
        },
        None => "".to_string(),
    };

    HttpResponse::Ok().body(flagged_msg_block)
}

/// Handler for processing login credentials
#[post("/dashboard")]
pub async fn login_page(
    data: web::Data<ApplicationState<'_>>,
    session: actix_session::Session,
    form: web::Form<LoginForm>,
    //query: web::Query<DashboardQueryOptions>,
) -> impl Responder {
    if form.password == data.config.admin_password {
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
