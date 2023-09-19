use actix_files;
use actix_multipart::Multipart;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Mutex;
use tokio;
use tokio_postgres;

// functions for turning plaintext db data into html
mod html_proc;

#[derive(Serialize, Deserialize)]
struct BoardConfig {
    // IP address (numeric!) of the server where database is hosted
    db_host: String,
    // username used for database connection
    db_user: String,
    // password used for database connection
    db_password: String,
    // http server's IP. MUST be set even if you're binding to 0.0.0.0
    server_ip: String,
    // http server's port.
    server_port: u16,
    // `true` binds server to values set above, `false` binds to 0.0.0.0
    bind_to_one_ip: bool,
}

#[derive(Deserialize)]
struct MsgForm {
    message: String,
    author: String,
}

#[derive(Deserialize)]
struct PathInfo {
    message_num: i64,
}

struct ApplicationState {
    server_address: Mutex<String>,
    db_client: Mutex<tokio_postgres::Client>,
}

#[get("/")]
async fn main_page(data: web::Data<ApplicationState>) -> impl Responder {
    let server_address = data.server_address.lock().unwrap();
    let client = data.db_client.lock().unwrap();
    let mut inserted_msg = String::from("");

    // Restoring messages from DB
    for row in client
        .query("SELECT * FROM messages", &[])
        .await
        .unwrap()
        .into_iter()
        .rev()
    {
        inserted_msg.push_str(
            html_proc::format_into_html(
                html_proc::BoardMessageType::Message,
                &*server_address,
                &row.get::<usize, i64>(0),              // message id
                &html_proc::get_time(row.get(1)).await, // time of creation
                &row.get::<usize, String>(2),           // author
                &html_proc::prepare_msg(&row.get::<usize, String>(3), &*server_address).await, // message contents
            )
            .await
            .as_str(),
        );
    }
    HttpResponse::Ok().body(format!(include_str!("../html/index.html"), inserted_msg))
}

#[post("/")]
async fn process_form(
    form: web::Form<MsgForm>,
    data: web::Data<ApplicationState>,
) -> impl Responder {
    let client = data.db_client.lock().unwrap();

    // getting time
    let since_epoch: i64 = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
    {
        Ok(n) => n.as_secs().try_into().unwrap(),
        Err(_) => 1,
    };

    // if fits, push new message into DB and vector
    if form.author.len() < 254 && form.message.len() < 4094 {
        let filtered_author = html_proc::filter_string(&form.author).await;
        let filtered_msg = html_proc::filter_string(&form.message).await;

        client
            .execute(
                "INSERT INTO messages(time, author, msg) VALUES (($1), ($2), ($3))",
                &[&since_epoch, &filtered_author, &filtered_msg],
            )
            .await; // i'll just pretend this `Result` doesn't exist
    }
    web::Redirect::to("/").see_other()
}

#[get("/topic/{message_num}")]
async fn message_page(
    data: web::Data<ApplicationState>,
    info: web::Path<PathInfo>,
    //form: web::Form<MsgForm>,
) -> impl Responder {
    let client = data.db_client.lock().unwrap();
    let server_address = data.server_address.lock().unwrap();
    let head_msg: String;
    let head_msg_data = client
        .query_one(
            "SELECT * FROM messages WHERE msgid=($1)",
            &[&info.message_num],
        )
        .await;
    if let Ok(d) = head_msg_data {
        head_msg = html_proc::format_into_html(
            html_proc::BoardMessageType::ParentMessage,
            &*server_address,
            &d.get::<usize, i64>(0),              // message id
            &html_proc::get_time(d.get(1)).await, // time of creation
            &d.get::<usize, String>(2),           // author
            &html_proc::prepare_msg(&d.get::<usize, String>(3), &*server_address).await, // message contents
        )
        .await;
    } else {
        return HttpResponse::Ok().body("404 No Such Message Found");
    }
    let mut inserted_submsg = String::from("");
    let mut submessage_counter = 0;
    for row in client
        .query(
            "SELECT * FROM submessages WHERE parent_msg=($1)",
            &[&info.message_num],
        )
        .await
        .unwrap()
    {
        submessage_counter += 1;
        inserted_submsg.push_str(
            html_proc::format_into_html(
                html_proc::BoardMessageType::Submessage,
                &*server_address,
                &submessage_counter,                    // ordinal number
                &html_proc::get_time(row.get(1)).await, // time of creation
                &row.get::<usize, String>(2),           // author
                &html_proc::prepare_msg(&row.get::<usize, String>(3), &*server_address).await, // message contents
            )
            .await
            .as_str(),
        );
    }

    HttpResponse::Ok().body(format!(
        include_str!("../html/topic.html"),
        &info.message_num.to_string(),
        head_msg,
        inserted_submsg,
        &info.message_num.to_string(),
    ))
}

#[post("/topic/{message_num}")]
async fn process_submessage_form(
    data: web::Data<ApplicationState>,
    form: web::Form<MsgForm>,
    info: web::Path<PathInfo>,
) -> impl Responder {
    let client = data.db_client.lock().unwrap();
    let server_address = data.server_address.lock().unwrap();

    // getting time
    let since_epoch: i64 = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
    {
        Ok(n) => n.as_secs().try_into().unwrap(),
        Err(_) => 1,
    };

    // if fits, push new message into DB and vector
    if form.author.len() < 254 && form.message.len() < 4094 {
        let filtered_author = html_proc::filter_string(&form.author).await;
        let filtered_msg = html_proc::filter_string(&form.message).await;
        client
            .execute(
                "INSERT INTO submessages(parent_msg, time, author, submsg) VALUES (($1), ($2), ($3), ($4))",
                &[&info.message_num, &since_epoch, &filtered_author, &filtered_msg],
            )
            .await; // i'll just pretend this `Result` doesn't exist
    }
    web::Redirect::to(format!("{}/topic/{}", &*server_address, info.message_num)).see_other()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config: BoardConfig =
        serde_json::from_str(include_str!("../config.json")).expect("Can't parse config.json");

    let (client, connection) = tokio_postgres::connect(
        format!(
            "dbname=actixtest hostaddr={} user={} password={}",
            config.db_host.as_str(),
            config.db_user.as_str(),
            config.db_password.as_str()
        )
        .as_str(),
        tokio_postgres::NoTls,
    )
    .await
    .unwrap();

    // copypasted from docs.rs/tokio-postgres
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // creating unified address for the server
    let unified_address = format!("http://{}:{}", config.server_ip, config.server_port);

    // selecting where to bind the server
    let mut bound_ip = "0.0.0.0";
    if config.bind_to_one_ip {
        bound_ip = config.server_ip.as_str();
    }

    // creating application state
    let application_data = web::Data::new(ApplicationState {
        server_address: Mutex::new(unified_address),
        db_client: Mutex::new(client),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(application_data.clone())
            .service(actix_files::Files::new("/html", "./html"))
            .service(message_page)
            .service(process_form)
            .service(process_submessage_form)
            .service(main_page)
    })
    .bind((bound_ip, config.server_port))?
    .run()
    .await
}
