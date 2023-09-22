// std
use std::path::PathBuf;
use std::sync::Mutex;
// actix and serde
use actix_files;
use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use actix_web::{middleware::Logger, get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use serde_json;
// misc
use tokio;
use tokio_postgres;
use rand;
use fern;
use log;

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

#[derive(MultipartForm)]
struct MsgForm {
    message: Text<String>,
    author: Text<String>,
    #[multipart(limit = "5 MiB")]
    image: Option<TempFile>,
}

#[derive(Deserialize)]
struct PathInfo {
    message_num: i64,
}

struct ApplicationState {
    server_address: Mutex<String>,
    db_client: Mutex<tokio_postgres::Client>,
}

fn log_query_status(status: Result<u64, tokio_postgres::error::Error>, operation: &str) {
    match status {
        Ok(v) => log::debug!("{} success: {}", operation, v),
        Err(e) => log::error!("{} failure: {}", operation, e),
    };
}

#[get("/")]
async fn main_page(data: web::Data<ApplicationState>) -> impl Responder {
    let server_address = data.server_address.lock().unwrap();
    let client = data.db_client.lock().unwrap();
    let mut inserted_msg = String::from("");

    // Restoring messages from DB
    for row in client
        .query("SELECT * FROM messages ORDER BY latest_submsg ASC", &[])
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
                &html_proc::get_time(row.get(1)), // time of creation
                &html_proc::filter_string(&row.get::<usize, String>(2)).await, // author
                &html_proc::prepare_msg(&row.get::<usize, String>(3), &*server_address).await, // message contents
                &row.get::<usize, String>(4), // associated image
            )
            .await
            .as_str(),
        );
    }
    HttpResponse::Ok().body(format!(include_str!("../html/index.html"), inserted_msg))
}

#[post("/")]
async fn process_form(
    form: MultipartForm<MsgForm>,
    data: web::Data<ApplicationState>,
) -> impl Responder {
    let client = data.db_client.lock().unwrap();
    let mut new_filepath: PathBuf = PathBuf::new();

    if let Some(f) = &form.image {
        let temp_file_path = f.file.path();
        if f.file_name != Some(String::from("")) {
            let orig_name = f
                .file_name
                .as_ref()
                .expect("no file name")
                .split(".")
                .collect::<Vec<&str>>();
            let new_name = rand::random::<u64>().to_string();
            new_filepath = PathBuf::from(format!("./user_images/{}.{}", new_name, orig_name[1]));
            let _copy_status = std::fs::copy(temp_file_path, new_filepath.clone());
            let _delete_status = std::fs::remove_file(temp_file_path);
        }
    }

    // getting time
    let since_epoch = html_proc::since_epoch();

    // if fits, push new message into DB and vector
    if form.author.len() < 254 && form.message.len() < 4094 {
        let filtered_author = html_proc::filter_string(&form.author).await;
        let filtered_msg = html_proc::filter_string(&form.message).await;

        let result_update = client
            .execute(
                "INSERT INTO messages(time, author, msg, image, latest_submsg) VALUES (($1), ($2), ($3), ($4), ($5))",
                &[
                    &since_epoch,
                    &filtered_author,
                    &filtered_msg,
                    &new_filepath.to_str().unwrap(),
                    &since_epoch,
                ],
            )
            .await;
            log_query_status(result_update, "Message table insertion");
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
            &html_proc::get_time(d.get(1)), // time of creation
            &html_proc::filter_string(&d.get::<usize, String>(2)).await, // author
            &html_proc::prepare_msg(&d.get::<usize, String>(3), &*server_address).await, // message contents
            &d.get::<usize, String>(4), // associated image
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
                &html_proc::get_time(row.get(1)), // time of creation
                &html_proc::filter_string(&row.get::<usize, String>(2)).await, // author
                &html_proc::prepare_msg(&row.get::<usize, String>(3), &*server_address).await, // message contents
                &row.get::<usize, String>(4), // associated image
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
    form: MultipartForm<MsgForm>,
    info: web::Path<PathInfo>,
) -> impl Responder {
    let client = data.db_client.lock().unwrap();
    let server_address = data.server_address.lock().unwrap();
    let mut new_filepath: PathBuf = PathBuf::new();

    if let Some(f) = &form.image {
        let temp_file_path = f.file.path();
        if f.file_name != Some(String::from("")) {
            let orig_name = f
                .file_name
                .as_ref()
                .expect("no file name")
                .split(".")
                .collect::<Vec<&str>>();
            let new_name = rand::random::<u64>().to_string();
            new_filepath = PathBuf::from(format!("./user_images/{}.{}", new_name, orig_name[1]));
            let _copy_status = std::fs::copy(temp_file_path, new_filepath.clone());
            let _remove_status = std::fs::remove_file(temp_file_path);
        }
    }

    // getting time
    let since_epoch = html_proc::since_epoch();

    // if fits, push new message into DB and vector
    if form.author.len() < 254 && form.message.len() < 4094 {
        let filtered_author = html_proc::filter_string(&form.author).await;
        let filtered_msg = html_proc::filter_string(&form.message).await;
        let result_update = client
            .execute(
                "INSERT INTO submessages(parent_msg, time, author, submsg, image) VALUES (($1), ($2), ($3), ($4), ($5))",
                &[&info.message_num, &since_epoch, &filtered_author, &filtered_msg, &new_filepath.to_str().unwrap()],
            )
            .await;
        log_query_status(result_update, "Submessage table insertion");
        
        let result_update2 = client
            .execute(
                "UPDATE messages SET latest_submsg = ($1) WHERE msgid = ($2)",
                &[&since_epoch, &info.message_num],
            )
            .await;
        log_query_status(result_update2, "Message table update");
    }
    web::Redirect::to(format!("{}/topic/{}", &*server_address, info.message_num)).see_other()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // reading board config
    let config: BoardConfig =
        serde_json::from_str(include_str!("../config.json")).expect("Can't parse config.json");

    // connecting to the database
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
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // selecting where to bind the server
    let mut bound_ip = "0.0.0.0";
    if config.bind_to_one_ip {
        bound_ip = config.server_ip.as_str();
    }

    // creating application state
    let unified_address = format!("http://{}:{}", config.server_ip, config.server_port);
    let application_data = web::Data::new(ApplicationState {
        server_address: Mutex::new(unified_address),
        db_client: Mutex::new(client),
    });

    // starting the logger
    let logger = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                html_proc::get_time(html_proc::since_epoch()),
                record.level(),
                message,
            ))
        })
        .level(log::LevelFilter::Info) // change `Info` to `Debug` for db query logs
        .chain(std::io::stdout())
        .chain(fern::log_file("actixtest.log").unwrap())
        .apply();
    match logger {
        Ok(_) => log::info!("Board engine starting"),
        Err(e) => println!("WARNING: Failed to start logger: {}", e)
    };

    // starting the server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(application_data.clone())
            .service(actix_files::Files::new("/html", "./html"))
            .service(actix_files::Files::new("/user_images", "./user_images"))
            .service(message_page)
            .service(process_form)
            .service(process_submessage_form)
            .service(main_page)
    })
    .bind((bound_ip, config.server_port))?
    .run()
    .await
}
