use actix_web::{get, web, App, HttpServer, HttpResponse};
use rand::Rng;
use rusqlite::Connection;

use std::{io::Result, collections::HashMap};

#[get("/{name}/{action}")]
async fn action(param: web::Path<(String, String)>) -> HttpResponse {
    let (site, action) = param.into_inner();

    let db = Connection::open("sites.db").unwrap();

    let mut stmt = db.prepare("SELECT COUNT(1) FROM sites").unwrap();
    let mut rows = stmt.query([]).unwrap();
    let sites_count: usize = rows.next().unwrap().unwrap().get_unwrap(0);

    let mut stmt = db.prepare("SELECT id FROM sites WHERE domain = ?1").unwrap();
    let mut rows = stmt.query([&site]).unwrap();

    let id: usize = match rows.next().unwrap() {
        Some(row) => row.get_unwrap(0),
        None => {
            return HttpResponse::NotFound().body(format!("site {:?} does not exists", site));
        }
    };

    let next_id;

    match action.as_str() {
        "next" => {
            next_id = match sites_count > id as usize {
                true => id + 1,
                false => 1,
            };
        }
        "previous" => {
            next_id = match id as usize > 1 {
                true => id - 1,
                false => sites_count,
            };
        }
        "random" => {
            let mut rng = rand::thread_rng();
            next_id = rng.gen_range(1..sites_count+1);
        }
        _ => {
            return HttpResponse::BadRequest().body("bad action");
        }
    }

    let mut stmt = db.prepare("SELECT link FROM sites WHERE id = ?1").unwrap();
    let mut rows = stmt.query([next_id]).unwrap();

    let next_url: String = rows.next().unwrap().unwrap().get_unwrap(0);

    return HttpResponse::Found().append_header(("Location", next_url.clone())).body(format!("next stop: {}", next_url));
}

#[get("/links")]
async fn links() -> HttpResponse {
    let db = Connection::open("sites.db").unwrap();
    let mut stmt = db.prepare("SELECT id, domain, link FROM sites").unwrap();
    let mut rows = stmt.query([]).unwrap();

    let mut sites: HashMap<usize, (String, String)> = HashMap::new();

    while let Some(row) = rows.next().unwrap() {
        let id: usize = row.get(0).unwrap();
        let domain: String = row.get(1).unwrap();
        let link: String = row.get(2).unwrap();
        sites.insert(id, (domain, link));
    }

    return HttpResponse::Ok().json(sites);
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = Connection::open("sites.db").unwrap();
    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS "sites" (
            "id"	INTEGER,
            "domain"	TEXT UNIQUE,
            "https"	INTEGER,
            PRIMARY KEY("id" AUTOINCREMENT)
        );
        "#, []
    ).unwrap();

    HttpServer::new(|| {
        App::new()
            .service(action)
            .service(links)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}