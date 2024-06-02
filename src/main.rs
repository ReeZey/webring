use actix_web::{get, web, App, HttpResponse, HttpServer};
use rand::Rng;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{io::Result, time::{SystemTime, UNIX_EPOCH}};
use chrono::prelude::*;

const HOST_URL: &str = "https://ring.reez.it";
const CHECK_INTERVAL: u128 = 1000 * 60 * 60; //once every hour

#[get("/{name}/{action}")]
async fn action(param: web::Path<(String, String)>) -> HttpResponse {
    let (start_site, site_action) = param.into_inner();

    let db = Connection::open("sites.db").unwrap();

    let mut stmt = db.prepare("SELECT COUNT(1) FROM sites").unwrap();
    let mut rows = stmt.query([]).unwrap();
    let sites_count: usize = rows.next().unwrap().unwrap().get_unwrap(0);

    let mut stmt = db.prepare("SELECT id FROM sites WHERE site = ?1").unwrap();
    let mut rows = stmt.query([&start_site]).unwrap();

    let current_id: usize = match rows.next().unwrap() {
        Some(row) => row.get_unwrap(0),
        None => {
            return HttpResponse::NotFound().body(format!("site {:?} does not exists", start_site));
        }
    };

    //do them actions baby
    let mut next_id;
    match site_action.as_str() {
        "next" => {
            next_id = match sites_count > current_id as usize {
                true => current_id + 1,
                false => 1,
            };
        }
        "previous" => {
            next_id = match current_id as usize > 1 {
                true => current_id - 1,
                false => sites_count,
            };
        }
        "random" => {
            let mut rng = rand::thread_rng();

            next_id = rng.gen_range(1..sites_count+1);

            if next_id == current_id {
                next_id += 1;
            }

            if next_id > sites_count {
                next_id = 1;
            }
        }
        _ => {
            return HttpResponse::BadRequest().body("bad action\nactions available: next, previous, random");
        }
    }

    let mut stmt = db.prepare("SELECT site, url, down, last_checked FROM sites WHERE id = ?1").unwrap();
    let mut query = stmt.query([next_id]).unwrap();

    let row = query.next().unwrap().unwrap();
    let target_site: String = row.get_unwrap(0);
    let target_url: String = row.get_unwrap(1);
    let mut target_down: bool = row.get_unwrap::<usize, String>(2).parse().unwrap();
    let target_last_check: u128 = row.get_unwrap::<usize, String>(3).parse::<u128>().unwrap();

    let now = Utc::now();
    //check if site is alive
    println!("[{}] {} > {} [{}]", now.format("%Y-%m-%d %H:%M:%S"), start_site, target_site, site_action);

    let current_unix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

    let current_unix_copy = current_unix.clone();
    let target_site_copy = target_site.clone();
    let site_action_copy = site_action.clone();

    let mut stmt = db.prepare("INSERT INTO logs ('from', 'to', 'using', 'timestamp') VALUES (?1, ?2, ?3, ?4)").unwrap();
    stmt.execute([
        start_site, 
        target_site_copy.clone(), 
        site_action_copy.clone(), 
        current_unix_copy.to_string()
    ]).unwrap();

    if current_unix > target_last_check + CHECK_INTERVAL {
        let is_down = match reqwest::get(&target_url).await {
            Ok(response) => {
                !response.status().is_success()
            },
            Err(_) => {
                true
            },
        };

        if is_down != target_down {
            let mut stmt = db.prepare("UPDATE sites SET 'down' = ?1, 'last_checked' = ?2 WHERE site = ?3").unwrap();
            stmt.execute([
                is_down.to_string(), 
                current_unix.to_string(),
                target_site.clone()
            ]).unwrap();

            target_down = is_down;
        }
    }

    if target_down {
        println!("{} is down", target_site);

        return HttpResponse::Found()
            .append_header(("Location", format!("{}/{}/{}", HOST_URL, &target_site, &site_action)))
            .body(format!("{} down...", &target_site));
    }

    return HttpResponse::Found().append_header(("Location", target_url.clone())).body(format!("next stop: {}", target_url));
}

#[get("/links")]
async fn links() -> HttpResponse {
    let db = Connection::open("sites.db").unwrap();
    let mut stmt = db.prepare("SELECT site, url, down, last_checked FROM sites").unwrap();
    
    let sites: Vec<Value> = stmt.query_map([], |a| {
        Ok(json!({
            "site": a.get_unwrap::<usize, String>(0),
            "url": a.get_unwrap::<usize, String>(1),
            "down": a.get_unwrap::<usize, String>(2).parse::<bool>().unwrap(),
            "last_checked": a.get_unwrap::<usize, String>(3).parse::<u128>().unwrap(),
        }))
    }).unwrap().filter_map(|a| a.ok()).collect();

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
            "id"	        INTEGER,
            "site"	        TEXT UNIQUE,
            "url"	        TEXT,
            "down"          INTEGER,
            "last_checked"  TEXT,
            PRIMARY KEY("id" AUTOINCREMENT)
        );
        "#, []
    ).unwrap();

    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS "logs" (
            "id"	        INTEGER,
            "from"	        TEXT,
            "to"	        TEXT,
            "using"         TEXT,
            "timestamp"     INTEGER,
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