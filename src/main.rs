use actix_web::{get, web, App, HttpServer, HttpResponse};
use rusqlite::Connection;
use std::io::Result;

#[get("/{name}/{action}")]
async fn action(param: web::Path<(String, String)>) -> HttpResponse {
    let (site, action) = param.into_inner();

    match action.as_str() {
        "next" => {
            return HttpResponse::Ok().body("next");
        }
        "previous" => {
            return HttpResponse::Ok().body("previous");
        }
        "random" => {
            return HttpResponse::Ok().body("randomooomm");
        }
        _ => {
            return HttpResponse::BadRequest().body("bad action");
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = Connection::open("sites.db").unwrap();

    HttpServer::new(|| {
        App::new().service(action)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}