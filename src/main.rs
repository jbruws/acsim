use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;
use std::sync::Mutex;
use tokio;
use tokio_postgres;

#[derive(Deserialize)]
struct MsgForm {
    message: String,
}

struct MutexState {
    counter: Mutex<i32>,
    messages_vec: Mutex<Vec<String>>,
    db_client: Mutex<tokio_postgres::Client>,
}

async fn hello(data: web::Data<MutexState>) -> impl Responder {
    let mut counter = data.counter.lock().unwrap();
    *counter += 1;
    let mut messages = data.messages_vec.lock().unwrap();
    let mut inserted_msg = String::from("There aren't any messages yet.");
    if messages.len() != 0 {
        (*messages).reverse(); // newest ones on top
        inserted_msg = messages.join("<br>");
    }
    // TODO: input validation
    // try inserting a <script> into the page. it's funny
    HttpResponse::Ok().body(format!(
        include_str!("../static/index.html"),
        counter, inserted_msg
    ))
}

async fn process_form(form: web::Form<MsgForm>, data: web::Data<MutexState>) -> impl Responder {
    let mut messages = data.messages_vec.lock().unwrap();
    let client = data.db_client.lock().unwrap();

    // pushing new message into DB and vector
    messages.push(form.message.clone());
    client
        .execute(
            "INSERT INTO Messages(message) VALUES ($1)",
            &[&form.message],
        )
        .await
        .unwrap();
    web::Redirect::to("http://192.168.0.110:8080").see_other()
}

async fn manual_hello(data: web::Data<MutexState>) -> impl Responder {
    let mut counter = data.counter.lock().unwrap();
    *counter += 1;
    HttpResponse::Ok().body(format!("Hey there! \nPage reloads: {}", counter))
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
    let mut db_messages = Vec::new();
    for row in client
        .query("SELECT message FROM Messages", &[])
        .await
        .unwrap()
    {
        db_messages.push(row.get(0));
    }

    // creating application state
    let count = web::Data::new(MutexState {
        counter: Mutex::new(0),
        messages_vec: Mutex::new(db_messages),
        db_client: Mutex::new(client),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(count.clone())
            .route("/", web::get().to(hello))
            .route("/process_form", web::post().to(process_form))
            .route("/counter", web::get().to(manual_hello))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
