use actix_files;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Mutex;
use tokio;
use tokio_postgres;

enum BoardMessageType {
    Message,       // messages on main page
    ParentMessage, // parent message on topic pages
    Submessage,    // submessages on topic pages
}

#[derive(Serialize, Deserialize)]
struct BoardConfig {
    // address of the server where database is hosted
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

// returns current date in time in 'YYYY-MM-DD hh:mm:ss' 24-hour format
async fn get_time(since_epoch: i64) -> String {
    let offset = FixedOffset::east_opt(3 * 3600).unwrap(); // +3 (hours) offset
    let naive = NaiveDateTime::from_timestamp_opt(since_epoch, 0).unwrap(); // UNIX epoch to datetime
    let dt = DateTime::<Local>::from_naive_utc_and_offset(naive, offset).to_string();
    dt[..dt.len() - 7].to_string() // 7 was chosen experimentally
}

// fits form data into one of several html templates
async fn format_into_html(
    message_type: BoardMessageType,
    address: &str,
    id: &i64,
    time: &str,
    author: &str,
    msg: &str,
) -> String {
    let f_result = match message_type {
        BoardMessageType::Message => format!(
            include_str!("../message_templates/message.html"),
            id = id,
            address = address,
            time = time,
            author = author,
            msg = msg
        ),
        BoardMessageType::ParentMessage => format!(
            include_str!("../message_templates/parent_message.html"),
            address = address,
            time = time,
            author = author,
            id = id,
            msg = msg
        ),
        BoardMessageType::Submessage => format!(
            include_str!("../message_templates/submessage.html"),
            id = id,
            time = time,
            author = author,
            msg = msg
        ),
    };
    f_result
}

// removes html tags from message
async fn filter_string(inp_string: &String) -> String {
    let filter = Regex::new(r##"<.*?>"##).unwrap();
    String::from(filter.replace_all(inp_string.as_str(), ""))
}

// processes messages entered by users by adding things usually implemented with html tags
async fn prepare_msg(inp_string: &String, addr: &String) -> String {
    // "#>" followed by numbers
    let msg_link_match = Regex::new(r##"#>\d+(\.\d+)?"##).unwrap();
    // direct link to an image
    let img_link_match = Regex::new(r##"https?:\/\/.*?\.(png|gif|jpg|jpeg|webp)"##).unwrap();

    let mut result = String::new();
    let mut start_of_next: usize = 0; // start of next match
    let mut end_of_last: usize = 0; // end of previous match

    // inserting links to other messages
    let msg_matches_iter = msg_link_match.find_iter(&inp_string);
    for m_raw in msg_matches_iter {
        let m = m_raw.as_str().to_string();
        start_of_next = m_raw.start();
        let mut finished_link = String::new();
        result.push_str(&inp_string[end_of_last..start_of_next]); // text between matches

        // if it's a link to a submessage("#>dddd.dd")
        if m.contains(".") {
            let link_parts = m.split(".").collect::<Vec<&str>>();
            finished_link = format!(
                include_str!("../message_templates/msglink.html"),
                addr,
                &link_parts[0][2..],
                &link_parts[1],
                &m
            );
        } else {
            finished_link = format!(
                include_str!("../message_templates/msglink.html"),
                addr,
                &m[2..],
                "",
                &m
            );
        }
        // trimming a newline (that is there for some reason)
        result.push_str(&finished_link[..finished_link.len() - 1]);
        end_of_last = m_raw.end();
    }

    result.push_str(&inp_string[end_of_last..]);
    start_of_next = 0; // resetting for second loop
    end_of_last = 0;

    // inserting <img> tags in place of image links
    let mut second_result = String::new();
    let img_matches_iter = img_link_match.find_iter(&result);
    for m_raw in img_matches_iter {
        let m = m_raw.as_str();
        start_of_next = m_raw.start();
        second_result.push_str(&result[end_of_last..start_of_next]);
        second_result.push_str(&format!("<img class=\"userimage\" src=\"{}\">", &m));
        end_of_last = m_raw.end();
    }

    second_result.push_str(&result[end_of_last..]);
    second_result
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
            format_into_html(
                BoardMessageType::Message,
                &*server_address,
                &row.get::<usize, i64>(0),    // message id
                &get_time(row.get(1)).await,  // time of creation
                &row.get::<usize, String>(2), // author
                &prepare_msg(&row.get::<usize, String>(3), &*server_address).await, // message contents
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
    let server_address = data.server_address.lock().unwrap();
    let client = data.db_client.lock().unwrap();

    // getting time
    let since_epoch: i64 = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
    {
        Ok(n) => n.as_secs().try_into().unwrap(),
        Err(_) => 1,
    };

    // if fits, push new message into DB and vector
    if form.author.len() < 254 && form.message.len() < 4094 {
        let filtered_author = filter_string(&form.author).await;
        let filtered_msg = filter_string(&form.message).await;

        client
            .execute(
                "INSERT INTO messages(time, author, msg) VALUES (($1), ($2), ($3))",
                &[&since_epoch, &filtered_author, &filtered_msg],
            )
            .await; // i'll just pretend this `Result` doesn't exist
    }
    web::Redirect::to((*server_address).clone()).see_other() // TODO: remove clone() (if possible)
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
        head_msg = format_into_html(
            BoardMessageType::ParentMessage,
            &*server_address,
            &d.get::<usize, i64>(0),    // message id
            &get_time(d.get(1)).await,  // time of creation
            &d.get::<usize, String>(2), // author
            &prepare_msg(&d.get::<usize, String>(3), &*server_address).await, // message contents
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
            format_into_html(
                BoardMessageType::Submessage,
                &*server_address,
                &submessage_counter,          // ordinal number
                &get_time(row.get(1)).await,  // time of creation
                &row.get::<usize, String>(2), // author
                &prepare_msg(&row.get::<usize, String>(3), &*server_address).await, // message contents
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
        let filtered_author = filter_string(&form.author).await;
        let filtered_msg = filter_string(&form.message).await;
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
            "host={} user={} password={}",
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
