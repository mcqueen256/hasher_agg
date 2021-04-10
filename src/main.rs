mod app;
mod constants;
mod packets;
mod routes;
mod util;
mod file_operations;
mod submitter;

use crate::routes::{boot, index, job_request, job_submit, pool_status, showdown};
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use std::sync::{Arc, Mutex};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // std::env::set_var("RUST_LOG", "actix_web=info");
    std::env::set_var("RUST_LOG", "actix_web=debug");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    let data = Arc::new(Mutex::new(app::ApplicationData::begin()));
    let app = Arc::clone(&data);
    let server = HttpServer::new(move || {
        App::new()
            .data(Arc::clone(&app))
            .service(index)
            .service(boot)
            .service(showdown)
            .service(job_request)
            .service(job_submit)
            .service(pool_status)
            .wrap(Logger::default())
    })
    .bind(("0.0.0.0", 9876))?;
    let and = server.run();
    and.await
}
