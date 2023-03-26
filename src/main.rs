use std::{net::SocketAddr, process::exit};

use askama::Template;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Html,
};
use log::{error, info, trace};
use rusqlite;

const DBPATH: &'static str = "diary.sqlite3";

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Initializing");
    {
        let mut cxn = db_connection().expect("couldn't connect to database");
        match init_db(&mut cxn) {
            Ok(()) => (),
            Err(e) => {
                error!("Error during database initialization {:?}", e);
                exit(1);
            }
        }
    }
    let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), 62336);
    let app = newapp();
    info!("Listening on {}", addr);
    // TODO static files
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("Failed to start server");
}

pub(crate) type AppError = (StatusCode, String);

fn db_connection() -> Result<rusqlite::Connection, AppError> {
    use std::path;
    trace!("Connecting to database at {}", DBPATH);
    let db_path = path::Path::new(DBPATH);
    rusqlite::Connection::open(db_path).map_err(|e| {
        error!("Error connecting to database: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database connection error".to_owned(),
        )
    })
}

fn init_db(cxn: &mut rusqlite::Connection) -> rusqlite::Result<()> {
    const INIT: &'static str = r##"
    CREATE TABLE IF NOT EXISTS entries
    (
        timestamp TEXT NOT NULL,
        body TEXT NOT NULL
    )
    "##;
    cxn.execute(INIT, []).map(|_| ())
}

fn newapp() -> axum::Router {
    use axum::routing::{get, get_service, post, MethodFilter, Router};
    use tower_http::services::ServeDir;

    /*
    endpoints:

    - new entry (post)
    - read entry (get)
    - month view
    - search / search results

    */
    Router::new()
        .route("/", get(get_index))
        .route("/new", get(get_new_entry))
        .nest_service("/static", get_service(ServeDir::new("./static/")))
}

type Response = Result<Html<String>, AppError>;

async fn get_index() -> Response {
    // TODO implement this
    let body = format!("This is going to be the index page");
    Ok(Html::from(body))
}

#[derive(Template)]
#[template(path = "new.html")]
struct NewEntryViewModel;

async fn get_new_entry() -> Response {
    let vm = NewEntryViewModel {};
    vm.render()
        .map_err(|err| {
            error!("rendering new entry: {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Template rendering error".to_owned(),
            )
        })
        .map(|b| Html::from(b))
}

async fn post_new_entry() -> Response {
    todo!("Implement thïs")
}
