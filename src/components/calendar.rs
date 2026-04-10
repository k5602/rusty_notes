use chrono::{Datelike, Days, Local, Months, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::{action::Action, database::Database};

#[derive(Debug)]
pub enum CurrentlyEditing {
    Year,
    Month,
    Day,
}

pub struct CalendarComponent {
    pub date: NaiveDate,
    pub editing: CurrentlyEditing,
    open: bool,
    show_help: bool,
}

impl CalendarComponent {
    pub fn new() -> Self {
        Self {
            date: Local::now().date_naive(),
            editing: CurrentlyEditing::Month,
            open: false,
            show_help: false,
        }
    }

    pub fn handle_event(&mut self, key: KeyEvent) -> Action {
        if self.open {
            self.open = false;
            return Action::OpenNote(self.date);
        }

        match key.code {
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            KeyCode::Left | KeyCode::Char('h') => self.move_left(),
            KeyCode::Right | KeyCode::Char('l') => self.move_right(),
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Enter => self.choose_selection(),
            KeyCode::Backspace => self.backtrace_selection(),
            KeyCode::Char('q') | KeyCode::Esc => return Action::BackToTitle,
            _ => {}
        }

        Action::Noop
    }

    fn move_left(&mut self) {
        match self.editing {
            CurrentlyEditing::Year => {
                self.date = self.date.checked_sub_months(Months::new(12)).unwrap();
            }
            CurrentlyEditing::Month => {
                self.date = self.date.checked_sub_months(Months::new(1)).unwrap();
            }
            CurrentlyEditing::Day => {
                self.date = self.date.checked_sub_days(Days::new(1)).unwrap();
            }
        }
    }

    fn move_right(&mut self) {
        match self.editing {
            CurrentlyEditing::Year => {
                self.date = self.date.checked_add_months(Months::new(12)).unwrap();
            }
            CurrentlyEditing::Month => {
                self.date = self.date.checked_add_months(Months::new(1)).unwrap();
            }
            CurrentlyEditing::Day => {
                self.date = self.date.checked_add_days(Days::new(1)).unwrap();
            }
        }
    }

    fn move_up(&mut self) {
        match self.editing {
            CurrentlyEditing::Year => {
                self.date = self.date.checked_sub_months(Months::new(12)).unwrap();
            }
            CurrentlyEditing::Month => {
                self.date = self.date.checked_sub_months(Months::new(4)).unwrap();
            }
            CurrentlyEditing::Day => {
                self.date = self.date.checked_sub_days(Days::new(7)).unwrap();
            }
        }
    }

    fn move_down(&mut self) {
        match self.editing {
            CurrentlyEditing::Year => {
                self.date = self.date.checked_add_months(Months::new(12)).unwrap();
            }
            CurrentlyEditing::Month => {
                self.date = self.date.checked_add_months(Months::new(4)).unwrap();
            }
            CurrentlyEditing::Day => {
                self.date = self.date.checked_add_days(Days::new(7)).unwrap();
            }
        }
    }

    fn choose_selection(&mut self) {
        match self.editing {
            CurrentlyEditing::Year => self.editing = CurrentlyEditing::Month,
            CurrentlyEditing::Month => self.editing = CurrentlyEditing::Day,
            CurrentlyEditing::Day => self.open = true,
        }
    }

    fn backtrace_selection(&mut self) {
        match self.editing {
            CurrentlyEditing::Year => {}
            CurrentlyEditing::Month => self.editing = CurrentlyEditing::Year,
            CurrentlyEditing::Day => self.editing = CurrentlyEditing::Month,
        }
    }

    pub fn render(&mut self, f: &mut Frame, db: &Database) {
        let rect = f.area();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .title(Line::from("Calendar"))
            .title(Line::from("^_^").right_aligned());
        f.render_widget(block, rect);

        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Min(8), Constraint::Min(8)])
            .split(rect.inner(Margin::new(1, 1)));

        let mut start = NaiveDate::from_ymd_opt(self.date.year(), 1, 1).unwrap();

        for chunk in vertical_chunks.iter() {
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(20),
                    Constraint::Min(20),
                    Constraint::Min(20),
                    Constraint::Min(20),
                ])
                .split(*chunk);

            for chunk in horizontal_chunks.iter() {
                let month_rect = Rect::new(
                    chunk.x + (chunk.width.saturating_sub(20)) / 2,
                    chunk.y + (chunk.height.saturating_sub(8)) / 2,
                    20.min(chunk.width),
                    8.min(chunk.height),
                );

                let mut lines = get_month_in_lines(start);

                if let CurrentlyEditing::Month = self.editing
                    && start.month() == self.date.month()
                {
                    lines = lines.iter().map(|line| line.clone().on_blue()).collect();
                }

                let dates = db
                    .get_dates_with_notes(start.year(), start.month())
                    .unwrap_or_default();
                highlight_dates(
                    &mut lines,
                    start.month(),
                    start.year(),
                    Some(self.date),
                    &dates,
                );

                let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
                start = start.checked_add_months(Months::new(1)).unwrap();
                f.render_widget(paragraph, month_rect);
            }
        }

        if self.show_help {
            self.render_help_overlay(f);
        }
    }

    fn render_help_overlay(&self, f: &mut Frame) {
        let area = centered_rect(42, 11, f.area());

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
            Line::from(vec![Span::styled("  Calendar:", w)]),
            Line::from(vec![
                Span::styled("    ← → ↑ ↓ / h j k l  ", g),
                Span::styled("Navigate", w),
            ]),
            Line::from(vec![
                Span::styled("    Enter                ", g),
                Span::styled("Drill down", w),
            ]),
            Line::from(vec![
                Span::styled("    Backspace            ", g),
                Span::styled("Go back", w),
            ]),
            Line::from(vec![
                Span::styled("    Q / Esc              ", g),
                Span::styled("Back to title", w),
            ]),
            Line::from(vec![
                Span::styled("    ?                    ", g),
                Span::styled("Toggle this help", w),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
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

fn highlight_dates(
    lines: &mut [Line],
    month: u32,
    year: i32,
    cursor: Option<NaiveDate>,
    dates: &[NaiveDate],
) {
    for span in lines[2].spans.iter_mut() {
        let day_str = span.to_string().trim().to_string();
        if dates
            .iter()
            .any(|d| d.year() == year && d.month() == month && d.day().to_string() == day_str)
        {
            *span = span.clone().on_yellow();
        }
        if let Some(cursor) = cursor
            && month == cursor.month()
            && cursor.day().to_string() == day_str
        {
            *span = span.clone().on_blue();
        }
    }
}
