use chrono::{DateTime, Local, NaiveDate};

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub id: i32,
    pub text: String,
    pub creation_date: NaiveDate,
    pub last_edited: DateTime<Local>,
}
