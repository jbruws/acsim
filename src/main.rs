use actix_files;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};
use serde::Deserialize;
use std::sync::Mutex;
use tokio;
use tokio_postgres;

#[derive(Deserialize)]
struct MsgForm {
    message: String,
    author: String,
}

struct ApplicationState {
    counter: Mutex<i32>,
    messages_vec: Mutex<Vec<(i64, String, String)>>,
    db_client: Mutex<tokio_postgres::Client>,
}

async fn main_page(data: web::Data<ApplicationState>) -> impl Responder {
    let mut counter = data.counter.lock().unwrap();
    *counter += 1;
    let messages = data.messages_vec.lock().unwrap();
    let mut inserted_msg = String::from("<br>");

    if messages.len() != 0 {
        for t in (&*messages).into_iter().rev() {
            let offset = FixedOffset::east_opt(3 * 3600).unwrap(); // +3 offset
            let naive = NaiveDateTime::from_timestamp_opt(t.0, 0).unwrap(); // UNIX epoch to datetime
            let dt = DateTime::<Local>::from_naive_utc_and_offset(naive, offset).to_string();

            inserted_msg.push_str(
                format!(
                    "<div class=\"message\">
                        <p class=\"message_header\">
                            {} <br> {}
                        </p>
                        <br> {}
                    </div>\n",
                    &dt[..dt.len() - 7], // 7 was chosen experimentally
                    &t.1,
                    &t.2
                )
                .as_str(),
            );
        }
    }
    // TODO: input validation
    // try inserting a <script> into the page. it's funny
    HttpResponse::Ok().body(format!(include_str!("../html/index.html"), inserted_msg))
}

async fn process_form(
    form: web::Form<MsgForm>,
    data: web::Data<ApplicationState>,
) -> impl Responder {
    let mut messages = data.messages_vec.lock().unwrap();
    let client = data.db_client.lock().unwrap();

    // getting time
    let since_epoch: i64 = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
    {
        Ok(n) => n.as_secs().try_into().unwrap(),
        Err(_) => 1,
    };

    // if fits, push new message into DB and vector
    if form.author.len() < 254 && form.message.len() < 4094 {
        messages.push((since_epoch, form.author.clone(), form.message.clone()));
        client
            .execute(
                "INSERT INTO messages(time, author, msg) VALUES (($1), ($2), ($3))",
                &[&since_epoch, &form.author, &form.message],
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
    let mut db_messages: Vec<(i64, String, String)> = Vec::new();
    for row in client.query("SELECT * FROM messages", &[]).await.unwrap() {
        db_messages.push((row.get(0), row.get(1), row.get(2)));
    }

    // creating application state
    let count = web::Data::new(ApplicationState {
        counter: Mutex::new(0),
        messages_vec: Mutex::new(db_messages),
        db_client: Mutex::new(client),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(count.clone())
            .service(actix_files::Files::new("/html", "./html"))
            .route("/", web::get().to(main_page))
            .route("/process_form", web::post().to(process_form))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
