#![feature(exclusive_range_pattern)]
#![feature(drain_filter)]
#![feature(backtrace)]
#![feature(thread_id_value)]

mod app;
mod logdata;
mod parser;
mod ui;

use crate::logdata::LogData;
use crate::parser::LogParser;
use crate::ui::filter::DataModelFilter;
use app::App;
use clap::Parser;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, RwLock};
use tui::backend::CrosstermBackend;
use tui::layout::Constraint;
use tui::Terminal;

#[cfg(windows)]
const LINE_ENDING: &'static str = "\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &'static str = "\n";

fn parse_kv(kv: &str) -> HashMap<String, Regex> {
    let _lines = kv.split(LINE_ENDING);
    kv.split(LINE_ENDING)
        .map(|line| {
            let mut pair = line.splitn(2, "=");
            match (pair.next(), pair.next()) {
                (Some(key), Some(value)) => (key, regex::Regex::new(value).unwrap()),
                _ => unreachable!(),
            }
        })
        .map(|(k, v)| (String::from(k), v))
        .collect::<HashMap<_, _>>()
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    directory: String,

    #[clap(short, long, value_parser)]
    group: String,

    #[clap(short, long, value_parser)]
    filter: String,
}

lazy_static::lazy_static! {
    static ref FILTER: RwLock<HashMap<String, Regex>> = RwLock::new(HashMap::new());
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    *FILTER.write().unwrap() = parse_kv(args.filter.as_str());
    let channel = LogParser::parse(args.directory.clone());
    let log_data = DataModelFilter::new(
        Arc::new(LogData::new(
            channel,
            vec!["time", "event", "duration", "process", "OSThread"]
                .into_iter()
                .map(String::from)
                .collect(),
        )),
        HashMap::new(),
    );

    App::new(
        log_data,
        vec![
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ],
    )
    .run(&mut terminal)?;

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
