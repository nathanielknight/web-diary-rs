"""Import entries from a directory called "docs"

Files should be the markdown body of the entry. File names
should be "{year}-{month}-{day}-{hour}-{minute}.md"
(WITHOUT leading zeroes).

For example,

    2023-2-3-15-59.md
"""
import datetime
import pathlib
import sqlite3
import zoneinfo

LOCAL_TIMEZONE = "America/Vancouver"
TZ = zoneinfo.ZoneInfo(LOCAL_TIMEZONE)


def get_date(timestamp: int) -> str:
    utc_dt = datetime.datetime.fromtimestamp(timestamp, datetime.UTC)
    local_dt = utc_dt.astimezone(TZ)
    return local_dt.strftime("%Y-%m-%d")


def add_entry(cursor: sqlite3.Cursor, timestamp: int, body: str):
    date = get_date(timestamp)
    cursor.execute("""
    INSERT INTO entries (timestamp, date, body)
    VALUES (?, ?, ?)
    """, [timestamp, date, body])
    cursor.execute("INSERT INTO entrytext (body) VALUES (?)", [body])


def ts_for(y, mo, d, h, mi):
    dt = datetime.datetime(y, mo, d, h, mi, tzinfo=TZ)
    return int(dt.timestamp())

if __name__ == "__main__":
    inputdirname = "docs"
    docs = pathlib.Path(inputdirname)
    assert docs.exists() and docs.is_dir()
    with sqlite3.connect("diary.sqlite3") as cxn:
        cursor = cxn.cursor()
        for f in docs.iterdir():
            if f.is_file():
                ts = ts_for(*[int(n) for n in f.stem.split("-")])
                add_entry(cursor, ts, f.read_text())
