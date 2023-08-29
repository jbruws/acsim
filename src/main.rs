use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
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
    let messages = data.messages_vec.lock().unwrap();
    if messages.len() == 0 {
        HttpResponse::Ok().body(format!("Hello world! \n\nThere aren't any messages yet."))
    } else {
        HttpResponse::Ok().body(format!("Hello world! \n\nMessages: \n\n{}", messages.join("\n")))
    }

}

async fn show_form(form: web::Form<MsgForm>, data: web::Data<MutexState>) -> impl Responder {
    let mut messages = data.messages_vec.lock().unwrap();
    messages.push(form.message.clone());
    HttpResponse::Ok().body(format!("Thank you for your admission: {}", form.message))
}

async fn forms() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/form.html"))
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
            .route("/forms", web::get().to(forms))
            .route("/show_form", web::post().to(show_form))
            .route("/counter", web::get().to(manual_hello))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
