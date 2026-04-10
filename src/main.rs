use anyhow::Result;
use clap::Parser;

mod action;
mod app;
mod components;
mod config;
mod database;
mod editor;
mod event;
mod note;
mod terminal;

use app::App;
use config::Config;
use database::Database;

#[derive(Parser)]
#[command(
    name = "rusty-notes",
    version,
    about = "A TUI note-taking and task tracking app"
)]
struct Cli {
    #[arg(short, long, help = "Append text to today's note")]
    capture: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config = Config::load()?;
    let db_path = config.db_path()?;

    if let Some(text) = cli.capture {
        let db = Database::new(&db_path)?;
        let today = chrono::Local::now().date_naive();
        db.append_to_note(&today, &text)?;
        println!("Captured to {}: {}", today.format("%d/%m/%Y"), text);
        return Ok(());
    }

    let db = Database::new(&db_path)?;
    let mut app = App::new(db, config);

    let mut terminal = terminal::init()?;
    let result = app.run(&mut terminal);
    terminal::restore()?;

    result
}
