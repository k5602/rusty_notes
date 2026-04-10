use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::{Local, NaiveDate};
use crossterm::event::KeyEvent;

use crate::action::Action;
use crate::components::calendar::CalendarComponent;
use crate::components::title_screen::TitleScreenComponent;
use crate::config::Config;
use crate::database::Database;
use crate::editor::Editor;
use crate::event::{AppEvent, next_event};
use crate::note::Note;
use crate::terminal::Tui;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    TitleScreen,
    Editor,
    Calendar,
}

pub struct App {
    mode: AppMode,
    db: Database,
    config: Config,
    editor: Editor,
    title_screen: TitleScreenComponent,
    calendar: CalendarComponent,
    should_quit: bool,
    last_save: Instant,
}

impl App {
    pub fn new(db: Database, config: Config) -> Self {
        let note_count = db.get_note_count().unwrap_or(0);
        let word_count = db.get_total_word_count().unwrap_or(0);
        let mut title_screen = TitleScreenComponent::new();
        title_screen.update_stats(note_count, word_count);
        let mut editor = Editor::new();
        editor.side_panel = config.side_panel_default;
        Self {
            mode: AppMode::TitleScreen,
            db,
            config,
            editor,
            title_screen,
            calendar: CalendarComponent::new(),
            should_quit: false,
            last_save: Instant::now(),
        }
    }

    pub fn run(&mut self, terminal: &mut Tui) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| match self.mode {
                AppMode::TitleScreen => self.title_screen.render(f),
                AppMode::Editor => self.editor.render(f),
                AppMode::Calendar => self.calendar.render(f, &self.db),
            })?;

            let event = next_event()?;

            let action = match event {
                AppEvent::Key(key) => self.handle_key_event(key),
                AppEvent::Resize(w, h) => {
                    self.editor.screen_size = (h, w);
                    Action::Noop
                }
                AppEvent::Tick => {
                    self.try_auto_save();
                    Action::Noop
                }
            };

            self.process_action(action)?;
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        match self.mode {
            AppMode::TitleScreen => self.title_screen.handle_event(key),
            AppMode::Editor => self.editor.handle_event(key),
            AppMode::Calendar => self.calendar.handle_event(key),
        }
    }

    fn process_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Quit => {
                if self.mode == AppMode::Editor {
                    self.save_editor()?;
                }
                self.should_quit = true;
            }
            Action::Noop => {}
            Action::OpenTodaysEntry => {
                let today = Local::now().date_naive();
                self.open_note(today)?;
            }
            Action::OpenEntryPicker => {}
            Action::OpenCalendar => {
                self.mode = AppMode::Calendar;
            }
            Action::OpenNote(date) => {
                self.open_note(date)?;
            }
            Action::ShowStats => {
                let note_count = self.db.get_note_count()?;
                let word_count = self.db.get_total_word_count()?;
                self.title_screen.update_stats(note_count, word_count);
            }
            Action::ShowOptions => {}
            Action::BackToTitle => {
                if self.mode == AppMode::Editor {
                    self.save_editor()?;
                }
                self.mode = AppMode::TitleScreen;
            }
            Action::SaveNote(note) => {
                self.db.upsert_note(&note)?;
                self.editor.write = false;
                self.last_save = Instant::now();
            }
            Action::ToggleSidePanel => {
                self.editor.side_panel = !self.editor.side_panel;
            }
            Action::StartSearch => {}
            Action::StartReplace => {}
            Action::BrowseByTag => {
                let tags = self.db.get_all_tags()?;
                self.title_screen.start_tag_filter(tags);
            }
            Action::FilterByTag(tag) => {
                let notes = self.db.get_notes_by_tag(&tag)?;
                if let Some(note) = notes.first() {
                    self.open_note(note.creation_date)?;
                }
            }
            Action::LoadTagNotes(tag) => {
                let notes = self.db.get_notes_by_tag(&tag)?;
                self.title_screen.set_filtered_notes(notes);
            }
        }
        Ok(())
    }

    fn open_note(&mut self, date: NaiveDate) -> Result<()> {
        if self.mode == AppMode::Editor {
            self.save_editor()?;
        }
        let note = self.db.get_or_create_note(&date)?;
        self.editor = Editor::from_note(note);
        self.editor.side_panel = self.config.side_panel_default;
        self.mode = AppMode::Editor;
        self.last_save = Instant::now();
        Ok(())
    }

    fn save_editor(&mut self) -> Result<()> {
        if !self.editor.write {
            return Ok(());
        }
        let note = Note {
            id: 0,
            text: self.editor.current_note_text(),
            creation_date: self.editor.creation_date,
            last_edited: Local::now(),
        };
        self.db.upsert_note(&note)?;
        self.editor.write = false;
        self.last_save = Instant::now();
        Ok(())
    }

    fn try_auto_save(&mut self) {
        if self.mode != AppMode::Editor || !self.editor.write {
            return;
        }
        if self.last_save.elapsed() >= Duration::from_secs(self.config.auto_save_seconds) {
            let _ = self.save_editor();
        }
    }
}
