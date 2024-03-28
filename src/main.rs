use actix_web::{get, web, App, HttpResponse, HttpServer};
use rand::Rng;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{io::Result, time::{SystemTime, UNIX_EPOCH}};

const HOST_URL: &str = "http://localhost:666";//"https://ring.reez.it";
const CHECK_INTERVAL: u128 = 60_1000 * 60;

#[get("/{name}/{action}")]
async fn action(param: web::Path<(String, String)>) -> HttpResponse {
    let (start_site, action) = param.into_inner();

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
    match action.as_str() {
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
    let end_site: String = row.get_unwrap(0);
    let end_url: String = row.get_unwrap(1);
    let end_down: bool = row.get_unwrap::<usize, String>(2).parse().unwrap();
    let end_last_checked: u128 = row.get_unwrap::<usize, String>(3).parse::<u128>().unwrap();

    
    //check if site is alive
    println!("[{}] {} > {}", action, start_site, end_site);
    
    let current_unix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let mut stmt = db.prepare("INSERT INTO logs ('from', 'to', 'using', 'timestamp') VALUES (?1, ?2, ?3, ?4)").unwrap();
    stmt.execute([
        start_site, 
        end_site.clone(), 
        action.clone(), 
        current_unix.to_string()
    ]).unwrap();

    //hack: pls fix better solution
    let update_site = end_site.clone();
    let update_url = end_url.clone();
    tokio::spawn(async move {
        if current_unix > end_last_checked + CHECK_INTERVAL {
            let updated_down = match reqwest::get(&update_url).await {
                Ok(response) => {
                    !response.status().is_success()
                },
                Err(_) => {
                    true
                },
            };
    
            let db = Connection::open("sites.db").unwrap();
    
            let mut stmt = db.prepare("UPDATE sites SET 'down' = ?1, 'last_checked' = ?2 WHERE site = ?3").unwrap();
            stmt.execute([
                updated_down.to_string(), 
                current_unix.to_string(),
                update_site.clone()
            ]).unwrap();
        }
    });

    if end_down {
        return HttpResponse::Found()
            .append_header(("Location", format!("{}/{}/{}", HOST_URL, &end_site, &action)))
            .body(format!("{} down...", &end_site));
    }

    return HttpResponse::Found().append_header(("Location", end_url.clone())).body(format!("next stop: {}", end_url));
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