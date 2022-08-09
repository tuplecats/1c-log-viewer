use crate::ui::table::TableView;
use crate::{DataModelFilter, FILTER};
use crossterm::event;
use crossterm::event::{Event, KeyCode};
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
use tui::backend::Backend;
use tui::layout::{Constraint, Direction, Layout};
use tui::{Frame, Terminal};

// group=event,process
// (event,process)
pub enum AppState {
    AggregateWindow,
    DataWindow,
}

pub struct App {
    pub app_state: AppState,
    pub table: TableView,
}

impl App {
    pub fn new(model: DataModelFilter, widths: Vec<Constraint>) -> Self {
        Self {
            app_state: AppState::DataWindow,
            table: TableView::new(model, widths),
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), Box<dyn Error>> {
        loop {
            terminal.draw(|f| ui(f, self))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('f') => {
                            self.table.set_filter(FILTER.read().unwrap().clone())
                            //self.log_data.set_filter(FILTER.read().unwrap().clone())
                        }
                        KeyCode::Char('g') => {
                            self.table.set_filter(HashMap::new())
                            //self.log_data.set_filter(HashMap::new())
                        }
                        KeyCode::Down => self.next(),
                        KeyCode::Up => self.previous(),
                        _ => {}
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn current(&self) -> Option<usize> {
        None
    }

    pub fn next(&mut self) {
        self.table.next()
    }

    pub fn previous(&mut self) {
        self.table.prev()
    }

    pub fn data_window(&mut self) {
        todo!()
    }

    pub fn aggregate_window(&mut self) {
        self.app_state = AppState::AggregateWindow;
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let rects = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());

    f.render_widget(app.table.widget(), rects[0]);
}
