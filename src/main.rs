mod app;
mod parser;
mod ui;
mod util;

/// TODO:
/// 1. Добить запрос с разными типами
/// 2. Индексация по полям
/// 3. Читать файлы и запоминать только байты конкретных данных


use crate::parser::LogParser;
use app::App;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::error::Error;
use tui::{backend::CrosstermBackend, Terminal};


use parser::logdata::LogCollection;
use crate::util::parse_date;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None, verbatim_doc_comment)]
struct Args {
    /// Путь к директории с файлами логов
    /// (Также ищет файлы в поддиректориях)
    #[clap(short, long, value_parser, verbatim_doc_comment)]
    directory: String,

    /// Временая точка начала чтения логов.
    /// Формат: now-{digit}{s/m/h/d/w}
    /// Пример: now-1d или now-30s
    #[clap(long, value_parser, verbatim_doc_comment)]
    from: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let date = match &args.from {
        Some(value) => Some(parse_date(value.as_str())?),
        None => None,
    };

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    App::new(args.directory.as_str(), date).run(&mut terminal)?;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
