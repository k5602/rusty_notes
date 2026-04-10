use anyhow::Result;
use chrono::{Local, NaiveDate};
use rusqlite::{Connection, params};

use crate::note::Note;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS note (
                id INTEGER PRIMARY KEY,
                text TEXT NOT NULL,
                creation_date DATE NOT NULL UNIQUE,
                last_edited DATETIME NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_note_creation_date ON note(creation_date);",
        )?;
        Ok(Self { conn })
    }

    fn row_to_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<Note> {
        Ok(Note {
            id: row.get(0)?,
            text: row.get(1)?,
            creation_date: row.get(2)?,
            last_edited: row.get(3)?,
        })
    }

    pub fn insert_note(&self, note: &Note) -> Result<i32> {
        self.conn.execute(
            "INSERT INTO note (text, creation_date, last_edited) VALUES (?1, ?2, ?3)",
            params![note.text, note.creation_date, note.last_edited],
        )?;
        Ok(self.conn.last_insert_rowid() as i32)
    }

    pub fn update_note(&self, note: &Note) -> Result<()> {
        self.conn.execute(
            "UPDATE note SET text = ?1, last_edited = ?2 WHERE id = ?3",
            params![note.text, note.last_edited, note.id],
        )?;
        Ok(())
    }

    pub fn get_note_by_date(&self, date: &NaiveDate) -> Result<Option<Note>> {
        let result = self.conn.query_row(
            "SELECT id, text, creation_date, last_edited FROM note WHERE creation_date = ?1",
            params![date],
            Self::row_to_note,
        );
        match result {
            Ok(note) => Ok(Some(note)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_or_create_note(&self, date: &NaiveDate) -> Result<Note> {
        if let Some(note) = self.get_note_by_date(date)? {
            return Ok(note);
        }
        let note = Note {
            id: 0,
            text: String::new(),
            creation_date: *date,
            last_edited: Local::now(),
        };
        let id = self.insert_note(&note)?;
        Ok(Note { id, ..note })
    }

    pub fn upsert_note(&self, note: &Note) -> Result<()> {
        self.conn.execute(
            "INSERT INTO note (text, creation_date, last_edited) VALUES (?1, ?2, ?3)
             ON CONFLICT(creation_date) DO UPDATE SET text = excluded.text, last_edited = excluded.last_edited",
            params![note.text, note.creation_date, note.last_edited],
        )?;
        Ok(())
    }

    pub fn get_all_notes(&self) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, creation_date, last_edited FROM note ORDER BY creation_date",
        )?;
        let mut notes = Vec::new();
        for note in stmt.query_map([], Self::row_to_note)? {
            notes.push(note?);
        }
        Ok(notes)
    }

    pub fn get_notes_in_range(&self, start: &NaiveDate, end: &NaiveDate) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, creation_date, last_edited FROM note
             WHERE creation_date BETWEEN ?1 AND ?2 ORDER BY creation_date",
        )?;
        let mut notes = Vec::new();
        for note in stmt.query_map(params![start, end], Self::row_to_note)? {
            notes.push(note?);
        }
        Ok(notes)
    }

    pub fn get_dates_with_notes(&self, year: i32, month: u32) -> Result<Vec<NaiveDate>> {
        let start = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| anyhow::anyhow!("invalid year {year}, month {month}"))?;
        let next_month = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or_else(|| anyhow::anyhow!("invalid date range end"))?;
        let end = next_month
            .pred_opt()
            .ok_or_else(|| anyhow::anyhow!("invalid date range"))?;

        let mut stmt = self.conn.prepare(
            "SELECT creation_date FROM note WHERE creation_date BETWEEN ?1 AND ?2 ORDER BY creation_date",
        )?;
        let mut dates = Vec::new();
        for date in stmt.query_map(params![start, end], |row| row.get::<_, NaiveDate>(0))? {
            dates.push(date?);
        }
        Ok(dates)
    }

    pub fn get_total_word_count(&self) -> Result<usize> {
        let mut stmt = self.conn.prepare("SELECT text FROM note")?;
        let mut count = 0;
        for word_count in stmt.query_map([], |row| {
            let text: String = row.get(0)?;
            Ok(text.split_whitespace().count())
        })? {
            count += word_count?;
        }
        Ok(count)
    }

    pub fn get_note_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM note", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn append_to_note(&self, date: &NaiveDate, text: &str) -> Result<()> {
        let now = Local::now();
        match self.get_note_by_date(date)? {
            Some(note) => {
                let new_text = if note.text.is_empty() {
                    text.to_string()
                } else {
                    format!("{}\n{}", note.text, text)
                };
                self.conn.execute(
                    "UPDATE note SET text = ?1, last_edited = ?2 WHERE id = ?3",
                    params![new_text, now, note.id],
                )?;
            }
            None => {
                let note = Note {
                    id: 0,
                    text: text.to_string(),
                    creation_date: *date,
                    last_edited: now,
                };
                self.insert_note(&note)?;
            }
        }
        Ok(())
    }
}
