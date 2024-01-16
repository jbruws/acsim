//! # ACSIM - AsynChronous Simple Imageboard
//! ACSIM is a basic imageboard engine designed to have a small codebase,
//! as well as simple configuration and deployment process.

use actix_web::{middleware, web, App, HttpServer};
use indexmap::map::IndexMap;
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslMethod};
use serde::Deserialize;
use std::fs::read_to_string;
use std::sync::Arc;
use tokio::sync::Mutex;

mod db_control;
mod html_proc;
mod routes;

/// Deserialized version of config.yaml file
#[derive(Deserialize, Clone)]
pub struct BoardConfig {
    server_ipv4: String,
    server_ipv6: String,
    server_port: u16,
    bind_to_one_ip: bool,
    use_https: bool,
    bumplimit: u16,
    hard_limit: u16,
    page_limit: u16,
    requests_limit: u16,
    log_debug_data: bool,
    site_name: String,
    site_frontend: String,
    boards: IndexMap<String, String>,
    taglines: Vec<String>,
}

fn create_ssl_acceptor() -> SslAcceptorBuilder {
    // loading ssl keys
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("./data/keys/key.pem", openssl::ssl::SslFiletype::PEM)
        .unwrap();
    builder
        .set_certificate_chain_file("./data/keys/cert.pem")
        .unwrap();
    builder
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Setting working directory
    let path_local = format!("{}/.local/share/acsim", std::env::var("HOME").unwrap());

    let acsim_dir = if std::path::Path::new("./data").exists() {
        ".".to_string()
    } else if std::path::Path::new(&path_local).exists() {
        path_local
    } else {
        panic!("Cannot locate data directory")
    };

    match std::env::set_current_dir(&std::path::Path::new(&acsim_dir)) {
        Ok(_) => println!("[PRELOG] Successfully set working directory to {}", acsim_dir),
        Err(e) => println!("[PRELOG] Failed to set working directory to {}: {}", acsim_dir, e),
    };

    // reading board config
    let raw_config: BoardConfig = serde_yaml::from_str(
        &read_to_string("./data/config.yaml")
            .unwrap_or_else(|_| panic!("Critical: can't read data/config.yaml")),
    )
    .expect("Critical: can't parse data/config.yaml");

    // starting the logger
    let logger = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} ({})] {}",
                html_proc::get_time(html_proc::since_epoch()),
                record.target(),
                message,
            ))
        })
        .level(if raw_config.log_debug_data {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        // disabling handlebars logs (they clog up the file too much)
        .level_for("handlebars", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("./data/acsim.log").unwrap())
        .apply();

    let acsim_ver = std::env::var("CARGO_PKG_VERSION");

    match logger {
        Ok(_) => match acsim_ver {
            Ok(ver) => log::info!("ACSIM v{} starting", ver),
            Err(e) => panic!(
                "Critical: failed to get ACSIM version from CARGO_PKG_VERSION: {}",
                e
            ),
        },
        Err(e) => panic!("Critical: failed to start logger: {}", e),
    };

    // loading database data from .env
    match dotenv::dotenv() {
        Ok(v) => log::info!("Loaded .env file. Path: {}", v.display()),
        Err(_) => log::error!(".env file failed to load. What happened?"),
    };

    let config = Arc::new(raw_config);
    let frontend_name: String = config.site_frontend.clone();

    // creating db connection through DatabaseWrapper
    let raw_client = db_control::DatabaseWrapper::new()
        .await
        .expect("Critical: something went wrong during database connection");
    let client = Arc::new(Mutex::new(raw_client));

    // creating html formatter
    let formatter = Arc::new(html_proc::HtmlFormatter::new(frontend_name.clone()));

    // creating application state
    let application_data = web::Data::new(routes::ApplicationState {
        db_client: Arc::clone(&client),
        formatter: Arc::clone(&formatter),
        config: Arc::clone(&config),
    });

    // rate limiting
    let governor_conf = actix_governor::GovernorConfigBuilder::default()
        .per_second(5)
        .burst_size(config.requests_limit as u32)
        .finish()
        .unwrap();

    // configuring and starting the server
    let server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .wrap(middleware::NormalizePath::trim())
            .wrap(actix_governor::Governor::new(&governor_conf))
            .app_data(application_data.clone())
            .app_data(web::PayloadConfig::new(1024 * 1024 * 100))
            .service(actix_files::Files::new(
                "/web_data",
                format!("./frontends/{}/web_data", &frontend_name.clone()),
            ))
            .service(actix_files::Files::new(
                "/user_images",
                "./data/user_images",
            ))
            .service(routes::index::root)
            .service(routes::error::error_page)
            .service(routes::board::board)
            .service(routes::board::board_process_form)
            .service(routes::topic::topic)
            .service(routes::topic::topic_process_form)
            .service(routes::catalog::board_catalog)
    });

    let mut bind_ipv4: &str = "0.0.0.0";
    let mut bind_ipv6: &str = "::1";

    if config.bind_to_one_ip {
        bind_ipv4 = &config.server_ipv4;
        bind_ipv6 = &config.server_ipv6;
    }

    if config.use_https {
        server
            .bind_openssl(
                format!("{}:{}", bind_ipv4, config.server_port).as_str(),
                create_ssl_acceptor(),
            )?
            .bind_openssl(
                format!("[{}]:{}", bind_ipv6, config.server_port).as_str(),
                create_ssl_acceptor(),
            )?
            .run()
            .await
    } else {
        server
            .bind((bind_ipv4, config.server_port))?
            .bind(format!("[{}]:{}", bind_ipv6, config.server_port).as_str())?
            .run()
            .await
    }
}
