use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};

const TICK_RATE: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub enum AppEvent {
    Key(crossterm::event::KeyEvent),
    Resize(u16, u16),
    Tick,
}

pub fn next_event() -> Result<AppEvent> {
    if event::poll(TICK_RATE)? {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => Ok(AppEvent::Key(key)),
            Event::Resize(w, h) => Ok(AppEvent::Resize(w, h)),
            _ => next_event(),
        }
    } else {
        Ok(AppEvent::Tick)
    }
}
