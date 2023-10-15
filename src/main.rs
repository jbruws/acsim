//! # ACSIM - AsynChronous Simple Imageboard
//! ACSIM is a basic imageboard engine designed to have a small codebase,
//! as well as simple configuration and deployment process.

// std
use std::collections::HashMap;
use std::fs::read_to_string;
use std::sync::Arc;
// actix and serde
use actix_web::{middleware::Logger, web, App, HttpServer};
use serde::Deserialize;
// async mutex
use tokio::sync::Mutex;

mod db_control;
mod html_proc;
mod routes;

/// Deserialized version of config.json file
#[derive(Deserialize, Clone)]
pub struct BoardConfig {
    db_host: String,
    db_user: String,
    db_password: String,
    server_ipv4: String,
    server_ipv6: String,
    server_port: u16,
    bind_to_one_ip: bool,
    bumplimit: u16,
    hard_limit: u16,
    site_name: String,
    site_frontend: String,
    page_limit: u16,
    boards: HashMap<String, String>,
    taglines: Vec<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // reading board config
    let raw_config: BoardConfig = serde_yaml::from_str(
        &read_to_string("./config.yaml")
            .unwrap_or_else(|_| panic!("Critical: can't read config.yaml")),
    )
    .expect("Critical: can't parse config.yaml");

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
        config: Arc::clone(&config),
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

    // starting the server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(application_data.clone())
            .service(actix_files::Files::new(
                "/web_data",
                format!("./frontends/{}/web_data", &frontend_name.clone()),
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
