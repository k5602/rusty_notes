use chrono::{Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::action::Action;
use crate::note::Note;

#[derive(Clone, Debug, PartialEq)]
pub enum TitleScreenState {
    Options,
    Stats,
    EntryPicker(EntryPicker),
    TagFilter(TagFilterState),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TagFilterState {
    pub tags: Vec<String>,
    pub selected: usize,
    pub filtered_notes: Vec<Note>,
    pub note_selected: usize,
    pub browsing_notes: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntryPicker {
    pub input: String,
    pub cursor: usize,
}

impl EntryPicker {
    pub fn new() -> EntryPicker {
        EntryPicker {
            input: Local::now().format("%d%m%y").to_string(),
            cursor: 0,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        if !c.is_ascii_digit() {
            return;
        }
        self.input
            .replace_range(self.cursor..self.cursor + 1, &c.to_string());
        self.move_right();
    }

    pub fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
    }

    pub fn move_right(&mut self) {
        if self.cursor == 5 {
            return;
        }
        self.cursor += 1;
    }

    pub fn get_date(&self) -> Option<NaiveDate> {
        let day: u32 = self.input[0..2].parse().ok()?;
        let month: u32 = self.input[2..4].parse().ok()?;
        let year: i32 = 2000 + self.input[4..6].parse::<i32>().ok()?;
        let date = NaiveDate::from_ymd_opt(year, month, day)?;
        if date <= Local::now().date_naive() {
            return Some(date);
        }
        None
    }
}

pub struct TitleScreenComponent {
    pub state: TitleScreenState,
    note_count: usize,
    word_count: usize,
    show_help: bool,
}

impl TitleScreenComponent {
    pub fn new() -> Self {
        Self {
            state: TitleScreenState::Options,
            note_count: 0,
            word_count: 0,
            show_help: false,
        }
    }

    pub fn update_stats(&mut self, note_count: usize, word_count: usize) {
        self.note_count = note_count;
        self.word_count = word_count;
    }

    pub fn start_tag_filter(&mut self, tags: Vec<String>) {
        self.state = TitleScreenState::TagFilter(TagFilterState {
            tags,
            selected: 0,
            filtered_notes: Vec::new(),
            note_selected: 0,
            browsing_notes: false,
        });
    }

    pub fn set_filtered_notes(&mut self, notes: Vec<Note>) {
        if let TitleScreenState::TagFilter(ref mut tf) = self.state {
            tf.filtered_notes = notes;
            tf.note_selected = 0;
            tf.browsing_notes = true;
        }
    }

    pub fn handle_event(&mut self, key: KeyEvent) -> Action {
        if key.code == KeyCode::Char('?') {
            self.show_help = !self.show_help;
            return Action::Noop;
        }
        match &self.state {
            TitleScreenState::Options => match key.code {
                KeyCode::Char('t') => Action::OpenTodaysEntry,
                KeyCode::Char('o') => {
                    self.state = TitleScreenState::EntryPicker(EntryPicker::new());
                    Action::Noop
                }
                KeyCode::Char('c') => Action::OpenCalendar,
                KeyCode::Char('s') => {
                    self.state = TitleScreenState::Stats;
                    Action::Noop
                }
                KeyCode::Char('g') => Action::BrowseByTag,
                KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
                _ => Action::Noop,
            },
            TitleScreenState::Stats => match key.code {
                KeyCode::Char('s') | KeyCode::Backspace | KeyCode::Esc => {
                    self.state = TitleScreenState::Options;
                    Action::Noop
                }
                _ => Action::Noop,
            },
            TitleScreenState::EntryPicker(ep) => {
                let mut ep = ep.clone();
                let action = match key.code {
                    KeyCode::Esc => {
                        self.state = TitleScreenState::Options;
                        Action::Noop
                    }
                    KeyCode::Enter => {
                        if let Some(date) = ep.get_date() {
                            self.state = TitleScreenState::Options;
                            Action::OpenNote(date)
                        } else {
                            Action::Noop
                        }
                    }
                    KeyCode::Left => {
                        ep.move_left();
                        Action::Noop
                    }
                    KeyCode::Right => {
                        ep.move_right();
                        Action::Noop
                    }
                    KeyCode::Char(c) => {
                        ep.insert_char(c);
                        Action::Noop
                    }
                    _ => Action::Noop,
                };
                if matches!(self.state, TitleScreenState::EntryPicker(_)) {
                    self.state = TitleScreenState::EntryPicker(ep);
                }
                action
            }
            TitleScreenState::TagFilter(tf) => {
                let mut tf = tf.clone();
                let action = match key.code {
                    KeyCode::Esc => {
                        if tf.browsing_notes {
                            tf.browsing_notes = false;
                            tf.filtered_notes.clear();
                            Action::Noop
                        } else {
                            self.state = TitleScreenState::Options;
                            Action::Noop
                        }
                    }
                    KeyCode::Up => {
                        if tf.browsing_notes {
                            if tf.note_selected > 0 {
                                tf.note_selected -= 1;
                            }
                        } else if tf.selected > 0 {
                            tf.selected -= 1;
                        }
                        Action::Noop
                    }
                    KeyCode::Down => {
                        if tf.browsing_notes {
                            if tf.note_selected + 1 < tf.filtered_notes.len() {
                                tf.note_selected += 1;
                            }
                        } else if tf.selected + 1 < tf.tags.len() {
                            tf.selected += 1;
                        }
                        Action::Noop
                    }
                    KeyCode::Enter => {
                        if tf.browsing_notes {
                            if let Some(note) = tf.filtered_notes.get(tf.note_selected) {
                                let date = note.creation_date;
                                self.state = TitleScreenState::Options;
                                Action::OpenNote(date)
                            } else {
                                Action::Noop
                            }
                        } else if let Some(tag) = tf.tags.get(tf.selected).cloned() {
                            Action::LoadTagNotes(tag)
                        } else {
                            Action::Noop
                        }
                    }
                    _ => Action::Noop,
                };
                if matches!(self.state, TitleScreenState::TagFilter(_)) {
                    self.state = TitleScreenState::TagFilter(tf);
                }
                action
            }
        }
    }

    pub fn render(&mut self, f: &mut Frame) {
        const TITLE_SIZE: (u16, u16) = (10, 93);
        let title_text = [
            " ██▀███   █    ██   ██████ ▄▄▄█████▓▓██   ██▓    ███▄    █  ▒█████  ▄▄▄█████▓▓█████   ██████ ",
            "▓██ ▒ ██▒ ██  ▓██▒▒██    ▒ ▓  ██▒ ▓▒ ▒██  ██▒    ██ ▀█   █ ▒██▒  ██▒▓  ██▒ ▓▒▓█   ▀ ▒██    ▒ ",
            "▓██ ░▄█ ▒▓██  ▒██░░ ▓██▄   ▒ ▓██░ ▒░  ▒██ ██░   ▓██  ▀█ ██▒▒██░  ██▒▒ ▓██░ ▒░▒███   ░ ▓██▄   ",
            "▒██▀▀█▄  ▓▓█  ░██░  ▒   ██▒░ ▓██▓ ░   ░ ▐██▓░   ▓██▒  ▐▌██▒▒██   ██░░ ▓██▓ ░ ▒▓█  ▄   ▒   ██▒",
            "░██▓ ▒██▒▒▒█████▓ ▒██████▒▒  ▒██▒ ░   ░ ██▒▓░   ▒██░   ▓██░░ ████▓▒░  ▒██▒ ░ ░▒████▒▒██████▒▒",
            "░ ▒▓ ░▒▓░░▒▓▒ ▒ ▒ ▒ ▒▓▒ ▒ ░  ▒ ░░      ██▒▒▒    ░ ▒░   ▒ ▒ ░ ▒░▒░▒░   ▒ ░░   ░░ ▒░ ░▒ ▒▓▒ ▒ ░",
            "  ░▒ ░ ▒░░░▒░ ░ ░ ░ ░▒  ░ ░    ░     ▓██ ░▒░    ░ ░░   ░ ▒░  ░ ▒ ▒░     ░     ░ ░  ░░ ░▒  ░ ░",
            "  ░░   ░  ░░░ ░ ░ ░  ░  ░    ░       ▒ ▒ ░░        ░   ░ ░ ░ ░ ░ ▒    ░         ░   ░  ░  ░  ",
            "   ░        ░           ░            ░ ░                 ░     ░ ░              ░  ░      ░  ",
            "                                     ░ ░                                                     ",
        ];

        let title_rect = ratatui::layout::Rect::new(
            (f.area().width.saturating_sub(TITLE_SIZE.1)) / 2,
            (f.area().height.saturating_sub(TITLE_SIZE.0)) / 5,
            TITLE_SIZE.1,
            TITLE_SIZE.0,
        );

        let title = Paragraph::new(
            title_text
                .iter()
                .map(|x| Line::raw(String::from(*x)))
                .collect::<Vec<_>>(),
        )
        .red()
        .alignment(Alignment::Center);

        f.render_widget(title, title_rect);

        match &self.state {
            TitleScreenState::Options => self.render_options(f),
            TitleScreenState::Stats => self.render_stats(f),
            TitleScreenState::EntryPicker(ep) => {
                self.render_options(f);
                self.render_entry_picker(f, ep);
            }
            TitleScreenState::TagFilter(tf) => {
                self.render_options(f);
                self.render_tag_filter(f, tf);
            }
        }

        if self.show_help {
            self.render_help_overlay(f);
        }
    }

    fn render_help_overlay(&self, f: &mut Frame) {
        let area = centered_rect(36, 11, f.area());

        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .title("Keyboard Shortcuts")
            .on_dark_gray();

        let g = Color::Green;
        let w = Color::White;

        let lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled("  Title Screen:", w)]),
            Line::from(vec![
                Span::styled("    T              ", g),
                Span::styled("Open today's entry", w),
            ]),
            Line::from(vec![
                Span::styled("    O              ", g),
                Span::styled("Open old entry", w),
            ]),
            Line::from(vec![
                Span::styled("    C              ", g),
                Span::styled("Open calendar", w),
            ]),
            Line::from(vec![
                Span::styled("    S              ", g),
                Span::styled("View stats", w),
            ]),
            Line::from(vec![
                Span::styled("    Q              ", g),
                Span::styled("Exit", w),
            ]),
            Line::from(vec![
                Span::styled("    ?              ", g),
                Span::styled("Toggle this help", w),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_options(&self, f: &mut Frame) {
        const OPTIONS_SIZE: (u16, u16) = (8, 22);
        let text = vec![
            Line::raw("T - Open today's entry"),
            Line::raw("O - Open old entry    "),
            Line::raw("C - Open calendar     "),
            Line::raw("G - Browse by tag     "),
            Line::raw("S - View stats        "),
            Line::raw("Q or Esc - Exit       "),
        ];
        let rect = ratatui::layout::Rect::new(
            (f.area().width.saturating_sub(OPTIONS_SIZE.1)) / 2,
            (f.area().height.saturating_sub(OPTIONS_SIZE.0)) * 3 / 5,
            OPTIONS_SIZE.1,
            OPTIONS_SIZE.0,
        );
        let paragraph = Paragraph::new(text).blue().alignment(Alignment::Center);
        f.render_widget(paragraph, rect);
    }

    fn render_stats(&self, f: &mut Frame) {
        const STATS_SIZE: (u16, u16) = (7, 20);
        let text = vec![
            Line::raw(format!("Days Entered:   {:>4}", self.note_count)),
            Line::raw(format!("Total Words:    {:>4}", self.word_count)),
            Line::raw(""),
            Line::raw("S - Go back  "),
        ];
        let rect = ratatui::layout::Rect::new(
            (f.area().width.saturating_sub(STATS_SIZE.1)) / 2,
            (f.area().height.saturating_sub(STATS_SIZE.0)) * 3 / 5,
            STATS_SIZE.1,
            STATS_SIZE.0,
        );
        let paragraph = Paragraph::new(text).blue().alignment(Alignment::Center);
        f.render_widget(paragraph, rect);
    }

    fn render_entry_picker(&self, f: &mut Frame, ep: &EntryPicker) {
        const SIZE: (u16, u16) = (5, 30);
        let rect = ratatui::layout::Rect::new(
            (f.area().width.saturating_sub(SIZE.1)) / 2,
            (f.area().height.saturating_sub(SIZE.0)) / 2,
            SIZE.1,
            SIZE.0,
        );
        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .title("Entry picker");

        let date = format!("{}.{}.{}", &ep.input[0..2], &ep.input[2..4], &ep.input[4..]);
        let split = ep.cursor + ep.cursor / 2;
        let spans = vec![
            Span::from("Enter date:         "),
            Span::from(date.split_at(split).0),
            Span::from(date.chars().nth(split).unwrap().to_string())
                .black()
                .on_white(),
            Span::from(date.split_at(split + 1).1),
        ];
        let lines = vec![
            Line::from(spans),
            Line::from(""),
            Line::from("Enter - Confirm   Esc - Exit"),
        ];
        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(Clear, rect);
        f.render_widget(paragraph, rect);
    }

    fn render_tag_filter(&self, f: &mut Frame, tf: &TagFilterState) {
        let height = 14u16;
        let width = 40u16;
        let rect = centered_rect(width, height, f.area());

        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .title("Browse by tag");

        let mut lines = Vec::new();

        if tf.browsing_notes {
            if tf.filtered_notes.is_empty() {
                lines.push(Line::from("No notes found."));
            } else {
                for (i, note) in tf.filtered_notes.iter().enumerate() {
                    let date_str = note.creation_date.format("%d/%m/%Y").to_string();
                    let prefix = format!(" {} ", date_str);
                    if i == tf.note_selected {
                        lines.push(Line::from(vec![
                            Span::raw(" > ").green(),
                            Span::raw(prefix).black().on_white(),
                        ]));
                    } else {
                        lines.push(Line::from(format!("   {}", prefix)));
                    }
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Enter - Open   Esc - Back"));
        } else {
            if tf.tags.is_empty() {
                lines.push(Line::from("No tags found."));
            } else {
                for (i, tag) in tf.tags.iter().enumerate() {
                    if i == tf.selected {
                        lines.push(Line::from(vec![
                            Span::raw(" > ").green(),
                            Span::raw(format!("#{}", tag)).black().on_white(),
                        ]));
                    } else {
                        lines.push(Line::from(format!("   #{}", tag)));
                    }
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Enter - Select   Esc - Back"));
        }

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(Clear, rect);
        f.render_widget(paragraph, rect);
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width.min(area.width),
        height.min(area.height),
    )
}
