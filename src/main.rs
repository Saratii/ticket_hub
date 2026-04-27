use actix_cors::Cors;
use actix_web::{App, HttpResponse, HttpServer, Responder, delete, get, post, put, web};
use bcrypt::{DEFAULT_COST, hash};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;

const HOST: &str = "0.0.0.0:8081";

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

// Wraps a value in a 200 JSON envelope.
fn ok<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        data: Some(data),
        message: None,
    })
}

// Returns a 400 with an error message in the standard envelope.
fn err_resp(msg: &str) -> HttpResponse {
    HttpResponse::BadRequest().json(ApiResponse::<()> {
        success: false,
        data: None,
        message: Some(msg.to_string()),
    })
}

// Returns a 404 with a message — used when a row lookup comes up empty.
fn not_found(msg: &str) -> HttpResponse {
    HttpResponse::NotFound().json(ApiResponse::<()> {
        success: false,
        data: None,
        message: Some(msg.to_string()),
    })
}

// Creates all tables (if they don't already exist) and runs the seeder.
fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(
        "
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
    ",
    )?;
    seed_db(conn)?;
    Ok(())
}

// Populates the database with artists, events, and tickets on first run; skips if data exists.
fn seed_db(conn: &Connection) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM artists", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }

    let artists: Vec<(&str, &str, &str, &str)> = vec![
        (
            "a0",
            "Sabrina Carpenter",
            "Pop",
            "★ OUR UNDISPUTED QUEEN ★ Sabrina Carpenter is the greatest pop artist of her generation — full stop. From her breakout 'emails i can't send' era to the unstoppable 'Short n' Sweet' world tour, she has redefined what it means to be a pop star: razor-sharp wit, perfect melodies, and the stage presence of a goddess. Every other artist on this platform exists in her shadow and they know it.",
        ),
        (
            "a1",
            "Taylor Swift",
            "Pop",
            "Grammy-winning global pop superstar known for era-defining albums.",
        ),
        (
            "a2",
            "Metallica",
            "Metal",
            "Iconic heavy metal band formed in 1981 with decades of sold-out tours.",
        ),
        (
            "a3",
            "Billie Eilish",
            "Alternative Pop",
            "Whisper-pop sensation, multi-Grammy winner and Oscar winner.",
        ),
        (
            "a4",
            "Bad Bunny",
            "Reggaeton",
            "Latin trap and reggaeton global phenomenon from Puerto Rico.",
        ),
        (
            "a5",
            "The Weeknd",
            "R&B",
            "Critically acclaimed R&B and pop artist known for After Hours era.",
        ),
        (
            "a6",
            "Chappell Roan",
            "Indie Pop",
            "Midwest princess turned pop phenomenon, known for theatrical performances and debut album 'The Rise and Fall of a Midwest Princess'.",
        ),
        (
            "a7",
            "Olivia Rodrigo",
            "Pop Rock",
            "Multi-Grammy-winning singer-songwriter behind the generational debut 'SOUR' and follow-up 'GUTS'.",
        ),
        (
            "a8",
            "Harry Styles",
            "Pop Rock",
            "Former One Direction member turned critically acclaimed solo artist and fashion icon.",
        ),
        (
            "a9",
            "Doja Cat",
            "Pop/R&B",
            "Grammy-winning shapeshifter known for viral hits and her genre-bending 'Scarlet' era.",
        ),
        (
            "a10",
            "SZA",
            "R&B",
            "Multi-Grammy-nominated R&B powerhouse behind the critically acclaimed 'SOS' album.",
        ),
        (
            "a11",
            "Gracie Abrams",
            "Indie Pop",
            "Intimate indie-pop storyteller known for emotionally raw lyrics and a devoted cult following.",
        ),
        (
            "a12",
            "Coldplay",
            "Alternative Rock",
            "British rock icons whose stadium spectacles are legendary for their light shows and emotional anthems.",
        ),
    ];

    for (id, name, genre, bio) in &artists {
        conn.execute(
            "INSERT OR IGNORE INTO artists VALUES (?1,?2,?3,?4)",
            params![id, name, genre, bio],
        )?;
    }

    let events: Vec<(&str, &str, &str, &str, &str, &str, i64)> = vec![
        (
            "e_sc1",
            "a0",
            "Madison Square Garden",
            "New York",
            "NY",
            "2025-07-11",
            20789,
        ),
        (
            "e_sc2",
            "a0",
            "Madison Square Garden",
            "New York",
            "NY",
            "2025-07-12",
            20789,
        ),
        (
            "e_sc3",
            "a0",
            "Madison Square Garden",
            "New York",
            "NY",
            "2025-07-13",
            20789,
        ),
        (
            "e_sc4",
            "a0",
            "United Center",
            "Chicago",
            "IL",
            "2025-07-19",
            20917,
        ),
        (
            "e_sc5",
            "a0",
            "Chase Center",
            "San Francisco",
            "CA",
            "2025-07-25",
            18064,
        ),
        (
            "e_sc6",
            "a0",
            "Kia Forum",
            "Los Angeles",
            "CA",
            "2025-08-01",
            17505,
        ),
        (
            "e_sc7",
            "a0",
            "Kia Forum",
            "Los Angeles",
            "CA",
            "2025-08-02",
            17505,
        ),
        (
            "e_sc8",
            "a0",
            "Scotiabank Arena",
            "Toronto",
            "ON",
            "2025-08-09",
            19800,
        ),
        (
            "e_sc9",
            "a0",
            "TD Garden",
            "Boston",
            "MA",
            "2025-08-15",
            19580,
        ),
        (
            "e_sc10",
            "a0",
            "Capital One Arena",
            "Washington",
            "DC",
            "2025-08-22",
            20356,
        ),
        (
            "e_sc11",
            "a0",
            "State Farm Arena",
            "Atlanta",
            "GA",
            "2025-08-29",
            21000,
        ),
        (
            "e_sc12",
            "a0",
            "American Airlines Center",
            "Dallas",
            "TX",
            "2025-09-05",
            20000,
        ),
        (
            "e_sc13",
            "a0",
            "Ball Arena",
            "Denver",
            "CO",
            "2025-09-12",
            19520,
        ),
        (
            "e_sc14",
            "a0",
            "Rogers Place",
            "Edmonton",
            "AB",
            "2025-09-19",
            18647,
        ),
        (
            "e_sc15",
            "a0",
            "Climate Pledge Arena",
            "Seattle",
            "WA",
            "2025-09-26",
            17459,
        ),
        (
            "e1",
            "a1",
            "Arrowhead Stadium",
            "Kansas City",
            "MO",
            "2025-08-15",
            70000,
        ),
        (
            "e2",
            "a1",
            "Soldier Field",
            "Chicago",
            "IL",
            "2025-08-22",
            61500,
        ),
        (
            "e_ts3",
            "a1",
            "MetLife Stadium",
            "East Rutherford",
            "NJ",
            "2025-10-03",
            82500,
        ),
        (
            "e_ts4",
            "a1",
            "SoFi Stadium",
            "Los Angeles",
            "CA",
            "2025-10-17",
            70240,
        ),
        (
            "e3",
            "a2",
            "Madison Square Garden",
            "New York",
            "NY",
            "2025-09-10",
            20789,
        ),
        (
            "e7",
            "a2",
            "United Center",
            "Chicago",
            "IL",
            "2025-09-20",
            20917,
        ),
        (
            "e_m3",
            "a2",
            "Toyota Center",
            "Houston",
            "TX",
            "2025-10-10",
            18300,
        ),
        (
            "e4",
            "a3",
            "Hollywood Bowl",
            "Los Angeles",
            "CA",
            "2025-07-30",
            17500,
        ),
        (
            "e8",
            "a3",
            "Red Rocks Amphitheatre",
            "Morrison",
            "CO",
            "2025-08-05",
            9525,
        ),
        (
            "e_be3",
            "a3",
            "Barclays Center",
            "Brooklyn",
            "NY",
            "2025-09-14",
            19000,
        ),
        (
            "e5",
            "a4",
            "American Airlines Arena",
            "Miami",
            "FL",
            "2025-10-05",
            19600,
        ),
        (
            "e_bb2",
            "a4",
            "Yankee Stadium",
            "New York",
            "NY",
            "2025-10-18",
            54251,
        ),
        (
            "e6",
            "a5",
            "Chase Center",
            "San Francisco",
            "CA",
            "2025-11-12",
            18064,
        ),
        (
            "e_tw2",
            "a5",
            "T-Mobile Arena",
            "Las Vegas",
            "NV",
            "2025-11-20",
            20000,
        ),
        (
            "e_cr1",
            "a6",
            "Red Rocks Amphitheatre",
            "Morrison",
            "CO",
            "2025-07-04",
            9525,
        ),
        (
            "e_cr2",
            "a6",
            "The Gorge Amphitheatre",
            "George",
            "WA",
            "2025-07-12",
            27500,
        ),
        (
            "e_cr3",
            "a6",
            "Hollywood Bowl",
            "Los Angeles",
            "CA",
            "2025-08-10",
            17500,
        ),
        (
            "e_cr4",
            "a6",
            "Fiserv Forum",
            "Milwaukee",
            "WI",
            "2025-09-06",
            17341,
        ),
        (
            "e_or1",
            "a7",
            "Bridgestone Arena",
            "Nashville",
            "TN",
            "2025-08-08",
            19000,
        ),
        (
            "e_or2",
            "a7",
            "Moda Center",
            "Portland",
            "OR",
            "2025-08-16",
            19980,
        ),
        (
            "e_or3",
            "a7",
            "Little Caesars Arena",
            "Detroit",
            "MI",
            "2025-09-03",
            19422,
        ),
        (
            "e_hs1",
            "a8",
            "Allegiant Stadium",
            "Las Vegas",
            "NV",
            "2025-10-24",
            65000,
        ),
        (
            "e_hs2",
            "a8",
            "Mercedes-Benz Stadium",
            "Atlanta",
            "GA",
            "2025-11-01",
            71000,
        ),
        (
            "e_dc1",
            "a9",
            "Crypto.com Arena",
            "Los Angeles",
            "CA",
            "2025-09-27",
            20000,
        ),
        (
            "e_dc2",
            "a9",
            "PPG Paints Arena",
            "Pittsburgh",
            "PA",
            "2025-10-11",
            18387,
        ),
        (
            "e_sza1",
            "a10",
            "Spectrum Center",
            "Charlotte",
            "NC",
            "2025-08-23",
            19026,
        ),
        (
            "e_sza2",
            "a10",
            "Enterprise Center",
            "St. Louis",
            "MO",
            "2025-09-13",
            18096,
        ),
        (
            "e_sza3",
            "a10",
            "Paycom Center",
            "Oklahoma City",
            "OK",
            "2025-09-27",
            18203,
        ),
        (
            "e_ga1",
            "a11",
            "The Wiltern",
            "Los Angeles",
            "CA",
            "2025-07-22",
            1850,
        ),
        (
            "e_ga2",
            "a11",
            "Terminal 5",
            "New York",
            "NY",
            "2025-08-02",
            3000,
        ),
        (
            "e_ga3",
            "a11",
            "Riviera Theatre",
            "Chicago",
            "IL",
            "2025-08-18",
            2300,
        ),
        (
            "e_cp1",
            "a12",
            "Rose Bowl",
            "Pasadena",
            "CA",
            "2025-10-11",
            90888,
        ),
        (
            "e_cp2",
            "a12",
            "Gillette Stadium",
            "Foxborough",
            "MA",
            "2025-10-25",
            65878,
        ),
        (
            "e_cp3",
            "a12",
            "Hard Rock Stadium",
            "Miami Gardens",
            "FL",
            "2025-11-08",
            65326,
        ),
    ];

    for (id, artist, venue, city, state, date, cap) in &events {
        conn.execute(
            "INSERT OR IGNORE INTO events VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, artist, venue, city, state, date, cap],
        )?;
    }

    let tickets: Vec<(&str, &str, f64, i64)> = vec![
        ("t_sc1a", "e_sc1", 249.99, 1500),
        ("t_sc1b", "e_sc1", 149.99, 3000),
        ("t_sc1c", "e_sc1", 89.99, 6000),
        ("t_sc2a", "e_sc2", 249.99, 1500),
        ("t_sc2b", "e_sc2", 149.99, 3000),
        ("t_sc2c", "e_sc2", 89.99, 6000),
        ("t_sc3a", "e_sc3", 299.99, 1000),
        ("t_sc3b", "e_sc3", 179.99, 2500),
        ("t_sc3c", "e_sc3", 99.99, 5500),
        ("t_sc4a", "e_sc4", 229.99, 1800),
        ("t_sc4b", "e_sc4", 129.99, 4000),
        ("t_sc5a", "e_sc5", 239.99, 1500),
        ("t_sc5b", "e_sc5", 139.99, 3500),
        ("t_sc6a", "e_sc6", 259.99, 1200),
        ("t_sc6b", "e_sc6", 159.99, 3000),
        ("t_sc6c", "e_sc6", 99.99, 4500),
        ("t_sc7a", "e_sc7", 259.99, 1200),
        ("t_sc7b", "e_sc7", 159.99, 3000),
        ("t_sc7c", "e_sc7", 99.99, 4500),
        ("t_sc8a", "e_sc8", 219.99, 2000),
        ("t_sc8b", "e_sc8", 119.99, 4000),
        ("t_sc9a", "e_sc9", 229.99, 1800),
        ("t_sc9b", "e_sc9", 129.99, 3500),
        ("t_sc10a", "e_sc10", 219.99, 2200),
        ("t_sc10b", "e_sc10", 119.99, 4000),
        ("t_sc11a", "e_sc11", 209.99, 2500),
        ("t_sc11b", "e_sc11", 109.99, 5000),
        ("t_sc12a", "e_sc12", 209.99, 2200),
        ("t_sc12b", "e_sc12", 109.99, 4500),
        ("t_sc13a", "e_sc13", 199.99, 2000),
        ("t_sc13b", "e_sc13", 109.99, 4000),
        ("t_sc14a", "e_sc14", 199.99, 1800),
        ("t_sc14b", "e_sc14", 109.99, 3800),
        ("t_sc15a", "e_sc15", 219.99, 1800),
        ("t_sc15b", "e_sc15", 119.99, 3500),
        ("t1", "e1", 89.99, 500),
        ("t2", "e1", 149.99, 200),
        ("t3", "e2", 99.99, 350),
        ("t_ts3a", "e_ts3", 199.99, 800),
        ("t_ts3b", "e_ts3", 129.99, 1500),
        ("t_ts4a", "e_ts4", 189.99, 700),
        ("t_ts4b", "e_ts4", 119.99, 1200),
        ("t4", "e3", 79.99, 150),
        ("t8", "e7", 85.00, 600),
        ("t_m3", "e_m3", 75.00, 500),
        ("t5", "e4", 65.00, 400),
        ("t9", "e8", 55.00, 800),
        ("t_be3", "e_be3", 95.00, 450),
        ("t6", "e5", 110.00, 300),
        ("t_bb2a", "e_bb2", 135.00, 1200),
        ("t_bb2b", "e_bb2", 85.00, 2500),
        ("t7", "e6", 120.00, 250),
        ("t_tw2", "e_tw2", 130.00, 600),
        ("t_cr1", "e_cr1", 79.99, 800),
        ("t_cr2", "e_cr2", 89.99, 1500),
        ("t_cr3", "e_cr3", 99.99, 700),
        ("t_cr4", "e_cr4", 74.99, 900),
        ("t_or1", "e_or1", 85.00, 600),
        ("t_or2", "e_or2", 80.00, 750),
        ("t_or3", "e_or3", 88.00, 550),
        ("t_hs1a", "e_hs1", 150.00, 2000),
        ("t_hs1b", "e_hs1", 95.00, 5000),
        ("t_hs2a", "e_hs2", 150.00, 2200),
        ("t_hs2b", "e_hs2", 95.00, 5500),
        ("t_dc1", "e_dc1", 105.00, 800),
        ("t_dc2", "e_dc2", 95.00, 700),
        ("t_sza1", "e_sza1", 90.00, 700),
        ("t_sza2", "e_sza2", 85.00, 650),
        ("t_sza3", "e_sza3", 85.00, 600),
        ("t_ga1", "e_ga1", 45.00, 600),
        ("t_ga2", "e_ga2", 40.00, 900),
        ("t_ga3", "e_ga3", 42.00, 700),
        ("t_cp1a", "e_cp1", 175.00, 3000),
        ("t_cp1b", "e_cp1", 95.00, 8000),
        ("t_cp2a", "e_cp2", 165.00, 2500),
        ("t_cp2b", "e_cp2", 90.00, 7000),
        ("t_cp3a", "e_cp3", 165.00, 2500),
        ("t_cp3b", "e_cp3", 90.00, 7000),
    ];

    for (id, event, price, avail) in &tickets {
        conn.execute(
            "INSERT OR IGNORE INTO tickets VALUES (?1,?2,?3,?4)",
            params![id, event, price, avail],
        )?;
    }

    let pw_hash = hash("password123", DEFAULT_COST).unwrap_or_default();
    conn.execute(
        "INSERT OR IGNORE INTO users VALUES ('u1','demo_user','demo@tickethub.com',?1,'555-0100','2024-01-01T00:00:00')",
        params![pw_hash],
    )?;
    Ok(())
}

// Hashes the password and inserts a new user row; returns the generated user_id.
#[post("/api/users")]
async fn create_user(data: web::Data<AppState>, body: web::Json<CreateUser>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    let hashed = match hash(&body.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return err_resp("Failed to hash password"),
    };
    match db.execute(
        "INSERT INTO users VALUES (?1,?2,?3,?4,?5,?6)",
        params![id, body.username, body.email, hashed, body.phone, now],
    ) {
        Ok(_) => ok(serde_json::json!({ "user_id": id })),
        Err(e) => err_resp(&format!("Error: {}", e)),
    }
}

// Returns all users sorted by creation time, newest first.
#[get("/api/users")]
async fn list_users(data: web::Data<AppState>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT user_id,username,email,phone,creation_time FROM users ORDER BY creation_time DESC").unwrap();
    let users: Vec<User> = stmt
        .query_map([], |r| {
            Ok(User {
                user_id: r.get(0)?,
                username: r.get(1)?,
                email: r.get(2)?,
                phone: r.get(3)?,
                creation_time: r.get(4)?,
            })
        })
        .unwrap()
        .filter_map(|u| u.ok())
        .collect();
    ok(users)
}

// Fetches a single user by ID; 404s if not found.
#[get("/api/users/{id}")]
async fn get_user(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    match db.query_row(
        "SELECT user_id,username,email,phone,creation_time FROM users WHERE user_id=?1",
        params![id],
        |r| {
            Ok(User {
                user_id: r.get(0)?,
                username: r.get(1)?,
                email: r.get(2)?,
                phone: r.get(3)?,
                creation_time: r.get(4)?,
            })
        },
    ) {
        Ok(u) => ok(u),
        Err(_) => not_found("User not found"),
    }
}

// Patches whichever of username, email, or phone are present in the request body.
#[put("/api/users/{id}")]
async fn update_user(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<UpdateUser>,
) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    let mut n = 0usize;
    if let Some(ref v) = body.username {
        n += db
            .execute(
                "UPDATE users SET username=?1 WHERE user_id=?2",
                params![v, id],
            )
            .unwrap_or(0);
    }
    if let Some(ref v) = body.email {
        n += db
            .execute("UPDATE users SET email=?1 WHERE user_id=?2", params![v, id])
            .unwrap_or(0);
    }
    if let Some(ref v) = body.phone {
        n += db
            .execute("UPDATE users SET phone=?1 WHERE user_id=?2", params![v, id])
            .unwrap_or(0);
    }
    if n == 0 {
        return err_resp("Nothing updated");
    }
    ok(serde_json::json!({ "message": "User updated" }))
}

// Deletes a user by ID; 404s if no row was affected.
#[delete("/api/users/{id}")]
async fn delete_user(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    match db.execute("DELETE FROM users WHERE user_id=?1", params![id]) {
        Ok(1) => ok(serde_json::json!({ "message": "Deleted" })),
        _ => not_found("User not found"),
    }
}

// Returns all orders for a given user, joined with event and artist details.
#[get("/api/users/{id}/orders")]
async fn user_orders(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    let mut stmt = db.prepare(
        "SELECT o.order_id,o.user_id,o.ticket_id,o.date_of_purchase,NULL,e.venue_name,e.city,e.state,e.event_date,t.price,a.name
         FROM orders o JOIN tickets t ON o.ticket_id=t.ticket_id JOIN events e ON t.event_id=e.event_id JOIN artists a ON e.artist_id=a.artist_id
         WHERE o.user_id=?1 ORDER BY o.date_of_purchase DESC"
    ).unwrap();
    let orders: Vec<Order> = stmt
        .query_map(params![id], |r| {
            Ok(Order {
                order_id: r.get(0)?,
                user_id: r.get(1)?,
                ticket_id: r.get(2)?,
                date_of_purchase: r.get(3)?,
                event_name: r.get(4)?,
                venue_name: r.get(5)?,
                city: r.get(6)?,
                state: r.get(7)?,
                event_date: r.get(8)?,
                price: r.get(9)?,
                artist_name: r.get(10)?,
            })
        })
        .unwrap()
        .filter_map(|o| o.ok())
        .collect();
    ok(orders)
}

// Inserts a new artist and returns the generated artist_id.
#[post("/api/artists")]
async fn create_artist(data: web::Data<AppState>, body: web::Json<CreateArtist>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    match db.execute(
        "INSERT INTO artists VALUES (?1,?2,?3,?4)",
        params![id, body.name, body.genre, body.bio],
    ) {
        Ok(_) => ok(serde_json::json!({ "artist_id": id })),
        Err(e) => err_resp(&format!("Error: {}", e)),
    }
}

// Returns every artist sorted alphabetically by name.
#[get("/api/artists")]
async fn list_artists(data: web::Data<AppState>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT artist_id,name,genre,bio FROM artists ORDER BY name")
        .unwrap();
    let artists: Vec<Artist> = stmt
        .query_map([], |r| {
            Ok(Artist {
                artist_id: r.get(0)?,
                name: r.get(1)?,
                genre: r.get(2)?,
                bio: r.get(3)?,
            })
        })
        .unwrap()
        .filter_map(|a| a.ok())
        .collect();
    ok(artists)
}

// Looks up a single artist by ID.
#[get("/api/artists/{id}")]
async fn get_artist(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    match db.query_row(
        "SELECT artist_id,name,genre,bio FROM artists WHERE artist_id=?1",
        params![id],
        |r| {
            Ok(Artist {
                artist_id: r.get(0)?,
                name: r.get(1)?,
                genre: r.get(2)?,
                bio: r.get(3)?,
            })
        },
    ) {
        Ok(a) => ok(a),
        Err(_) => not_found("Artist not found"),
    }
}

// Updates genre and/or bio for an artist; at least one field must be provided.
#[put("/api/artists/{id}")]
async fn update_artist(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<UpdateArtist>,
) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    let mut n = 0usize;
    if let Some(ref v) = body.genre {
        n += db
            .execute(
                "UPDATE artists SET genre=?1 WHERE artist_id=?2",
                params![v, id],
            )
            .unwrap_or(0);
    }
    if let Some(ref v) = body.bio {
        n += db
            .execute(
                "UPDATE artists SET bio=?1 WHERE artist_id=?2",
                params![v, id],
            )
            .unwrap_or(0);
    }
    if n == 0 {
        return err_resp("Nothing updated");
    }
    ok(serde_json::json!({ "message": "Artist updated" }))
}

// Deletes an artist by ID.
#[delete("/api/artists/{id}")]
async fn delete_artist(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    match db.execute("DELETE FROM artists WHERE artist_id=?1", params![id]) {
        Ok(1) => ok(serde_json::json!({ "message": "Deleted" })),
        _ => not_found("Artist not found"),
    }
}

// Returns all events for a given artist, ordered by date ascending.
#[get("/api/artists/{id}/events")]
async fn artist_events(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    let mut stmt = db.prepare(
        "SELECT e.event_id,e.artist_id,a.name,e.venue_name,e.city,e.state,e.event_date,e.capacity
         FROM events e JOIN artists a ON e.artist_id=a.artist_id WHERE e.artist_id=?1 ORDER BY e.event_date"
    ).unwrap();
    let events: Vec<Event> = stmt
        .query_map(params![id], |r| {
            Ok(Event {
                event_id: r.get(0)?,
                artist_id: r.get(1)?,
                artist_name: r.get(2)?,
                venue_name: r.get(3)?,
                city: r.get(4)?,
                state: r.get(5)?,
                event_date: r.get(6)?,
                capacity: r.get(7)?,
            })
        })
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    ok(events)
}

// Creates a new event tied to an existing artist.
#[post("/api/events")]
async fn create_event(data: web::Data<AppState>, body: web::Json<CreateEvent>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    match db.execute(
        "INSERT INTO events VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            id,
            body.artist_id,
            body.venue_name,
            body.city,
            body.state,
            body.event_date,
            body.capacity
        ],
    ) {
        Ok(_) => ok(serde_json::json!({ "event_id": id })),
        Err(e) => err_resp(&format!("Error: {}", e)),
    }
}

// Lists events with optional filtering by artist name, city, and date range.
#[get("/api/events")]
async fn list_events(data: web::Data<AppState>, query: web::Query<EventSearch>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let mut sql = String::from(
        "SELECT e.event_id,e.artist_id,a.name,e.venue_name,e.city,e.state,e.event_date,e.capacity
         FROM events e JOIN artists a ON e.artist_id=a.artist_id WHERE 1=1",
    );
    let mut args: Vec<String> = vec![];
    if let Some(ref v) = query.artist {
        args.push(v.clone());
        sql.push_str(&format!(
            " AND LOWER(a.name) LIKE LOWER('%'||?{}||'%')",
            args.len()
        ));
    }
    if let Some(ref v) = query.city {
        args.push(v.clone());
        sql.push_str(&format!(
            " AND LOWER(e.city) LIKE LOWER('%'||?{}||'%')",
            args.len()
        ));
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
    let events: Vec<Event> = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), |r| {
            Ok(Event {
                event_id: r.get(0)?,
                artist_id: r.get(1)?,
                artist_name: r.get(2)?,
                venue_name: r.get(3)?,
                city: r.get(4)?,
                state: r.get(5)?,
                event_date: r.get(6)?,
                capacity: r.get(7)?,
            })
        })
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    ok(events)
}

// Fetches a single event by ID, joined with the artist name.
#[get("/api/events/{id}")]
async fn get_event(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    match db.query_row(
        "SELECT e.event_id,e.artist_id,a.name,e.venue_name,e.city,e.state,e.event_date,e.capacity
         FROM events e JOIN artists a ON e.artist_id=a.artist_id WHERE e.event_id=?1",
        params![id],
        |r| {
            Ok(Event {
                event_id: r.get(0)?,
                artist_id: r.get(1)?,
                artist_name: r.get(2)?,
                venue_name: r.get(3)?,
                city: r.get(4)?,
                state: r.get(5)?,
                event_date: r.get(6)?,
                capacity: r.get(7)?,
            })
        },
    ) {
        Ok(e) => ok(e),
        Err(_) => not_found("Event not found"),
    }
}

// Deletes an event by ID.
#[delete("/api/events/{id}")]
async fn delete_event(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    match db.execute("DELETE FROM events WHERE event_id=?1", params![id]) {
        Ok(1) => ok(serde_json::json!({ "message": "Deleted" })),
        _ => not_found("Event not found"),
    }
}

// Validates price and availability, then inserts a new ticket tier for an event.
#[post("/api/tickets")]
async fn create_ticket(data: web::Data<AppState>, body: web::Json<CreateTicket>) -> impl Responder {
    if body.price <= 0.0 {
        return err_resp("Price must be positive");
    }
    if body.num_available <= 0 {
        return err_resp("num_available must be positive");
    }
    let db = data.db.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    match db.execute(
        "INSERT INTO tickets VALUES (?1,?2,?3,?4)",
        params![id, body.event_id, body.price, body.num_available],
    ) {
        Ok(_) => ok(serde_json::json!({ "ticket_id": id })),
        Err(e) => err_resp(&format!("Error: {}", e)),
    }
}

// Returns all ticket tiers across all events, sorted cheapest first.
#[get("/api/tickets")]
async fn list_tickets(data: web::Data<AppState>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT t.ticket_id,t.event_id,e.venue_name||' — '||a.name,t.price,t.num_available
         FROM tickets t JOIN events e ON t.event_id=e.event_id JOIN artists a ON e.artist_id=a.artist_id ORDER BY t.price"
    ).unwrap();
    let tickets: Vec<Ticket> = stmt
        .query_map([], |r| {
            Ok(Ticket {
                ticket_id: r.get(0)?,
                event_id: r.get(1)?,
                event_name: r.get(2)?,
                price: r.get(3)?,
                num_available: r.get(4)?,
            })
        })
        .unwrap()
        .filter_map(|t| t.ok())
        .collect();
    ok(tickets)
}

// Returns all ticket tiers for a specific event.
#[get("/api/events/{id}/tickets")]
async fn event_tickets(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    let mut stmt = db
        .prepare(
            "SELECT ticket_id,event_id,NULL,price,num_available FROM tickets WHERE event_id=?1",
        )
        .unwrap();
    let tickets: Vec<Ticket> = stmt
        .query_map(params![id], |r| {
            Ok(Ticket {
                ticket_id: r.get(0)?,
                event_id: r.get(1)?,
                event_name: r.get(2)?,
                price: r.get(3)?,
                num_available: r.get(4)?,
            })
        })
        .unwrap()
        .filter_map(|t| t.ok())
        .collect();
    ok(tickets)
}

// Places an order for a ticket, checks availability, and decrements the count atomically.
#[post("/api/orders")]
async fn create_order(data: web::Data<AppState>, body: web::Json<CreateOrder>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let avail: i64 = match db.query_row(
        "SELECT num_available FROM tickets WHERE ticket_id=?1",
        params![body.ticket_id],
        |r| r.get(0),
    ) {
        Ok(n) => n,
        Err(_) => return not_found("Ticket not found"),
    };
    if avail <= 0 {
        return err_resp("No tickets available");
    }
    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    match db.execute(
        "INSERT INTO orders VALUES (?1,?2,?3,?4)",
        params![id, body.user_id, body.ticket_id, now],
    ) {
        Ok(_) => {
            let _ = db.execute(
                "UPDATE tickets SET num_available=num_available-1 WHERE ticket_id=?1",
                params![body.ticket_id],
            );
            ok(serde_json::json!({ "order_id": id, "date_of_purchase": now }))
        }
        Err(e) => err_resp(&format!("Error: {}", e)),
    }
}

// Returns all orders across all users, with full event and artist details joined in.
#[get("/api/orders")]
async fn list_orders(data: web::Data<AppState>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT o.order_id,o.user_id,o.ticket_id,o.date_of_purchase,NULL,e.venue_name,e.city,e.state,e.event_date,t.price,a.name
         FROM orders o JOIN tickets t ON o.ticket_id=t.ticket_id JOIN events e ON t.event_id=e.event_id JOIN artists a ON e.artist_id=a.artist_id
         ORDER BY o.date_of_purchase DESC"
    ).unwrap();
    let orders: Vec<Order> = stmt
        .query_map([], |r| {
            Ok(Order {
                order_id: r.get(0)?,
                user_id: r.get(1)?,
                ticket_id: r.get(2)?,
                date_of_purchase: r.get(3)?,
                event_name: r.get(4)?,
                venue_name: r.get(5)?,
                city: r.get(6)?,
                state: r.get(7)?,
                event_date: r.get(8)?,
                price: r.get(9)?,
                artist_name: r.get(10)?,
            })
        })
        .unwrap()
        .filter_map(|o| o.ok())
        .collect();
    ok(orders)
}

// Cancels an order and restores the ticket's availability count.
#[delete("/api/orders/{id}")]
async fn delete_order(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let db = data.db.lock().unwrap();
    let id = path.into_inner();
    let ticket_id: Option<String> = db
        .query_row(
            "SELECT ticket_id FROM orders WHERE order_id=?1",
            params![id],
            |r| r.get(0),
        )
        .ok();
    match db.execute("DELETE FROM orders WHERE order_id=?1", params![id]) {
        Ok(1) => {
            if let Some(tid) = ticket_id {
                let _ = db.execute(
                    "UPDATE tickets SET num_available=num_available+1 WHERE ticket_id=?1",
                    params![tid],
                );
            }
            ok(serde_json::json!({ "message": "Refunded" }))
        }
        _ => not_found("Order not found"),
    }
}

// Serves the frontend HTML.
#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(HTML)
}

// Opens the database, runs migrations, registers all routes, and starts the server.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let conn = Connection::open("tickethub.db").expect("Failed to open DB");
    init_db(&conn).expect("Failed to init DB");
    let data = web::Data::new(AppState {
        db: Mutex::new(conn),
    });
    println!("🎟  TicketHub running at http://{}", HOST);
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();
        App::new()
            .wrap(cors)
            .app_data(data.clone())
            .app_data(web::JsonConfig::default().error_handler(|err, _| {
                actix_web::error::InternalError::from_response(
                    err,
                    HttpResponse::BadRequest().body("Bad JSON"),
                )
                .into()
            }))
            .service(index)
            .service(create_user)
            .service(list_users)
            .service(get_user)
            .service(update_user)
            .service(delete_user)
            .service(user_orders)
            .service(create_artist)
            .service(list_artists)
            .service(get_artist)
            .service(update_artist)
            .service(delete_artist)
            .service(artist_events)
            .service(create_event)
            .service(list_events)
            .service(get_event)
            .service(delete_event)
            .service(create_ticket)
            .service(list_tickets)
            .service(event_tickets)
            .service(create_order)
            .service(list_orders)
            .service(delete_order)
    })
    .bind(HOST)?
    .run()
    .await
}

const HTML: &str = include_str!("frontend.html");
