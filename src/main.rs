use std::{net::SocketAddr, process::exit};

use askama::Template;
use axum::{
    extract::{Form, Path, Query},
    http::StatusCode,
    response::{Html, Redirect},
};
use log::{error, info, trace};
use rusqlite;

const DBPATH: &'static str = "diary.sqlite3";

#[tokio::main(flavor = "current_thread")]
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

type Response = Result<Html<String>, AppError>;

fn convert_db_error(err: rusqlite::Error) -> AppError {
    use rusqlite::Error;
    error!("{:?}", err);
    match err {
        Error::QueryReturnedNoRows => (StatusCode::NOT_FOUND, "Not found".to_owned()),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database Error".to_owned(),
        ),
    }
}

fn convert_parse_error(err: chrono::ParseError) -> AppError {
    error!("{:?}", err);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Date format conversion error".to_owned(),
    )
}

fn convert_render_error(err: askama::Error) -> AppError {
    error!("rendering new entry: {:?}", err);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Template rendering error".to_owned(),
    )
}

fn db_connection() -> Result<rusqlite::Connection, AppError> {
    use std::path;
    trace!("Connecting to database at {}", DBPATH);
    let db_path = path::Path::new(DBPATH);
    rusqlite::Connection::open(db_path).map_err(convert_db_error)
}

fn init_db(cxn: &mut rusqlite::Connection) -> rusqlite::Result<()> {
    const INIT: &'static str = r##"
    CREATE TABLE IF NOT EXISTS entries
    (
        timestamp INTEGER NOT NULL,
        date TEXT NOT NULL,
        body TEXT NOT NULL
    )
    "##;
    cxn.execute(INIT, []).map(|_| ())
}

fn newapp() -> axum::Router {
    use axum::routing::{get, get_service, Router};
    use tower_http::services::ServeDir;

    /*
    endpoints:

    - read entry (get)
    - month view
    - search / search results

    */
    Router::new()
        .route("/", get(get_index))
        .route("/new", get(get_new_entry).post(post_new_entry))
        .route("/entry/:rowid", get(get_entry))
        .nest_service("/static", get_service(ServeDir::new("./static/")))
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexViewModel;

async fn get_index() -> Response {
    let vm = IndexViewModel{};
    let body = vm.render().map_err(convert_render_error)?;
    Ok(Html::from(body))
}

#[derive(Template)]
#[template(path = "new.html")]
struct NewEntryViewModel;

async fn get_new_entry() -> Response {
    let vm = NewEntryViewModel {};
    vm.render()
        .map_err(convert_render_error)
        .map(|b| Html::from(b))
}

#[derive(serde::Deserialize)]
struct NewEntry {
    body: String,
}

async fn post_new_entry(Form(newentry): Form<NewEntry>) -> Result<Redirect, AppError> {
    let cxn = db_connection()?;
    const CMD: &'static str = r#"
        INSERT INTO entries (timestamp, date, body)
        VALUES (unixepoch('now'), date('now'), $1)
        RETURNING rowid
    "#;
    let new_entry_id: u32 = cxn
        .query_row(CMD, [&newentry.body], |r| r.get(0))
        .map_err(convert_db_error)?;
    let new_item_url = format!("/entry/{}", new_entry_id);
    Ok(Redirect::to(&new_item_url))
}

#[derive(Template)]
#[template(path = "entry.html")]
struct EntryViewModel {
    date: chrono::NaiveDate,
    timestamp: chrono::DateTime<chrono::Utc>,
    body: String,
}

impl EntryViewModel {
    fn fetch(id: u32) -> Result<Self, AppError> {
        use chrono::{LocalResult, NaiveDate, TimeZone, Utc};

        let cxn = db_connection()?;
        const QUERY: &'static str = "SELECT date, timestamp, body FROM entries WHERE rowid = ?";

        struct RawEntry {
            date: String,
            timestamp: u64,
            body: String,
        }

        let raw_entry: RawEntry = cxn
            .query_row(QUERY, [id], |r| {
                let entry = RawEntry {
                    date: r.get(0)?,
                    timestamp: r.get(1)?,
                    body: r.get(2)?,
                };
                Ok(entry)
            })
            .map_err(convert_db_error)?;

        let timestamp = match Utc.timestamp_opt(raw_entry.timestamp as i64, 0) {
            LocalResult::None | LocalResult::Ambiguous(_, _) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Invalid timestamp: {}", raw_entry.timestamp),
                ))
            }
            LocalResult::Single(t) => t,
        };

        let entry = EntryViewModel {
            date: NaiveDate::parse_from_str(&raw_entry.date, "%Y-%m-%d")
                .map_err(convert_parse_error)?,
            timestamp,
            body: raw_entry.body,
        };
        Ok(entry)
    }
}

async fn get_entry(Path(rowid): Path<u32>) -> Response {
    use ammonia::clean;
    use pulldown_cmark::{html::push_html, Options, Parser};

    let mut entry = EntryViewModel::fetch(rowid)?;

    let mut unsafe_html = String::new();
    {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
        let md_parse = Parser::new_ext(&entry.body, options);
        push_html(&mut unsafe_html, md_parse);
    }
    let safe_html = clean(&*unsafe_html);
    entry.body = safe_html;

    let body = entry.render().map_err(|e| {
        error!("{:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "".to_owned())
    })?;
    Ok(Html(body))
}

