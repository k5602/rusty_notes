use chrono::{DateTime, Datelike, Days, Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

mod copy_paste;
mod cursor;
mod editor_state;
mod scroll;
mod search;
mod text;
mod undo;

pub use cursor::Cursor;
pub use editor_state::EditorState;
pub use search::{Replace, Search};
pub use text::Text;
pub use undo::UndoStack;

use crate::action::Action;
use crate::note::Note;

#[derive(Debug)]
pub struct Editor {
    pub state: EditorState,
    pub text: Text,
    pub creation_date: NaiveDate,
    pub last_edited: DateTime<Local>,
    pub scroll_offset: (u16, u16),
    pub screen_size: (u16, u16),
    pub write: bool,
    pub side_panel: bool,
    pub show_help: bool,
    undo_stack: UndoStack,
}

impl Editor {
    pub fn new() -> Editor {
        let now = Local::now();
        Editor {
            state: EditorState::Edit,
            text: Text::new(),
            creation_date: now.date_naive(),
            last_edited: now,
            scroll_offset: (0, 0),
            screen_size: (0, 0),
            write: false,
            side_panel: true,
            show_help: false,
            undo_stack: UndoStack::new(),
        }
    }

    pub fn from_note(note: Note) -> Editor {
        Editor {
            state: EditorState::Edit,
            text: Text::from_string(note.text),
            creation_date: note.creation_date,
            last_edited: note.last_edited,
            scroll_offset: (0, 0),
            screen_size: (0, 0),
            write: false,
            side_panel: true,
            show_help: false,
            undo_stack: UndoStack::new(),
        }
    }

    pub fn current_note_text(&self) -> String {
        self.text.lines.join("\n")
    }

    fn save_snapshot(&mut self) {
        self.undo_stack.push(undo::Snapshot {
            text: self.text.clone(),
            cursor: self.text.cursor,
        });
    }

    pub fn handle_event(&mut self, key: KeyEvent) -> Action {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('w') => {
                    self.write = true;
                    return Action::Noop;
                }
                KeyCode::Char('j') => {
                    self.side_panel = !self.side_panel;
                    return Action::Noop;
                }
                _ => {}
            }
        }

        match self.state {
            EditorState::Edit => self.handle_edit(key),
            EditorState::Search(_) => self.handle_search(key),
            EditorState::Replace(_) => self.handle_replace(key),
            EditorState::Exit => Action::BackToTitle,
        }
    }

    fn handle_edit(&mut self, key: KeyEvent) -> Action {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('q') => {
                    self.state = EditorState::Exit;
                    return Action::BackToTitle;
                }
                KeyCode::Char('f') => {
                    self.state = EditorState::Search(Search::new());
                    return Action::Noop;
                }
                KeyCode::Char('r') => {
                    self.state = EditorState::Replace(Replace::new());
                    return Action::Noop;
                }
                KeyCode::Up => {
                    self.scroll_up();
                    return Action::Noop;
                }
                KeyCode::Down => {
                    self.scroll_down();
                    return Action::Noop;
                }
                KeyCode::Char('z') => {
                    if let Some(snapshot) = self.undo_stack.undo(undo::Snapshot {
                        text: self.text.clone(),
                        cursor: self.text.cursor,
                    }) {
                        self.text = snapshot.text;
                        self.text.focus = true;
                        self.write = true;
                    }
                    return Action::Noop;
                }
                KeyCode::Char('y') => {
                    if let Some(snapshot) = self.undo_stack.redo(undo::Snapshot {
                        text: self.text.clone(),
                        cursor: self.text.cursor,
                    }) {
                        self.text = snapshot.text;
                        self.text.focus = true;
                        self.write = true;
                    }
                    return Action::Noop;
                }
                KeyCode::Char('t') => {
                    self.toggle_task_checkbox();
                    self.write = true;
                    return Action::Noop;
                }
                KeyCode::Char('?') => {
                    self.show_help = !self.show_help;
                    return Action::Noop;
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Esc => {
                    self.state = EditorState::Exit;
                    return Action::BackToTitle;
                }
                KeyCode::Home => {
                    self.text.cursor.1 = 0;
                    self.text.focus = true;
                    return Action::Noop;
                }
                KeyCode::End => {
                    self.text.cursor.1 = self.text.lines[self.text.cursor.0].len();
                    self.text.focus = true;
                    return Action::Noop;
                }
                KeyCode::PageUp => {
                    let scroll = self.screen_size.0.saturating_sub(2) as usize;
                    self.text.cursor.0 = self.text.cursor.0.saturating_sub(scroll);
                    self.text.focus = true;
                    return Action::Noop;
                }
                KeyCode::PageDown => {
                    let scroll = self.screen_size.0.saturating_sub(2) as usize;
                    self.text.cursor.0 =
                        (self.text.cursor.0 + scroll).min(self.text.lines.len() - 1);
                    self.text.focus = true;
                    return Action::Noop;
                }
                _ => {}
            }
        }

        self.save_snapshot();
        text_input(&mut self.text, &key);
        self.write = true;
        self.focus_scroll_on_cursor();
        Action::Noop
    }

    fn handle_search(&mut self, key: KeyEvent) -> Action {
        if let EditorState::Search(ref mut search) = self.state {
            match key.code {
                KeyCode::Esc => {
                    self.state = EditorState::Edit;
                    return Action::Noop;
                }
                KeyCode::Enter => {
                    self.state = EditorState::Edit;
                    return Action::Noop;
                }
                _ => {
                    text_input(&mut search.text, &key);
                }
            }
        }
        Action::Noop
    }

    fn handle_replace(&mut self, key: KeyEvent) -> Action {
        if let EditorState::Replace(ref mut replace) = self.state {
            match key.code {
                KeyCode::Esc => {
                    self.state = EditorState::Edit;
                    return Action::Noop;
                }
                KeyCode::Enter => {
                    if let Some(replacement) = replace.get_replacement() {
                        self.text = replacement;
                        self.write = true;
                    }
                    self.state = EditorState::Edit;
                    return Action::Noop;
                }
                _ => {
                    text_input(&mut replace.text, &key);
                }
            }
        }
        Action::Noop
    }

    pub fn render(&mut self, f: &mut Frame) {
        if self.side_panel {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(1), Constraint::Length(22)])
                .split(f.area());

            self.render_editor_area(f, chunks[0]);

            let side_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(10)])
                .split(chunks[1]);

            self.render_side_details(f, side_chunks[0]);
            self.render_mini_calendar(f, side_chunks[1]);
        } else {
            self.render_editor_area(f, f.area());
        }

        if self.show_help {
            self.render_help_overlay(f);
        }
    }

    fn render_help_overlay(&self, f: &mut Frame) {
        let area = centered_rect(50, 20, f.area());

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
            Line::from(vec![Span::styled("  Navigation:", w)]),
            Line::from(vec![
                Span::styled("    ← → ↑ ↓           ", g),
                Span::styled("Move cursor", w),
            ]),
            Line::from(vec![
                Span::styled("    Home/End          ", g),
                Span::styled("Start/End of line", w),
            ]),
            Line::from(vec![
                Span::styled("    PgUp/PgDn         ", g),
                Span::styled("Scroll page", w),
            ]),
            Line::from(vec![
                Span::styled("    Ctrl+↑/↓          ", g),
                Span::styled("Scroll view", w),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled("  Editing:", w)]),
            Line::from(vec![
                Span::styled("    Ctrl+W            ", g),
                Span::styled("Save note", w),
            ]),
            Line::from(vec![
                Span::styled("    Ctrl+Q / Esc      ", g),
                Span::styled("Back to title", w),
            ]),
            Line::from(vec![
                Span::styled("    Ctrl+Z            ", g),
                Span::styled("Undo", w),
            ]),
            Line::from(vec![
                Span::styled("    Ctrl+Y            ", g),
                Span::styled("Redo", w),
            ]),
            Line::from(vec![
                Span::styled("    Ctrl+T            ", g),
                Span::styled("Toggle task checkbox", w),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled("  Search & Replace:", w)]),
            Line::from(vec![
                Span::styled("    Ctrl+F            ", g),
                Span::styled("Search", w),
            ]),
            Line::from(vec![
                Span::styled("    Ctrl+R            ", g),
                Span::styled("Replace", w),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled("  Other:", w)]),
            Line::from(vec![
                Span::styled("    Ctrl+J            ", g),
                Span::styled("Toggle side panel", w),
            ]),
            Line::from(vec![
                Span::styled("    Ctrl+?           ", g),
                Span::styled("Toggle this help", w),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_editor_area(&mut self, f: &mut Frame, rect: Rect) {
        match &self.state {
            EditorState::Search(search) => {
                let search_text = search.text.lines[0].clone();
                let search_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(rect);
                self.render_text(f, search_chunks[0]);
                self.render_search_bar(f, search_chunks[1], &search_text);
            }
            EditorState::Replace(replace) => {
                let search_text = replace.search_text();
                let replace_text = replace.replacement_text();
                let replace_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ])
                    .split(rect);
                self.render_text(f, replace_chunks[0]);
                self.render_search_bar(f, replace_chunks[1], &search_text);
                self.render_replace_bar(f, replace_chunks[2], &replace_text);
            }
            _ => {
                self.render_text(f, rect);
            }
        }
    }

    fn render_text(&mut self, f: &mut Frame, rect: Rect) {
        let editor_block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .title(Line::from(
                self.creation_date.format("%d/%m/%Y").to_string(),
            ))
            .title(Line::from("^_^").right_aligned())
            .title_bottom(
                Line::from(format!(
                    "{}:{}",
                    self.text.cursor.0 + 1,
                    self.text.cursor.1 + 1
                ))
                .right_aligned(),
            );

        let mut lines: Vec<Line> = self
            .text
            .lines
            .iter()
            .map(|line| render_task_line(line))
            .collect();

        if let Some(selection_start) = &self.text.selection_start {
            highlight_selection(&mut lines, &self.text.cursor, selection_start)
        } else {
            highlight_cursor(&mut lines, &self.text.cursor);
        }

        if let EditorState::Search(search) = &self.state {
            highlight_search(&mut lines, &search.text.lines[0]);
        }

        let inner_rect = rect.inner(Margin::new(1, 1));
        self.screen_size = (inner_rect.height, inner_rect.width.saturating_sub(4));

        let paragraph = Paragraph::new(lines)
            .block(editor_block)
            .scroll(self.scroll_offset)
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, rect);
    }

    fn render_side_details(&self, f: &mut Frame, rect: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .title("Details");

        let word_count: usize = self
            .text
            .lines
            .iter()
            .map(|line| {
                if line.trim().is_empty() {
                    0
                } else {
                    line.split_whitespace().count()
                }
            })
            .sum();

        let full_text = self.text.lines.join("\n");
        let tags = crate::database::extract_tags(&full_text);

        let total_tasks: usize = self
            .text
            .lines
            .iter()
            .filter(|l| l.contains("- [ ]") || l.contains("- [x]") || l.contains("- [X]"))
            .count();
        let done_tasks: usize = self
            .text
            .lines
            .iter()
            .filter(|l| l.contains("- [x]") || l.contains("- [X]"))
            .count();

        let mut lines = vec![
            Line::from("Date:"),
            Line::from(self.creation_date.format("%d/%m/%Y").to_string()),
            Line::from(""),
            Line::from("Edited:"),
            Line::from(self.last_edited.format("%d/%m/%Y %H:%M").to_string()),
            Line::from(""),
            Line::from(format!("Words: {}", word_count)),
        ];

        if total_tasks > 0 {
            lines.push(Line::from(format!(
                "Tasks: {}/{} done",
                done_tasks, total_tasks
            )));
        }

        if !tags.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from("Tags:"));
            let tag_line = tags
                .iter()
                .map(|t| format!("#{}", t))
                .collect::<Vec<_>>()
                .join(" ");
            for chunk in tag_line.as_bytes().chunks(20) {
                lines.push(Line::from(String::from_utf8_lossy(chunk).to_string()));
            }
        }

        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block);
        f.render_widget(paragraph, rect);
    }

    fn render_mini_calendar(&self, f: &mut Frame, rect: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .title("Month");

        let now = Local::now();
        let date = now.date_naive();
        let lines = get_month_in_lines(date);
        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block);
        f.render_widget(paragraph, rect);
    }

    fn render_search_bar(&self, f: &mut Frame, rect: Rect, search_text: &str) {
        let line = Line::from(vec![
            Span::raw(" search: "),
            Span::raw(search_text).black().on_white(),
        ]);
        let paragraph = Paragraph::new(line);
        f.render_widget(paragraph, rect);
    }

    fn render_replace_bar(&self, f: &mut Frame, rect: Rect, replace_text: &str) {
        let line = Line::from(vec![
            Span::raw(" replace: "),
            Span::raw(replace_text).black().on_white(),
        ]);
        let paragraph = Paragraph::new(line);
        f.render_widget(paragraph, rect);
    }

    fn toggle_task_checkbox(&mut self) {
        let line = &mut self.text.lines[self.text.cursor.0];
        if let Some(pos) = line.find("- [ ]") {
            line.replace_range(pos..pos + 5, "- [x]");
        } else if let Some(pos) = line.find("- [x]") {
            line.replace_range(pos..pos + 5, "- [ ]");
        } else if let Some(pos) = line.find("- [X]") {
            line.replace_range(pos..pos + 5, "- [ ]");
        }
        self.text.focus = true;
    }
}

fn render_task_line(line: &str) -> Line<'static> {
    if let Some(pos) = line.find("- [ ]") {
        let before = &line[..pos];
        let after = &line[pos + 5..];
        return Line::from(vec![
            Span::raw(format!("{} ", before)),
            Span::raw("[ ]").yellow(),
            Span::raw(format!(" {} ", after)),
        ]);
    }
    if let Some(pos) = line.find("- [x]") {
        let before = &line[..pos];
        let after = &line[pos + 5..];
        return Line::from(vec![
            Span::raw(format!("{} ", before)),
            Span::raw("[x]").green(),
            Span::raw(format!(" {} ", after)).dark_gray(),
        ]);
    }
    if let Some(pos) = line.find("- [X]") {
        let before = &line[..pos];
        let after = &line[pos + 5..];
        return Line::from(vec![
            Span::raw(format!("{} ", before)),
            Span::raw("[x]").green(),
            Span::raw(format!(" {} ", after)).dark_gray(),
        ]);
    }
    Line::from(format!("{} ", line))
}

fn text_input(text: &mut Text, key: &KeyEvent) {
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => text.copy(),
            KeyCode::Char('p') => text.paste(),
            KeyCode::Char('x') => text.cut(),
            KeyCode::Backspace => text.backspace_word(),
            KeyCode::Delete => text.delete_word(),
            KeyCode::Left => text.move_left_word(shift),
            KeyCode::Right => text.move_right_word(shift),
            _ => (),
        }
    } else {
        match key.code {
            KeyCode::Char(c) => text.insert_char(c),
            KeyCode::Enter => text.enter(),
            KeyCode::Backspace => text.backspace(),
            KeyCode::Delete => text.delete(),
            KeyCode::Left => text.move_left(shift),
            KeyCode::Right => text.move_right(shift),
            KeyCode::Up => text.move_up(shift),
            KeyCode::Down => text.move_down(shift),
            _ => (),
        }
    }
}

fn highlight_cursor(lines: &mut [Line], cursor: &Cursor) {
    let cursor_line = lines[cursor.0].to_string();
    let chars: Vec<char> = cursor_line.chars().collect();
    let left: String = chars[..cursor.1.min(chars.len())].iter().collect();
    let cursor_char = chars.get(cursor.1).unwrap_or(&'_');
    let right: String = chars[(cursor.1 + 1).min(chars.len())..].iter().collect();

    let display_char = if *cursor_char == ' ' {
        '_'
    } else {
        *cursor_char
    };

    lines[cursor.0] = Line::from(vec![
        Span::raw(left),
        Span::raw(display_char.to_string()).black().on_white(),
        Span::raw(right),
    ]);
}

fn highlight_selection(lines: &mut [Line], cursor: &Cursor, selection_start: &Cursor) {
    let mut start = cursor;
    let mut end = selection_start;
    if cursor > selection_start {
        start = selection_start;
        end = cursor;
    }

    if start.0 == end.0 {
        let cursor_line = lines[start.0].to_string();
        let chars: Vec<char> = cursor_line.chars().collect();
        let left: String = chars[..start.1.min(chars.len())].iter().collect();
        let selected: String = chars[start.1.min(chars.len())..=(end.1.min(chars.len() - 1))]
            .iter()
            .collect();
        let right: String = chars[(end.1 + 1).min(chars.len())..].iter().collect();

        lines[start.0] = Line::from(vec![
            Span::raw(left),
            Span::raw(selected).black().on_light_blue(),
            Span::raw(right),
        ]);
        return;
    }

    let start_line = lines[start.0].to_string();
    let start_chars: Vec<char> = start_line.chars().collect();
    let start_left: String = start_chars[..start.1.min(start_chars.len())]
        .iter()
        .collect();
    let start_right: String = start_chars[start.1.min(start_chars.len())..]
        .iter()
        .collect();

    lines[start.0] = Line::from(vec![
        Span::raw(start_left),
        Span::raw(start_right).black().on_light_blue(),
    ]);

    for line_id in (start.0..end.0).skip(1) {
        lines[line_id] = Line::from(
            Span::raw(lines[line_id].to_string())
                .black()
                .on_light_blue(),
        );
    }

    let end_line = lines[end.0].to_string();
    let end_chars: Vec<char> = end_line.chars().collect();
    let end_left: String = end_chars[..end.1.min(end_chars.len())].iter().collect();
    let end_right: String = end_chars[end.1.min(end_chars.len())..].iter().collect();

    lines[end.0] = Line::from(vec![
        Span::raw(end_left).black().on_light_blue(),
        Span::raw(end_right),
    ]);
}

fn highlight_search(lines: &mut [Line], search: &str) {
    if search.is_empty() {
        return;
    }

    for line in lines.iter_mut() {
        let line_str = line.to_string();
        if !line_str.contains(search) {
            continue;
        }

        let mut spans = Vec::new();
        let mut last_pos = 0;

        while let Some(pos) = line_str[last_pos..].find(search) {
            let abs_pos = last_pos + pos;
            if abs_pos > last_pos {
                spans.push(Span::raw(line_str[last_pos..abs_pos].to_string()));
            }
            spans.push(
                Span::raw(line_str[abs_pos..abs_pos + search.len()].to_string())
                    .black()
                    .on_green(),
            );
            last_pos = abs_pos + search.len();
        }
        if last_pos < line_str.len() {
            spans.push(Span::raw(line_str[last_pos..].to_string()));
        }

        line.spans = spans;
    }
}

fn get_month_in_lines(date: NaiveDate) -> Vec<Line<'static>> {
    let month = date.format("%B").to_string();
    let year = date.year().to_string();
    let spaces = 20 - month.len() - year.len();
    let mut lines = vec![
        Line::from(format!("{}{:>spaces$}{}", month, "", year)).yellow(),
        Line::from(" M  T  W  T  F  S  S").bold().green(),
    ];

    let now = Local::now().date_naive();
    let start_offset = ((7 + date.weekday().num_days_from_monday() as i16 - date.day0() as i16 % 7)
        % 7
        * 3) as usize;
    let mut current = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap();
    let mut line = Line::from(format!("{:<start_offset$}", ""));

    while current.month() == date.month() {
        let mut span = Span::from(format!("{:>2} ", current.day()));
        if current.weekday().num_days_from_monday() >= 5 {
            span = span.red();
        }
        if current == now {
            span = span.on_green();
        }
        line.spans.push(span);
        current = current.checked_add_days(Days::new(1)).unwrap();
    }

    lines.push(line);
    lines
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width.min(area.width),
        height.min(area.height),
    )
}
