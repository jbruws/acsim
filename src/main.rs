//! # ACSIM - AsynChronous Simple Imageboard
//! ACSIM is a basic imageboard engine designed to have a small codebase,
//! as well as simple configuration and deployment process.

// std
use std::collections::HashMap;
use std::sync::Arc;
// actix and serde
use actix_web::{middleware::Logger, web, App, HttpServer};
use serde::{Deserialize, Serialize};
// async mutex
use tokio::sync::Mutex;

mod db_control;
mod html_proc;
mod routes;

/// Deserialized version of config.json file
#[derive(Serialize, Deserialize, Clone)]
pub struct BoardConfig {
    db_host: String,
    db_user: String,
    db_password: String,
    server_ipv4: String,
    server_ipv6: String,
    server_port: u16,
    bind_to_one_ip: bool,
    deletion_timer: u16,
    bumplimit: u16,
    soft_limit: u16,
    hard_limit: u16,
    site_name: String,
    site_frontend: String,
    page_limit: u16,
    boards: HashMap<String, String>,
    taglines: Vec<String>,
}

// this function does not yet work, since i somehow need to pass Client
// to the function (to make queries work) without cloning it
/// Soft limit message deletion timer (currently unused)
async fn deletion_loop(client: Arc<Mutex<db_control::DatabaseWrapper>>, config: Arc<BoardConfig>) {
    loop {
        // interval between deletion attempts
        tokio::time::sleep(tokio::time::Duration::from_secs(
            config.deletion_timer.into(),
        ))
        .await;

        // looking across all boards
        let cl = client.lock().await;
        for i in config.boards.keys() {
            let msg_count = cl.count_messages(i).await.unwrap();
            if msg_count > config.soft_limit.into() {
                cl.delete_least_active(i).await;
            }
        }
        drop(cl);
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // reading board config
    let raw_config: BoardConfig =
        serde_json::from_str(include_str!("../config.json")).expect("Can't parse config.json");

    let config = Arc::new(raw_config);
    let frontend_name: String = config.site_frontend.clone();

    // creating db connection through DatabaseWrapper
    let raw_client =
        db_control::DatabaseWrapper::new(&config.db_host, &config.db_user, &config.db_password)
            .await;
    let client = Arc::new(Mutex::new(raw_client));

    // creating html formatter
    let formatter = Arc::new(html_proc::HtmlFormatter::new(frontend_name.clone()));

    // selecting where to bind the server
    let mut bound_ipv4 = "0.0.0.0";
    let mut bound_ipv6 = "::1";
    if config.bind_to_one_ip {
        bound_ipv4 = config.server_ipv4.as_str();
        bound_ipv6 = config.server_ipv6.as_str();
    }

    // creating application state
    let application_data = web::Data::new(routes::ApplicationState {
        db_client: Arc::clone(&client),
        formatter: Arc::clone(&formatter),
        config: Arc::clone(&config), // alas, we must clone here
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
        .chain(fern::log_file("./acsim.log").unwrap())
        .apply();
    match logger {
        Ok(_) => log::info!("ACSIM starting"),
        Err(e) => println!("WARNING: Failed to start logger: {}", e),
    };

    // start soft limit message deletion loop
    //thread::spawn(|| {
    //    tokio::task::spawn(async { deletion_loop(Arc::clone(&client), Arc::clone(&config)).await })
    //})
    //.join();

    // starting the server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(application_data.clone())
            .service(actix_files::Files::new(
                "/web-data",
                format!("./frontends/{}/web-data", &frontend_name.clone()),
            ))
            .service(actix_files::Files::new("/user_images", "./user_images"))
            .service(routes::root)
            .service(routes::message_page)
            .service(routes::process_form)
            .service(routes::process_submessage_form)
            .service(routes::main_page)
    })
    .bind((bound_ipv4, config.server_port))?
    .bind(format!("[{}]:{}", bound_ipv6, config.server_port).as_str())?
    .run()
    .await
}
