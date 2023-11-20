use actix_web::{get, web, App, HttpServer, HttpResponse};
use rand::Rng;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::io::Result;

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

    let mut next_id;

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

            for _ in 0..10 {
                if next_id != id {
                    break;
                }
                next_id = rng.gen_range(1..sites_count+1);
            }
        }
        _ => {
            return HttpResponse::BadRequest().body("bad action\nactions available: next, previous, random");
        }
    }

    let mut stmt = db.prepare("SELECT link, domain FROM sites WHERE id = ?1").unwrap();
    let mut rows = stmt.query([next_id]).unwrap();

    let row = rows.next().unwrap().unwrap();
    let next_url: String = row.get_unwrap(0);
    let domain: String = row.get_unwrap(1);

    println!("[{}] {} > {}", action, site, domain);

    return HttpResponse::Found().append_header(("Location", next_url.clone())).body(format!("next stop: {}", next_url));
}

#[get("/links")]
async fn links() -> HttpResponse {
    let db = Connection::open("sites.db").unwrap();
    let mut stmt = db.prepare("SELECT domain, link FROM sites").unwrap();
    let mut rows = stmt.query([]).unwrap();

    let mut sites: Vec<Value> = vec![];

    while let Some(row) = rows.next().unwrap() {
        let domain: String = row.get(0).unwrap();
        let link: String = row.get(1).unwrap();

        let site_info: Value = json!({
            "alias": domain,
            "link": link
        });

        sites.push(site_info);
    }

    return HttpResponse::Ok().json(sites);
}

#[get("/")]
async fn start_page() -> HttpResponse {
    return HttpResponse::Ok().body("im just a ring");
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("hej världen");

    let db = Connection::open("sites.db").unwrap();
    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS "sites" (
            "id"	INTEGER,
            "domain"	TEXT UNIQUE,
            "link"	TEXT,
            PRIMARY KEY("id" AUTOINCREMENT)
        );
        "#, []
    ).unwrap();

    HttpServer::new(|| {
        App::new()
            .service(action)
            .service(links)
            .service(start_page)
    })
    .bind(("0.0.0.0", 666))?
    .run()
    .await
}