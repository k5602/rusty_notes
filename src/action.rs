use chrono::NaiveDate;

use crate::note::Note;

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Noop,
    OpenTodaysEntry,
    OpenEntryPicker,
    OpenCalendar,
    OpenNote(NaiveDate),
    ShowStats,
    ShowOptions,
    BackToTitle,
    SaveNote(Note),
    ToggleSidePanel,
    StartSearch,
    StartReplace,
    BrowseByTag,
    FilterByTag(String),
    LoadTagNotes(String),
}
