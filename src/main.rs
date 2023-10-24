//! # ACSIM - AsynChronous Simple Imageboard
//! ACSIM is a basic imageboard engine designed to have a small codebase,
//! as well as simple configuration and deployment process.

// std
use std::collections::BTreeMap;
use std::fs::read_to_string;
use std::sync::Arc;
// actix and serde
use actix_web::{middleware::Logger, web, App, HttpServer};
use serde::Deserialize;
// async mutex
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslMethod};
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
    use_https: bool,
    bumplimit: u16,
    hard_limit: u16,
    site_name: String,
    site_frontend: String,
    page_limit: u16,
    boards: BTreeMap<String, String>,
    taglines: Vec<String>,
}

fn create_ssl_acceptor() -> SslAcceptorBuilder {
    // loading ssl keys
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("keys/key.pem", openssl::ssl::SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file("keys/cert.pem").unwrap();
    builder
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

    // configuring and starting the server
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(application_data.clone())
            .app_data(web::PayloadConfig::new(1024 * 1024 * 100))
            .service(actix_files::Files::new(
                "/web_data",
                format!("./frontends/{}/web_data", &frontend_name.clone()),
            ))
            .service(actix_files::Files::new("/user_images", "./user_images"))
            .service(routes::root)
            .service(routes::board)
            .service(routes::board_process_form)
            .service(routes::topic)
            .service(routes::topic_process_form)
    });

    let mut bind_ipv4: &str = "0.0.0.0";
    let mut bind_ipv6: &str = "::1";

    if config.bind_to_one_ip {
        bind_ipv4 = &config.server_ipv4;
        bind_ipv6 = &config.server_ipv6;
    }

    if config.use_https {
        return server
            .bind_openssl(
                format!("{}:{}", bind_ipv4, config.server_port).as_str(),
                create_ssl_acceptor(),
            )?
            .bind_openssl(
                format!("[{}]:{}", bind_ipv6, config.server_port).as_str(),
                create_ssl_acceptor(),
            )?.run().await;
    } else {
        return server
            .bind((bind_ipv4, config.server_port))?
            .bind(format!("[{}]:{}", bind_ipv6, config.server_port).as_str())?.run().await;
    }
}
