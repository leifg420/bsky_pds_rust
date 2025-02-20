use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware};
use serde::Deserialize;
use std::{sync::{Arc, Mutex}};
use rusqlite::Connection;
use tokio::task;
use log::{error, info, warn};
use simplelog::*;
use actix_files::Files as ActixFiles;
mod models;
use models::{Post, User};
use std::fs::File;

struct AppState {
    users: Arc<Mutex<Vec<User>>>,
    posts: Arc<Mutex<Vec<Post>>>,
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
fn init_db() -> rusqlite::Result<Connection> {
    let conn = Connection::open("app.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY,
            message TEXT NOT NULL
        )",
        [],
    )?;
    Ok(conn)
}

fn create_log(conn: &Connection, message: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO logs (message) VALUES (?1)",
        &[message],
    )?;
    Ok(())
}

async fn create_post(data: web::Data<AppState>, post: web::Json<CreatePost>) -> impl Responder {
    let mut posts = data.posts.lock().unwrap();
    let id = posts.len() as u32 + 1;
    let new_post = Post {
        id,
        user_id: post.user_id,
        content: post.content.clone(),
    };
    posts.push(new_post.clone());

    let log_message = serialize_post_to_log!(new_post);

    let log_message_clone = log_message.clone(); // Clone log_message to ensure it is Send

    if let Err(e) = task::spawn_blocking(move || {
        let conn = init_db().expect("Failed to initialize database");
        create_log(&conn, &log_message_clone)
    }).await.expect("Failed to run blocking task")
    {
        error!("Failed to insert log into database: {:?}", e);
    }

    HttpResponse::Ok().json("Post created")
}

async fn not_found() -> impl Responder {
    HttpResponse::NotFound().json("404 Not Found")
}

async fn internal_server_error() -> impl Responder {
    HttpResponse::InternalServerError().json("500 Internal Server Error")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    TermLogger::init(LevelFilter::Debug, Config::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();
    WriteLogger::init(LevelFilter::Debug, Config::default(), File::create("app.log").unwrap()).unwrap();

    let data = web::Data::new(AppState {
        users: Arc::new(Mutex::new(vec![])),
        posts: Arc::new(Mutex::new(vec![])),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(middleware::Logger::default())
            .wrap(middleware::ErrorHandlers::new()
                .handler(actix_web::http::StatusCode::NOT_FOUND, |res| {
                    Box::pin(async move {
                        let response = res.into_response(not_found().await.respond_to(&res).map_into_boxed_body());
                        Ok(actix_web::middleware::ErrorHandlerResponse::Response(response))
                    })
                })
                .handler(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, |res| {
                    Box::pin(async move {
                        let response = res.into_response(internal_server_error().await);
                        Ok(actix_web::middleware::ErrorHandlerResponse::Response(response))
                    })
                })
            )
            .route("/users", web::post().to(create_user))
            .route("/posts", web::post().to(create_post))
            .service(ActixFiles::new("/", "./static").index_file("index.html").default_handler(|req| {
                let response = req.into_response(not_found());
                Ok(response)
            }))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
