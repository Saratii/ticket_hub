use actix_cors::Cors;
use actix_web::{delete, get, post, put, web, App, HttpResponse, HttpServer, Responder};
use bcrypt::{hash, DEFAULT_COST};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;

struct AppState {
    db: Mutex<Connection>,
}

#[derive(Serialize, Deserialize, Debug)]
struct User {
    user_id: String,
    username: String,
    email: String,
    phone: Option<String>,
    creation_time: String,
}

#[derive(Deserialize)]
struct CreateUser {
    username: String,
    email: String,
    password: String,
    phone: Option<String>,
}

#[derive(Deserialize)]
struct UpdateUser {
    username: Option<String>,
    email: Option<String>,
    phone: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Artist {
    artist_id: String,
    name: String,
    genre: Option<String>,
    bio: Option<String>,
}

#[derive(Deserialize)]
struct CreateArtist {
    name: String,
    genre: Option<String>,
    bio: Option<String>,
}

#[derive(Deserialize)]
struct UpdateArtist {
    genre: Option<String>,
    bio: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Event {
    event_id: String,
    artist_id: String,
    artist_name: Option<String>,
    venue_name: String,
    city: String,
    state: String,
    event_date: String,
    capacity: i64,
}

#[derive(Deserialize)]
struct CreateEvent {
    artist_id: String,
    venue_name: String,
    city: String,
    state: String,
    event_date: String,
    capacity: i64,
}

#[derive(Deserialize)]
struct EventSearch {
    artist: Option<String>,
    city: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Ticket {
    ticket_id: String,
    event_id: String,
    event_name: Option<String>,
    price: f64,
    num_available: i64,
}

#[derive(Deserialize)]
struct CreateTicket {
    event_id: String,
    price: f64,
    num_available: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Order {
    order_id: String,
    user_id: String,
    ticket_id: String,
    date_of_purchase: String,
    event_name: Option<String>,
    venue_name: Option<String>,
    city: Option<String>,
    state: Option<String>,
    event_date: Option<String>,
    price: Option<f64>,
    artist_name: Option<String>,
}

#[derive(Deserialize)]
struct CreateOrder {
    user_id: String,
    ticket_id: String,
}

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

fn ok<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(ApiResponse { success: true, data: Some(data), message: None })
}

fn err_resp(msg: &str) -> HttpResponse {
    HttpResponse::BadRequest().json(ApiResponse::<()> { success: false, data: None, message: Some(msg.to_string()) })
}

fn not_found(msg: &str) -> HttpResponse {
    HttpResponse::NotFound().json(ApiResponse::<()> { success: false, data: None, message: Some(msg.to_string()) })
}

fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS users (
            user_id TEXT PRIMARY KEY NOT NULL,
            username TEXT UNIQUE NOT NULL,
            email TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            phone TEXT,
            creation_time TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS artists (
            artist_id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            genre TEXT,
            bio TEXT
        );
        CREATE TABLE IF NOT EXISTS events (
            event_id TEXT PRIMARY KEY NOT NULL,
            artist_id TEXT NOT NULL,
            venue_name TEXT NOT NULL,
            city TEXT NOT NULL,
            state TEXT NOT NULL,
            event_date TEXT NOT NULL,
            capacity INTEGER NOT NULL CHECK(capacity > 0),
            FOREIGN KEY (artist_id) REFERENCES artists(artist_id)
        );
        CREATE TABLE IF NOT EXISTS tickets (
            ticket_id TEXT PRIMARY KEY NOT NULL,
            event_id TEXT NOT NULL,
            price REAL NOT NULL CHECK(price > 0),
            num_available INTEGER NOT NULL CHECK(num_available >= 0),
            FOREIGN KEY (event_id) REFERENCES events(event_id)
        );
        CREATE TABLE IF NOT EXISTS orders (
            order_id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            ticket_id TEXT NOT NULL,
            date_of_purchase TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(user_id),
            FOREIGN KEY (ticket_id) REFERENCES tickets(ticket_id)
        );
    ")?;
    seed_db(conn)?;
    Ok(())
}

fn seed_db(conn: &Connection) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM artists", [], |r| r.get(0))?;
    if count > 0 { return Ok(()); }

    let artists = vec![
        ("a1", "Taylor Swift", "Pop", "Grammy-winning global pop superstar known for era-defining albums."),
        ("a2", "Metallica", "Metal", "Iconic heavy metal band formed in 1981 with decades of sold-out tours."),
        ("a3", "Billie Eilish", "Alternative Pop", "Whisper-pop sensation, multi-Grammy winner and Oscar winner."),
        ("a4", "Bad Bunny", "Reggaeton", "Latin trap and reggaeton global phenomenon from Puerto Rico."),
        ("a5", "The Weeknd", "R&B", "Critically acclaimed R&B and pop artist known for After Hours era."),
    ];
    for (id, name, genre, bio) in &artists {
        conn.execute("INSERT OR IGNORE INTO artists VALUES (?1,?2,?3,?4)", params![id, name, genre, bio])?;
    }

    let events = vec![
        ("e1","a1","Arrowhead Stadium","Kansas City","MO","2025-08-15",70000i64),
        ("e2","a1","Soldier Field","Chicago","IL","2025-08-22",61500),
        ("e3","a2","Madison Square Garden","New York","NY","2025-09-10",20789),
        ("e4","a3","Hollywood Bowl","Los Angeles","CA","2025-07-30",17500),
        ("e5","a4","American Airlines Arena","Miami","FL","2025-10-05",19600),
        ("e6","a5","Chase Center","San Francisco","CA","2025-11-12",18064),
        ("e7","a2","United Center","Chicago","IL","2025-09-20",20917),
        ("e8","a3","Red Rocks Amphitheatre","Morrison","CO","2025-08-05",9525),
    ];
    for (id, artist, venue, city, state, date, cap) in &events {
        conn.execute("INSERT OR IGNORE INTO events VALUES (?1,?2,?3,?4,?5,?6,?7)", params![id, artist, venue, city, state, date, cap])?;
    }

    let tickets: Vec<(&str, &str, f64, i64)> = vec![
        ("t1","e1",89.99,500),("t2","e1",149.99,200),("t3","e2",99.99,350),
        ("t4","e3",79.99,150),("t5","e4",65.00,400),("t6","e5",110.00,300),
        ("t7","e6",120.00,250),("t8","e7",85.00,600),("t9","e8",55.00,800),
    ];
    for (id, event, price, avail) in &tickets {
        conn.execute("INSERT OR IGNORE INTO tickets VALUES (?1,?2,?3,?4)", params![id, event, price, avail])?;
    }

    let pw_hash = hash("password123", DEFAULT_COST).unwrap_or_default();
    conn.execute(
        "INSERT OR IGNORE INTO users VALUES ('u1','demo_user','demo@tickethub.com',?1,'555-0100','2024-01-01T00:00:00')",
        params![pw_hash],
    )?;
    Ok(())
}

// ── Users ──────────────────────────────────────────────────────────────────────

#[post("/api/users")]
async fn create_user(data: web::Data<AppState>, body: web::Json<CreateUser>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string();
    let hashed = match hash(&body.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return err_resp("Failed to hash password"),
    };
    match db.execute("INSERT INTO users VALUES (?1,?2,?3,?4,?5,?6)",
        params![id, body.username, body.email, hashed, body.phone, now]) {
        Ok(_) => ok(serde_json::json!({ "user_id": id })),
        Err(e) => err_resp(&format!("Error: {}", e)),
    }
}

#[get("/api/users")]
async fn list_users(data: web::Data<AppState>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT user_id,username,email,phone,creation_time FROM users ORDER BY creation_time DESC").unwrap();
    let users: Vec<User> = stmt.query_map([], |r| Ok(User {
        user_id: r.get(0)?, username: r.get(1)?, email: r.get(2)?, phone: r.get(3)?, creation_time: r.get(4)?,
    })).unwrap().filter_map(|u| u.ok()).collect();
    ok(users)
}

#[get("/api/users/{id}")]
async fn get_user(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    match db.query_row("SELECT user_id,username,email,phone,creation_time FROM users WHERE user_id=?1",
        params![id], |r| Ok(User { user_id: r.get(0)?, username: r.get(1)?, email: r.get(2)?, phone: r.get(3)?, creation_time: r.get(4)? })) {
        Ok(u) => ok(u),
        Err(_) => not_found("User not found"),
    }
}

#[put("/api/users/{id}")]
async fn update_user(data: web::Data<AppState>, path: web::Path<String>, body: web::Json<UpdateUser>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    let mut n = 0usize;
    if let Some(ref v) = body.username { n += db.execute("UPDATE users SET username=?1 WHERE user_id=?2", params![v, id]).unwrap_or(0); }
    if let Some(ref v) = body.email { n += db.execute("UPDATE users SET email=?1 WHERE user_id=?2", params![v, id]).unwrap_or(0); }
    if let Some(ref v) = body.phone { n += db.execute("UPDATE users SET phone=?1 WHERE user_id=?2", params![v, id]).unwrap_or(0); }
    if n == 0 { return err_resp("Nothing updated"); }
    ok(serde_json::json!({ "message": "User updated" }))
}

#[delete("/api/users/{id}")]
async fn delete_user(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    match db.execute("DELETE FROM users WHERE user_id=?1", params![id]) {
        Ok(1) => ok(serde_json::json!({ "message": "Deleted" })),
        _ => not_found("User not found"),
    }
}

#[get("/api/users/{id}/orders")]
async fn user_orders(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    let mut stmt = db.prepare(
        "SELECT o.order_id,o.user_id,o.ticket_id,o.date_of_purchase,NULL,e.venue_name,e.city,e.state,e.event_date,t.price,a.name
         FROM orders o JOIN tickets t ON o.ticket_id=t.ticket_id JOIN events e ON t.event_id=e.event_id JOIN artists a ON e.artist_id=a.artist_id
         WHERE o.user_id=?1 ORDER BY o.date_of_purchase DESC"
    ).unwrap();
    let orders: Vec<Order> = stmt.query_map(params![id], |r| Ok(Order {
        order_id: r.get(0)?, user_id: r.get(1)?, ticket_id: r.get(2)?, date_of_purchase: r.get(3)?,
        event_name: r.get(4)?, venue_name: r.get(5)?, city: r.get(6)?, state: r.get(7)?, event_date: r.get(8)?,
        price: r.get(9)?, artist_name: r.get(10)?,
    })).unwrap().filter_map(|o| o.ok()).collect();
    ok(orders)
}

// ── Artists ────────────────────────────────────────────────────────────────────

#[post("/api/artists")]
async fn create_artist(data: web::Data<AppState>, body: web::Json<CreateArtist>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    match db.execute("INSERT INTO artists VALUES (?1,?2,?3,?4)", params![id, body.name, body.genre, body.bio]) {
        Ok(_) => ok(serde_json::json!({ "artist_id": id })),
        Err(e) => err_resp(&format!("Error: {}", e)),
    }
}

#[get("/api/artists")]
async fn list_artists(data: web::Data<AppState>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT artist_id,name,genre,bio FROM artists ORDER BY name").unwrap();
    let artists: Vec<Artist> = stmt.query_map([], |r| Ok(Artist { artist_id: r.get(0)?, name: r.get(1)?, genre: r.get(2)?, bio: r.get(3)? }))
        .unwrap().filter_map(|a| a.ok()).collect();
    ok(artists)
}

#[get("/api/artists/{id}")]
async fn get_artist(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    match db.query_row("SELECT artist_id,name,genre,bio FROM artists WHERE artist_id=?1", params![id],
        |r| Ok(Artist { artist_id: r.get(0)?, name: r.get(1)?, genre: r.get(2)?, bio: r.get(3)? })) {
        Ok(a) => ok(a),
        Err(_) => not_found("Artist not found"),
    }
}

#[put("/api/artists/{id}")]
async fn update_artist(data: web::Data<AppState>, path: web::Path<String>, body: web::Json<UpdateArtist>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    let mut n = 0usize;
    if let Some(ref v) = body.genre { n += db.execute("UPDATE artists SET genre=?1 WHERE artist_id=?2", params![v, id]).unwrap_or(0); }
    if let Some(ref v) = body.bio { n += db.execute("UPDATE artists SET bio=?1 WHERE artist_id=?2", params![v, id]).unwrap_or(0); }
    if n == 0 { return err_resp("Nothing updated"); }
    ok(serde_json::json!({ "message": "Artist updated" }))
}

#[delete("/api/artists/{id}")]
async fn delete_artist(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    match db.execute("DELETE FROM artists WHERE artist_id=?1", params![id]) {
        Ok(1) => ok(serde_json::json!({ "message": "Deleted" })),
        _ => not_found("Artist not found"),
    }
}

#[get("/api/artists/{id}/events")]
async fn artist_events(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    let mut stmt = db.prepare(
        "SELECT e.event_id,e.artist_id,a.name,e.venue_name,e.city,e.state,e.event_date,e.capacity
         FROM events e JOIN artists a ON e.artist_id=a.artist_id WHERE e.artist_id=?1 ORDER BY e.event_date"
    ).unwrap();
    let events: Vec<Event> = stmt.query_map(params![id], |r| Ok(Event {
        event_id: r.get(0)?, artist_id: r.get(1)?, artist_name: r.get(2)?, venue_name: r.get(3)?,
        city: r.get(4)?, state: r.get(5)?, event_date: r.get(6)?, capacity: r.get(7)?,
    })).unwrap().filter_map(|e| e.ok()).collect();
    ok(events)
}

// ── Events ─────────────────────────────────────────────────────────────────────

#[post("/api/events")]
async fn create_event(data: web::Data<AppState>, body: web::Json<CreateEvent>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    match db.execute("INSERT INTO events VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![id, body.artist_id, body.venue_name, body.city, body.state, body.event_date, body.capacity]) {
        Ok(_) => ok(serde_json::json!({ "event_id": id })),
        Err(e) => err_resp(&format!("Error: {}", e)),
    }
}

#[get("/api/events")]
async fn list_events(data: web::Data<AppState>, query: web::Query<EventSearch>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let mut sql = String::from(
        "SELECT e.event_id,e.artist_id,a.name,e.venue_name,e.city,e.state,e.event_date,e.capacity
         FROM events e JOIN artists a ON e.artist_id=a.artist_id WHERE 1=1"
    );
    let mut args: Vec<String> = vec![];
    if let Some(ref v) = query.artist {
        args.push(v.clone());
        sql.push_str(&format!(" AND LOWER(a.name) LIKE LOWER('%'||?{}||'%')", args.len()));
    }
    if let Some(ref v) = query.city {
        args.push(v.clone());
        sql.push_str(&format!(" AND LOWER(e.city) LIKE LOWER('%'||?{}||'%')", args.len()));
    }
    if let Some(ref v) = query.date_from {
        args.push(v.clone());
        sql.push_str(&format!(" AND e.event_date >= ?{}", args.len()));
    }
    if let Some(ref v) = query.date_to {
        args.push(v.clone());
        sql.push_str(&format!(" AND e.event_date <= ?{}", args.len()));
    }
    sql.push_str(" ORDER BY e.event_date");
    let mut stmt = db.prepare(&sql).unwrap();
    let events: Vec<Event> = stmt.query_map(rusqlite::params_from_iter(args.iter()), |r| Ok(Event {
        event_id: r.get(0)?, artist_id: r.get(1)?, artist_name: r.get(2)?, venue_name: r.get(3)?,
        city: r.get(4)?, state: r.get(5)?, event_date: r.get(6)?, capacity: r.get(7)?,
    })).unwrap().filter_map(|e| e.ok()).collect();
    ok(events)
}

#[get("/api/events/{id}")]
async fn get_event(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    match db.query_row(
        "SELECT e.event_id,e.artist_id,a.name,e.venue_name,e.city,e.state,e.event_date,e.capacity
         FROM events e JOIN artists a ON e.artist_id=a.artist_id WHERE e.event_id=?1",
        params![id], |r| Ok(Event {
            event_id: r.get(0)?, artist_id: r.get(1)?, artist_name: r.get(2)?, venue_name: r.get(3)?,
            city: r.get(4)?, state: r.get(5)?, event_date: r.get(6)?, capacity: r.get(7)?,
        })) {
        Ok(e) => ok(e),
        Err(_) => not_found("Event not found"),
    }
}

#[delete("/api/events/{id}")]
async fn delete_event(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    match db.execute("DELETE FROM events WHERE event_id=?1", params![id]) {
        Ok(1) => ok(serde_json::json!({ "message": "Deleted" })),
        _ => not_found("Event not found"),
    }
}

// ── Tickets ────────────────────────────────────────────────────────────────────

#[post("/api/tickets")]
async fn create_ticket(data: web::Data<AppState>, body: web::Json<CreateTicket>) -> impl Responder {
    if body.price <= 0.0 { return err_resp("Price must be positive"); }
    if body.num_available <= 0 { return err_resp("num_available must be positive"); }
    let db = data.db.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    match db.execute("INSERT INTO tickets VALUES (?1,?2,?3,?4)", params![id, body.event_id, body.price, body.num_available]) {
        Ok(_) => ok(serde_json::json!({ "ticket_id": id })),
        Err(e) => err_resp(&format!("Error: {}", e)),
    }
}

#[get("/api/tickets")]
async fn list_tickets(data: web::Data<AppState>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT t.ticket_id,t.event_id,e.venue_name||' — '||a.name,t.price,t.num_available
         FROM tickets t JOIN events e ON t.event_id=e.event_id JOIN artists a ON e.artist_id=a.artist_id ORDER BY t.price"
    ).unwrap();
    let tickets: Vec<Ticket> = stmt.query_map([], |r| Ok(Ticket {
        ticket_id: r.get(0)?, event_id: r.get(1)?, event_name: r.get(2)?, price: r.get(3)?, num_available: r.get(4)?,
    })).unwrap().filter_map(|t| t.ok()).collect();
    ok(tickets)
}

#[get("/api/events/{id}/tickets")]
async fn event_tickets(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    let mut stmt = db.prepare("SELECT ticket_id,event_id,NULL,price,num_available FROM tickets WHERE event_id=?1").unwrap();
    let tickets: Vec<Ticket> = stmt.query_map(params![id], |r| Ok(Ticket {
        ticket_id: r.get(0)?, event_id: r.get(1)?, event_name: r.get(2)?, price: r.get(3)?, num_available: r.get(4)?,
    })).unwrap().filter_map(|t| t.ok()).collect();
    ok(tickets)
}

// ── Orders ─────────────────────────────────────────────────────────────────────

#[post("/api/orders")]
async fn create_order(data: web::Data<AppState>, body: web::Json<CreateOrder>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let avail: i64 = match db.query_row("SELECT num_available FROM tickets WHERE ticket_id=?1", params![body.ticket_id], |r| r.get(0)) {
        Ok(n) => n,
        Err(_) => return not_found("Ticket not found"),
    };
    if avail <= 0 { return err_resp("No tickets available"); }
    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string();
    match db.execute("INSERT INTO orders VALUES (?1,?2,?3,?4)", params![id, body.user_id, body.ticket_id, now]) {
        Ok(_) => {
            let _ = db.execute("UPDATE tickets SET num_available=num_available-1 WHERE ticket_id=?1", params![body.ticket_id]);
            ok(serde_json::json!({ "order_id": id, "date_of_purchase": now }))
        }
        Err(e) => err_resp(&format!("Error: {}", e)),
    }
}

#[get("/api/orders")]
async fn list_orders(data: web::Data<AppState>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT o.order_id,o.user_id,o.ticket_id,o.date_of_purchase,NULL,e.venue_name,e.city,e.state,e.event_date,t.price,a.name
         FROM orders o JOIN tickets t ON o.ticket_id=t.ticket_id JOIN events e ON t.event_id=e.event_id JOIN artists a ON e.artist_id=a.artist_id
         ORDER BY o.date_of_purchase DESC"
    ).unwrap();
    let orders: Vec<Order> = stmt.query_map([], |r| Ok(Order {
        order_id: r.get(0)?, user_id: r.get(1)?, ticket_id: r.get(2)?, date_of_purchase: r.get(3)?,
        event_name: r.get(4)?, venue_name: r.get(5)?, city: r.get(6)?, state: r.get(7)?, event_date: r.get(8)?,
        price: r.get(9)?, artist_name: r.get(10)?,
    })).unwrap().filter_map(|o| o.ok()).collect();
    ok(orders)
}

#[delete("/api/orders/{id}")]
async fn delete_order(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    let ticket_id: Option<String> = db.query_row("SELECT ticket_id FROM orders WHERE order_id=?1", params![id], |r| r.get(0)).ok();
    match db.execute("DELETE FROM orders WHERE order_id=?1", params![id]) {
        Ok(1) => {
            if let Some(tid) = ticket_id {
                let _ = db.execute("UPDATE tickets SET num_available=num_available+1 WHERE ticket_id=?1", params![tid]);
            }
            ok(serde_json::json!({ "message": "Refunded" }))
        }
        _ => not_found("Order not found"),
    }
}

// ── Frontend ───────────────────────────────────────────────────────────────────

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(HTML)
}

// ── Main ───────────────────────────────────────────────────────────────────────

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let conn = Connection::open("tickethub.db").expect("Failed to open DB");
    init_db(&conn).expect("Failed to init DB");
    let data = web::Data::new(AppState { db: Mutex::new(conn) });
    println!("🎟  TicketHub running at http://127.0.0.1:8080");
    HttpServer::new(move || {
        let cors = Cors::default().allow_any_origin().allow_any_method().allow_any_header();
        App::new()
            .wrap(cors)
            .app_data(data.clone())
            .app_data(web::JsonConfig::default().error_handler(|err, _| {
                actix_web::error::InternalError::from_response(
                    err, HttpResponse::BadRequest().body("Bad JSON")).into()
            }))
            .service(index)
            .service(create_user).service(list_users).service(get_user).service(update_user).service(delete_user).service(user_orders)
            .service(create_artist).service(list_artists).service(get_artist).service(update_artist).service(delete_artist).service(artist_events)
            .service(create_event).service(list_events).service(get_event).service(delete_event)
            .service(create_ticket).service(list_tickets).service(event_tickets)
            .service(create_order).service(list_orders).service(delete_order)
    })
    .bind("127.0.0.1:8081")?
    .run()
    .await
}

const HTML: &str = include_str!("frontend.html");
