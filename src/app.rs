use std::{error::Error, time::Duration};
use std::cell::RefCell;
use std::rc::Rc;
use chrono::NaiveDateTime;
use crossterm::{
    event,
    event::{Event, KeyCode},
};
use crossterm::event::KeyModifiers;
use tui::{
    backend::Backend,
    Frame,
    layout::{Constraint, Direction, Layout}, Terminal,
};
use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::Paragraph;
use crate::{LogCollection, LogParser, ui::widgets::{WidgetExt, TableView, LineEdit}};
use crate::parser::{Compiler, FieldMap, Value};
use crate::ui::widgets::KeyValueView;

#[derive(Default)]
enum ActiveWidget {
    SearchBox,

    #[default]
    LogTable,

    InfoView,
}

pub struct App {
    pub table: Rc<RefCell<TableView>>,
    pub search: Rc<RefCell<LineEdit>>,
    pub text: Rc<RefCell<KeyValueView>>,
    pub log_data: Rc<RefCell<LogCollection>>,

    pub prev_size: (u16, u16),

    state: ActiveWidget,
}

impl App {
    pub fn new<T: Into<String>>(dir: T, date: Option<NaiveDateTime>) -> Self {
        let dir = dir.into();
        let widths = vec![
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ];

        let log_data = Rc::new(RefCell::new(
            LogCollection::new(
                LogParser::parse(dir, date)
            )
        ));

        let mut table_view = TableView::new(widths);
        table_view.set_model(log_data.clone());

        let app = Self {
            table: Rc::new(RefCell::new(table_view)),
            search: Rc::new(RefCell::new(LineEdit::new("Filter".into()))),
            text: Rc::new(RefCell::new(KeyValueView::new())),
            log_data: log_data.clone(),
            prev_size: (0, 0),
            state: ActiveWidget::default(),
        };

        app.table.borrow_mut().set_focus(true);

        let log_data = Rc::downgrade(&app.log_data);
        let table = Rc::downgrade(&app.table);
        app.search.borrow_mut().on_changed(move |sender| match log_data.upgrade() {
            Some(model) => {
                match model.borrow_mut().set_filter(sender.text().to_string()) {
                    Err(e) =>  {
                        sender.set_border_text(e.to_string());
                        sender.set_style(Style::default().fg(Color::Red));
                    },
                    _ => {
                        sender.set_border_text(String::new());
                        sender.set_style(Style::default());
                        if let Some(table) = table.upgrade() {
                            table.borrow_mut().reset_state();
                        }
                    },
                }
            },
            None => {}
        });

        let text = Rc::downgrade(&app.text);
        let log_data = Rc::downgrade(&app.log_data);
        app.table.borrow_mut().on_selection_changed(move |_sender, index| {
            if let (Some(log_data), Some(text)) = (log_data.upgrade(), text.upgrade()) {
                if let Some(index) = index {
                    if let Some(line) = log_data.borrow().line(index) {
                        text.borrow_mut().set_data(line.fields);
                        return
                    }
                }

                // Panic if we can't borrow. Because dont need reset state when filter from info widget.
                if let Ok(mut borrowed) = text.try_borrow_mut() {
                    borrowed.set_data(FieldMap::new());
                }
            }
        });

        let search = Rc::downgrade(&app.search);
        app.text.borrow_mut().on_add_to_filter(move |(key, value)| {
            if let Some(search) = search.upgrade() {
                let value = match value {
                    Value::String(s) => format!("\"{}\"", s),
                    Value::Number(n) => n.to_string(),
                    _ => unreachable!(),
                };


                let mut search_borrowed = search.borrow_mut();
                search_borrowed.show();
                let text = search_borrowed.text().to_string();
                if text.trim().is_empty() {
                    search_borrowed.set_text(format!(r#"WHERE {} = {}"#, key, value));
                }
                else if Compiler::new().compile(text.trim()).is_ok() {
                    search_borrowed.set_text(format!(r#"{} AND {} = {}"#, text, key, value));
                }
            }
        });

        app
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), Box<dyn Error>> {
        loop {

            terminal.draw(|f| ui(f, self))?;

            if event::poll(Duration::from_millis(100))? {
                let event = event::read()?;
                match event {
                    Event::Key(key) => match key.code {
                        KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => return Ok(()),
                        KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
                            match self.state {
                                ActiveWidget::LogTable | ActiveWidget::InfoView => {
                                    self.search.borrow_mut().set_visible(true);
                                    self.set_active_widget(ActiveWidget::SearchBox);
                                },
                                ActiveWidget::SearchBox => {
                                    self.search.borrow_mut().set_visible(false);
                                    self.set_active_widget(ActiveWidget::LogTable);
                                },
                            }
                        },
                        KeyCode::Tab => { // Next active widget
                            match self.state {
                                ActiveWidget::LogTable => {
                                    self.set_active_widget(ActiveWidget::InfoView);
                                },
                                ActiveWidget::SearchBox => {
                                    self.set_active_widget(ActiveWidget::LogTable);
                                },
                                ActiveWidget::InfoView => {
                                    if self.search.borrow().visible() {
                                        self.set_active_widget(ActiveWidget::SearchBox);
                                    }
                                    else {
                                        self.set_active_widget(ActiveWidget::LogTable);
                                    }
                                }
                            }
                        }
                        _ => match self.state {
                            ActiveWidget::LogTable => self.table.borrow_mut().key_press_event(key),
                            ActiveWidget::SearchBox => self.search.borrow_mut().key_press_event(key),
                            ActiveWidget::InfoView => self.text.borrow_mut().key_press_event(key),
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    fn set_active_widget(&mut self, widget: ActiveWidget) {
        match widget {
            ActiveWidget::LogTable => {
                self.table.borrow_mut().set_focus(true);
                self.search.borrow_mut().set_focus(false);
                self.text.borrow_mut().set_focus(false)
            },
            ActiveWidget::SearchBox => {
                self.table.borrow_mut().set_focus(false);
                self.search.borrow_mut().set_focus(true);
                self.text.borrow_mut().set_focus(false)
            },
            ActiveWidget::InfoView => {
                self.table.borrow_mut().set_focus(false);
                self.search.borrow_mut().set_focus(false);
                self.text.borrow_mut().set_focus(true)
            }
        }

        self.state = widget;
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {

    let rects = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            vec![
                Constraint::Min(1),
                Constraint::Length(1),
            ]
        )
        .split(f.size());

    let keys_rect = rects[1];
    let rects = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            vec![
                Constraint::Length(if app.search.borrow().visible() { 3 } else { 0 }),
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ],
        )
        .split(rects[0]);

    if rects[0].width != app.search.borrow().width() || rects[0].height != app.search.borrow().height() {
        app.search.borrow_mut().resize(rects[0].width, rects[0].height);
    }
    if rects[1].width != app.table.borrow().width() || rects[1].height != app.table.borrow().height() {
        app.table.borrow_mut().resize(rects[1].width, rects[1].height);
    }
    if rects[2].width != app.text.borrow().width() || rects[2].height != app.text.borrow().height() {
        app.text.borrow_mut().resize(rects[2].width, rects[2].height);
    }

    app.prev_size = (f.size().width, f.size().height);
    if app.search.borrow().visible() {
        f.render_widget(app.search.borrow_mut().widget(), rects[0]);
    }

    f.render_widget(app.table.borrow_mut().widget(), rects[1]);
    f.render_widget(app.text.borrow_mut().widget(), rects[2]);

    let mut common_keys = vec![
        Span::styled("Ctrl+Q", Style::default().fg(Color::White)),
        Span::raw(" "),
        Span::styled("Quit", Style::default().fg(Color::LightCyan)),

        Span::raw(" | "),

        Span::styled("Ctrl+F", Style::default().fg(Color::White)),
        Span::raw(" "),
        Span::styled("Search", Style::default().fg(Color::LightCyan)),

        Span::raw(" | "),

        Span::styled("Tab", Style::default().fg(Color::White)),
        Span::raw(" "),
        Span::styled("Next widget", Style::default().fg(Color::LightCyan)),
    ];

    match app.state {
        ActiveWidget::LogTable => {
            common_keys.extend_from_slice(&[
                Span::raw(" | "),
                Span::styled("PageUp", Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled("Go to begin", Style::default().fg(Color::LightCyan)),

                Span::raw(" | "),
                Span::styled("PageDown", Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled("Go to end", Style::default().fg(Color::LightCyan)),
            ]);
        },
        ActiveWidget::SearchBox => {
            common_keys.extend_from_slice(&[
                Span::raw(" | "),
                Span::styled("Ctrl-Bckspc", Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled("Clear", Style::default().fg(Color::LightCyan)),
            ])
        },
        ActiveWidget::InfoView => {
            common_keys.extend_from_slice(&[
                Span::raw(" | "),
                Span::styled("C", Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled("Copy", Style::default().fg(Color::LightCyan)),

                Span::raw(" | "),
                Span::styled("F", Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled("Add to filter", Style::default().fg(Color::LightCyan)),

                Span::raw(" | "),
                Span::styled("PageUp", Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled("Go to begin", Style::default().fg(Color::LightCyan)),

                Span::raw(" | "),
                Span::styled("PageDown", Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled("Go to end", Style::default().fg(Color::LightCyan)),
            ]);
        }
    };

    f.render_widget(Paragraph::new(Text::from(Spans::from(common_keys))), keys_rect)
}
