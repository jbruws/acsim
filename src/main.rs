use actix_files;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};
use regex::Regex;
use serde::Deserialize;
use std::sync::Mutex;
use tokio;
use tokio_postgres;

// WARNING: none of this works yet.
// Topics/submessages are still WIP.

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
    counter: Mutex<i32>,
    messages_vec: Mutex<Vec<(i64, i64, String, String)>>,
    db_client: Mutex<tokio_postgres::Client>,
    last_message_id: Mutex<i64>,
}

fn get_time(since_epoch: i64) -> String {
    let offset = FixedOffset::east_opt(3 * 3600).unwrap(); // +3 offset
    let naive = NaiveDateTime::from_timestamp_opt(since_epoch, 0).unwrap(); // UNIX epoch to datetime
    let dt = DateTime::<Local>::from_naive_utc_and_offset(naive, offset).to_string();
    dt[..dt.len() - 7].to_string() // 7 was chosen experimentally
}

fn filter_string(inp_string: &String) -> String { // removing html tags
    let filter = Regex::new(r##"<.*?>"##).unwrap(); 
    String::from(filter.replace_all(inp_string.as_str(), ""))
}

async fn main_page(data: web::Data<ApplicationState>) -> impl Responder {
    let mut counter = data.counter.lock().unwrap();
    *counter += 1;
    let messages = data.messages_vec.lock().unwrap();
    let mut inserted_msg = String::from("");

    if messages.len() != 0 {
        for t in (&*messages).into_iter().rev() {
            let string_time = get_time(t.1);
            inserted_msg.push_str(
                format!(
                    "<div class=\"message\" id={}> <p class=\"message_header\">{} <br> {} <br> {}</p><br> {}</div>\n",
                    &t.0, // message id
                    &string_time, // time of creation
                    &t.2, // author
                    &t.0,
                    &t.3 // message contents
                )
                .as_str(),
            );
        }
    }
    HttpResponse::Ok().body(format!(include_str!("../html/index.html"), inserted_msg))
}

#[post("/topic/{message_num}")]
async fn message_page(
    data: web::Data<ApplicationState>,
    info: web::Path<PathInfo>,
    form: web::Form<MsgForm>,
) -> impl Responder {
    let client = data.db_client.lock().unwrap();
    let mut head_msg: String;
    let head_msg_data = client
        .query_one(
            "SELECT * FROM messages WHERE msgid=($1)",
            &[&info.message_num],
        )
        .await;
    if let Ok(d) = head_msg_data {
        head_msg = format!("<div id=\"head_message\" id={}> <p class=\"message_header\">{} <br> {} <br> {}</p><br> {}</div>\n",
            d.get::<usize, i64>(0), // message id
            get_time(d.get(1)), // time of creation
            d.get::<usize, String>(2), // author
            d.get::<usize, i64>(0),
            d.get::<usize, String>(3) // message contents
        );
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
        let string_time = get_time(row.get(1));
        inserted_submsg.push_str(
                format!(
                    "<div class=\"submessage\" id={}> <p class=\"message_header\">{} <br> {} <br> {}</p><br> {}</div>\n",
                    &submessage_counter, // ordinal number
                    &string_time, // time of creation
                    &row.get::<usize, String>(2), // author
                    &submessage_counter,
                    &row.get::<usize, String>(3) // message contents
                )
                .as_str(),
            );
    }

    HttpResponse::Ok().body(format!(
        include_str!("../html/topic.html"),
        &info.message_num.to_string(),
        head_msg,
        inserted_submsg,
        &info.message_num.to_string(), // for the form
    ))
}

async fn process_submessage_form(
    data: web::Data<ApplicationState>,
    info: web::Path<PathInfo>,
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
        let filtered_author = filter_string(&form.author);
        let filtered_msg = filter_string(&form.message);

        client
            .execute(
                "INSERT INTO submessages(parent_msg, time, author, msg) VALUES (($1), ($2), ($3), ($4))",
                &[&info.message_num, &since_epoch, &filtered_author, &filtered_msg],
            )
            .await; // i'll just pretend this `Result` doesn't exist
    }
}

async fn process_form(
    form: web::Form<MsgForm>,
    data: web::Data<ApplicationState>,
) -> impl Responder {
    let mut messages = data.messages_vec.lock().unwrap();
    let client = data.db_client.lock().unwrap();
    let mut last_message_id = data.last_message_id.lock().unwrap();

    // getting time
    let since_epoch: i64 = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
    {
        Ok(n) => n.as_secs().try_into().unwrap(),
        Err(_) => 1,
    };

    // if fits, push new message into DB and vector
    if form.author.len() < 254 && form.message.len() < 4094 {
        let filtered_author = filter_string(&form.author);
        let filtered_msg = filter_string(&form.message);
        *last_message_id += 1;

        messages.push((
            *last_message_id,
            since_epoch,
            filtered_author.clone(),
            filtered_msg.clone(),
        ));
        client
            .execute(
                "INSERT INTO messages(time, author, msg) VALUES (($1), ($2), ($3))",
                &[&since_epoch, &filtered_author, &filtered_msg],
            )
            .await; // i'll just pretend this `Result` doesn't exist
    }
    web::Redirect::to("http://192.168.0.110:8080").see_other()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Reading database credentials and connecting
    // (credentials not included in repo. create the file yourself)
    let db_credentials: Vec<_> = include_str!("../db_user").split(" ").collect();
    let (client, connection) = tokio_postgres::connect(
        format!(
            "host=localhost user={} password={}",
            db_credentials[0], db_credentials[1]
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

    // Restoring messages from DB
    let mut db_messages: Vec<(i64, i64, String, String)> = Vec::new();
    for row in client.query("SELECT * FROM messages", &[]).await.unwrap() {
        db_messages.push((row.get(0), row.get(1), row.get(2), row.get(3)));
    }

    // getting serial ID of the last message
    let last_id = db_messages[db_messages.len() - 1].0;

    // creating application state
    let count = web::Data::new(ApplicationState {
        counter: Mutex::new(0),
        messages_vec: Mutex::new(db_messages),
        db_client: Mutex::new(client),
        last_message_id: Mutex::new(last_id),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(count.clone())
            .service(actix_files::Files::new("/html", "./html"))
            .service(message_page)
            .route("/", web::get().to(main_page))
            .route("/process_form", web::post().to(process_form))
            //.route("/topic/{message_num}/process_submessage_form", web::post().to(process_submessage_form))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
