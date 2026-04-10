use std::collections::HashSet;

use anyhow::Result;
use chrono::{Local, NaiveDate};
use rusqlite::{Connection, params};

use crate::note::Note;

pub fn extract_tags(text: &str) -> Vec<String> {
    let mut tags = HashSet::new();
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }
        if trimmed.starts_with('`') {
            continue;
        }

        let mut chars = line.char_indices().peekable();
        while let Some((_, c)) = chars.next() {
            if c == '#' {
                let mut tag = String::new();
                while let Some(&(_, nc)) = chars.peek() {
                    if nc.is_alphanumeric() || nc == '_' {
                        tag.push(nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if !tag.is_empty() {
                    tags.insert(tag);
                }
            }
        }
    }

    let mut result: Vec<String> = tags.into_iter().collect();
    result.sort();
    result
}

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
            CREATE INDEX IF NOT EXISTS idx_note_creation_date ON note(creation_date);
            CREATE TABLE IF NOT EXISTS tag (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            );
            CREATE TABLE IF NOT EXISTS note_tag (
                note_id INTEGER NOT NULL REFERENCES note(id) ON DELETE CASCADE,
                tag_id INTEGER NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
                PRIMARY KEY (note_id, tag_id)
            );",
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
        let note_id: i32 = self.conn.query_row(
            "SELECT id FROM note WHERE creation_date = ?1",
            params![note.creation_date],
            |row| row.get(0),
        )?;
        self.sync_tags(note_id, &note.text)?;
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

    pub fn sync_tags(&self, note_id: i32, text: &str) -> Result<()> {
        let tags = extract_tags(text);
        self.conn
            .execute("DELETE FROM note_tag WHERE note_id = ?1", params![note_id])?;
        for tag_name in &tags {
            self.conn.execute(
                "INSERT INTO tag (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
                params![tag_name],
            )?;
            let tag_id: i32 = self.conn.query_row(
                "SELECT id FROM tag WHERE name = ?1",
                params![tag_name],
                |row| row.get(0),
            )?;
            self.conn.execute(
                "INSERT OR IGNORE INTO note_tag (note_id, tag_id) VALUES (?1, ?2)",
                params![note_id, tag_id],
            )?;
        }
        Ok(())
    }

    pub fn get_all_tags(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT name FROM tag ORDER BY name")?;
        let mut tags = Vec::new();
        for tag in stmt.query_map([], |row| row.get::<_, String>(0))? {
            tags.push(tag?);
        }
        Ok(tags)
    }

    pub fn get_notes_by_tag(&self, tag: &str) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.text, n.creation_date, n.last_edited
             FROM note n
             INNER JOIN note_tag nt ON n.id = nt.note_id
             INNER JOIN tag t ON nt.tag_id = t.id
             WHERE t.name = ?1
             ORDER BY n.creation_date",
        )?;
        let mut notes = Vec::new();
        for note in stmt.query_map(params![tag], Self::row_to_note)? {
            notes.push(note?);
        }
        Ok(notes)
    }

    pub fn get_tags_for_note(&self, note_id: i32) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name FROM tag t
             INNER JOIN note_tag nt ON t.id = nt.tag_id
             WHERE nt.note_id = ?1
             ORDER BY t.name",
        )?;
        let mut tags = Vec::new();
        for tag in stmt.query_map(params![note_id], |row| row.get::<_, String>(0))? {
            tags.push(tag?);
        }
        Ok(tags)
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
                self.sync_tags(note.id, &new_text)?;
            }
            None => {
                let note = Note {
                    id: 0,
                    text: text.to_string(),
                    creation_date: *date,
                    last_edited: now,
                };
                let id = self.insert_note(&note)?;
                self.sync_tags(id, text)?;
            }
        }
        Ok(())
    }
}
