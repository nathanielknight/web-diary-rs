use std::{net::SocketAddr, process::exit};

use askama::Template;
use axum::{
    extract::{Form, Path, Query},
    http::StatusCode,
    response::{Html, Redirect},
};
use chrono::{DateTime, NaiveDate, Utc};
use log::{error, info, trace};

const DBPATH: &str = "diary.sqlite3";

fn newapp() -> axum::Router {
    use axum::routing::{get, get_service, Router};
    use tower_http::services::ServeDir;

    /*
    endpoints:
    - year view
    - search / search results

    */
    Router::new()
        .route("/", get(get_index))
        .route("/new", get(get_new_entry).post(post_new_entry))
        .route("/entry/:rowid", get(get_entry))
        .route("/year/:year", get(get_year))
        .nest_service("/static", get_service(ServeDir::new("./static/")))
}

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

struct Entry {
    id: u32,
    date: NaiveDate,
    timestamp: DateTime<Utc>,
    body: String,
}

struct RawEntry {
    id: u32,
    date: String,
    timestamp: u64,
    body: String,
}

impl RawEntry {
    fn from_row(r: &rusqlite::Row) -> rusqlite::Result<Self> {
        let entry = RawEntry {
            id: r.get(0)?,
            date: r.get(1)?,
            timestamp: r.get(2)?,
            body: r.get(3)?,
        };

        Ok(entry)
    }
}

impl TryInto<Entry> for RawEntry {
    type Error = AppError;
    fn try_into(self) -> Result<Entry, Self::Error> {
        use chrono::{LocalResult, TimeZone};

        let timestamp = match Utc.timestamp_opt(self.timestamp as i64, 0) {
            LocalResult::None | LocalResult::Ambiguous(_, _) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Invalid timestamp: {}", self.timestamp),
                ))
            }
            LocalResult::Single(t) => t,
        };

        let entry = Entry {
            id: self.id,
            date: NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").map_err(convert_parse_error)?,
            timestamp,
            body: self.body,
        };
        Ok(entry)
    }
}

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
    const INIT: &str = r##"
    CREATE TABLE IF NOT EXISTS entries
    (
        timestamp INTEGER NOT NULL,
        date TEXT NOT NULL,
        body TEXT NOT NULL
    )
    "##;
    cxn.execute(INIT, []).map(|_| ())
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexViewModel {
    recent: Vec<Entry>,
    year_counts: Vec<(u32, u32)>,
}

impl Entry {
    fn recent(count: usize) -> Result<Vec<Entry>, AppError> {
        let cxn = db_connection()?;
        const QUERY: &str = r#"
            SELECT rowid, date, timestamp, body
            FROM entries
            ORDER BY timestamp DESC
            LIMIT ?
        "#;
        let mut qry = cxn.prepare(QUERY).map_err(convert_db_error)?;
        let mut entries = Vec::new();
        let results = qry
            .query_map([count], RawEntry::from_row)
            .map_err(convert_db_error)?;
        for raw in results {
            let raw = raw.map_err(convert_db_error)?;
            let entry = raw.try_into()?;
            entries.push(entry);
        }
        Ok(entries)
    }
}

async fn get_index() -> Response {
    let recent = Entry::recent(8)?;
    let year_counts = year_counts()?;
    let vm = IndexViewModel {
        recent,
        year_counts,
    };
    let body = vm.render().map_err(convert_render_error)?;
    Ok(Html::from(body))
}

#[derive(Template)]
#[template(path = "new.html")]
struct NewEntryViewModel {}

async fn get_new_entry() -> Response {
    let vm = NewEntryViewModel {};
    vm.render()
        .map_err(convert_render_error)
        .map(Html::from)
}

#[derive(serde::Deserialize)]
struct NewEntry {
    body: String,
}

async fn post_new_entry(Form(newentry): Form<NewEntry>) -> Result<Redirect, AppError> {
    let cxn = db_connection()?;
    const CMD: &str = r#"
        INSERT INTO entries (timestamp, date, body)
        VALUES (unixepoch('now'), date('now', 'localtime'), $1)
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
    date: NaiveDate,
    timestamp: DateTime<Utc>,
    body: String,
}

impl EntryViewModel {
    fn fetch(id: u32) -> Result<Self, AppError> {
        let cxn = db_connection()?;
        const QUERY: &str =
            "SELECT rowid, date, timestamp, body FROM entries WHERE rowid = ?";

        let raw_entry: RawEntry = cxn
            .query_row(QUERY, [id], RawEntry::from_row)
            .map_err(convert_db_error)?;
        let entry: Entry = raw_entry.try_into()?;
        let vm = EntryViewModel {
            date: entry.date,
            timestamp: entry.timestamp,
            body: entry.body,
        };
        Ok(vm)
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
    let safe_html = clean(&unsafe_html);
    entry.body = safe_html;

    let body = entry.render().map_err(|e| {
        error!("{:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "".to_owned())
    })?;
    Ok(Html(body))
}

fn year_counts() -> Result<Vec<(u32, u32)>, AppError> {
    let cxn = db_connection()?;
    let qry = r#"
        SELECT
            strftime('%Y', date) AS year,
            COUNT(*) as cnt
        FROM entries
        GROUP BY year
        ORDER BY year DESC
    "#;
    let mut stmt = cxn.prepare(qry).map_err(convert_db_error)?;
    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(convert_db_error)?;
    let mut results = Vec::new();
    for row in rows {
        let raw: (String, u32) = row.map_err(convert_db_error)?;
        let year: u32 = raw.0.parse().map_err(|e| {
            error!("{:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Year parsing error".to_string(),
            )
        })?;
        results.push((year, raw.1));
    }
    Ok(results)
}

#[derive(Template)]
#[template(path = "year.html")]
struct YearViewModel {
    year: u32,
    months: Vec<(chrono::Month, Vec<Entry>)>,
    entry_count: u32,
}

impl Entry {
    fn month(&self) -> Result<chrono::Month, AppError> {
        use chrono::prelude::*;
        use num_traits::FromPrimitive;

        Month::from_u32(self.timestamp.month()).ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Date conversion error".to_string(),
        ))
    }
}

impl YearViewModel {
    fn get(year: u32) -> Result<Self, AppError> {
        use chrono::Month;
        use std::collections::HashMap;
        let cxn = db_connection()?;
        const QUERY: &str = r#"
        SELECT rowid, date, timestamp, body,
            strftime('%Y', date) as year, strftime('%m', date) as month
        FROM entries
        WHERE ? = CAST(year AS INTEGER)
        ORDER BY month
        "#;
        let mut qry = cxn.prepare(QUERY).map_err(convert_db_error)?;
        let mut entries: HashMap<chrono::Month, Vec<Entry>> = HashMap::new();
        let results = qry
            .query_map([year], RawEntry::from_row)
            .map_err(convert_db_error)?;
        let mut entry_count = 0;
        for raw in results {
            let raw = raw.map_err(convert_db_error)?;
            let entry: Entry = raw.try_into()?;
            let month = entry.month()?;
            if let Some(month_list) = entries.get_mut(&month) {
                month_list.push(entry);
            } else {
                entries.insert(month, vec![entry]);
            }
            entry_count += 1;
        }
        let months: Vec<(Month, Vec<Entry>)> = entries.into_iter().collect();
        Ok(YearViewModel {
            year,
            months,
            entry_count,
        })
    }
}

async fn get_year(Path(year): Path<u32>) -> Response {
    let vm = YearViewModel::get(year)?;
    let body = vm.render().map_err(convert_render_error)?;
    Ok(Html(body))
}
