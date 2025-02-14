use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;
use std::sync::Mutex;
use rusqlite;
use rusqlite::Connection;
use serde_json::json;
use serde_json::Value;
use tokio::task;

mod models;
use models::{Post, User};

#[derive(Debug, Deserialize)]
pub struct Log {
    id: i32,
    message: String,
    timestamp: String,
}

struct AppState {
    users: Mutex<Vec<User>>,
    posts: Mutex<Vec<Post>>,
}

#[derive(Deserialize)]
struct CreateUser {
    username: String,
    email: String,
}

fn create_user(data: web::Data<AppState>, user: web::Json<CreateUser>) -> impl Responder {
    let conn = match init_db() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().json("Failed to initialize database"),
    };

    let mut users = data.users.lock().unwrap();
    let id = users.len() as u32 + 1;
    let new_user = User {
        id,
        username: user.username.clone(),
        email: user.email.clone(),
    };
    users.push(new_user);

    if let Err(_) = insert_user(&conn, &user.username, &user.email) {
        return HttpResponse::InternalServerError().json("Failed to insert user into database");
    }

    HttpResponse::Ok().json("User created")
}

#[derive(Deserialize)]
struct CreatePost {
    user_id: u32,
    content: String,
}

macro_rules! serialize_post_to_log {
    ($post:expr) => {
        format!("User {} created a post: {}", $post.user_id, $post.content)
    };
}

async fn create_post(data: web::Data<AppState>, post: web::Json<CreatePost>) -> impl Responder {
    let mut posts = data.posts.lock().unwrap();
    let id = posts.len() as u32 + 1;
    let new_post = Post {
        id,
        user_id: post.user_id,
        content: post.content.clone(),
    };
    posts.push(new_post);

    let log_message = serialize_post_to_log!(new_post);

    if let Err(e) = task::spawn_blocking(move || {
        let conn = init_db().expect("Failed to initialize database");
        create_log(&conn, &log_message)
    }).await.expect("Failed to run blocking task")
    {
        eprintln!("Failed to insert log into database: {:?}", e);
    }

    HttpResponse::Ok().json("Post created")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let data = web::Data::new(AppState {
        users: Mutex::new(vec![]),
        posts: Mutex::new(vec![]),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/users", web::post().to(create_user))
            .route("/posts", web::post().to(create_post))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

use rusqlite::{params, Connection, Result};

pub fn init_db() -> Result<Connection> {
    let conn = Connection::open("pds_server.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS user (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL,
            email TEXT NOT NULL
        )",
        params![],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS log (
            id INTEGER PRIMARY KEY,
            message TEXT NOT NULL,
            timestamp TEXT NOT NULL
        )",
        params![],
    )?;
    Ok(conn)
}

pub fn insert_user(conn: &Connection, username: &str, email: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO user (username, email) VALUES (?1, ?2)",
        params![username, email],
    )?;
    Ok(())
}

pub fn create_log(conn: &Connection, message: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO log (message, timestamp) VALUES (?1, datetime('now'))",
        params![message],
    )?;
    Ok(())
}

pub fn get_logs(conn: &Connection) -> Result<Vec<Log>> {
    let mut stmt = conn.prepare("SELECT id, message, timestamp FROM log")?;
    let log_iter = stmt.query_map(params![], |row| {
        Ok(Log {
            id: row.get(0)?,
            message: row.get(1)?,
            timestamp: row.get(2)?,
        })
    })?;

    let mut logs = Vec::new();
    for log in log_iter {
        logs.push(log?);
    }
    Ok(logs)
}
