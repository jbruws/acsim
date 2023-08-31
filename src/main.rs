use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::sync::Mutex;
use serde::Deserialize;

#[derive(Deserialize)]
struct MsgForm {
    message: String,
}

struct MutexState {
    counter: Mutex<i32>,
    messages_vec: Mutex<Vec<String>>,
}

async fn hello(data: web::Data<MutexState>) -> impl Responder {
    let mut counter = data.counter.lock().unwrap();
    *counter += 1;
    let mut messages = data.messages_vec.lock().unwrap();
    let mut inserted_msg = String::from("There aren't any messages yet.");
    if messages.len() != 0 {
        (*messages).reverse();
        inserted_msg = messages.join("<br>");
    }
    // TODO: input validation
    // try inserting a <script> into the page. it's funny
    HttpResponse::Ok().body(format!(include_str!("../static/index.html"), counter, inserted_msg))
}

async fn process_form(form: web::Form<MsgForm>, data: web::Data<MutexState>) -> impl Responder {
    let mut messages = data.messages_vec.lock().unwrap();
    messages.push(form.message.clone());
    web::Redirect::to("http://192.168.0.110:8080").see_other()
}

async fn manual_hello(data: web::Data<MutexState>) -> impl Responder {
    let mut counter = data.counter.lock().unwrap();
    *counter += 1;
    HttpResponse::Ok().body(format!("Hey there! \nPage reloads: {}", counter))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let count = web::Data::new(MutexState {
        counter: Mutex::new(0),
        messages_vec: Mutex::new(Vec::new()),
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
