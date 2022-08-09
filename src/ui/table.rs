use crate::ui::index::ModelIndex;
use crate::ui::model::{Column, TableModel};
use crate::DataModelFilter;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tui::buffer::Buffer;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Modifier, Style};
use tui::text::Text;
use tui::widgets::{Block, Borders, Widget};

#[derive(Default)]
struct State {
    offset: usize,
    index: Option<usize>,
}

impl State {
    fn selected(&self) -> Option<usize> {
        self.index
    }

    fn select(&mut self, index: Option<usize>) {
        self.index = index;
        if index.is_none() {
            self.offset = 0;
        }
    }
}

pub struct TableView {
    state: State,
    model: Arc<dyn TableModel>,
    widths: Vec<Constraint>,
    highlight_style: Style,
    header_style: Style,
}

impl TableView {
    pub fn new<T: TableModel + 'static>(data: T, widths: Vec<Constraint>) -> Self {
        Self {
            state: State::default(),
            model: Arc::new(data),
            widths,
            header_style: Style::default(),
            highlight_style: Style::default().add_modifier(Modifier::REVERSED),
        }
    }

    pub fn header_style(mut self, header_style: Style) -> Self {
        self.header_style = header_style;
        self
    }

    pub fn highlight_style(mut self, highlight_style: Style) -> Self {
        self.highlight_style = highlight_style;
        self
    }

    pub fn set_filter(&mut self, filter: HashMap<String, Regex>) {
        if let Some(model) = self.model.as_any().downcast_ref::<DataModelFilter>() {
            model.set_filter(filter)
        }
    }

    pub fn next(&mut self) {
        let i = self.next_inner(self.state.selected(), self.model.rows());
        self.state.select(Some(i));
    }

    fn next_inner(&mut self, current: Option<usize>, length: usize) -> usize {
        match current {
            Some(i) => {
                if i >= length - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        }
    }

    pub fn prev(&mut self) {
        let i = self.prev_inner(self.state.selected(), self.model.rows());
        self.state.select(Some(i));
    }

    fn prev_inner(&mut self, current: Option<usize>, length: usize) -> usize {
        match current {
            Some(i) => {
                if i == 0 {
                    length - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        }
    }

    pub fn widget(&mut self) -> impl Widget + '_ {
        Renderer(self)
    }

    fn get_column_widths(&self, max_width: u16) -> Vec<u16> {
        let mut constraints = Vec::with_capacity(self.widths.len() * 2);
        for constraint in self.widths.iter() {
            constraints.push(*constraint);
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(Rect {
                x: 0,
                y: 0,
                width: max_width,
                height: 1,
            });

        chunks.iter().map(|c| c.width).collect()
    }

    fn get_row_bounds(&self, max_height: u16) -> (usize, usize) {
        let offset = self.state.offset.min(self.model.rows().saturating_sub(1));
        let mut start = offset;
        let mut end = offset;

        let mut height = 0;
        for _ in (0..self.model.rows()).skip(offset) {
            if height + 1 >= max_height {
                break;
            }

            height += 1;
            end += 1;
        }

        let selected = self
            .state
            .selected()
            .unwrap_or(0)
            .min(self.model.rows() - 1);
        while selected >= end {
            height = height.saturating_add(1);
            end += 1;
            start += 1;
        }
        while selected < start {
            start -= 1;
            end -= 1;
        }
        (start, end)
    }
}

struct Renderer<'a>(&'a mut TableView);

fn render_cell(buf: &mut Buffer, cell: &Column, area: Rect) {
    buf.set_style(area, Style::default());
    for (i, spans) in cell.text.lines.iter().enumerate() {
        if i as u16 >= area.height {
            break;
        }
        buf.set_spans(area.x, area.y + i as u16, spans, area.width);
    }
}

impl<'a> Widget for Renderer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.area() == 0 {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Table! Rows {}", self.0.model.rows()));
        let table_area = {
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        };

        let has_selection = self.0.state.selected().is_some();
        let rows_height = table_area.height.saturating_sub(1);
        let column_widths = self.0.get_column_widths(table_area.width);
        let mut current_height = 1;

        buf.set_style(
            Rect {
                x: table_area.left(),
                y: table_area.top(),
                width: table_area.width,
                height: table_area.height.min(1),
            },
            self.0.header_style,
        );

        let mut col = table_area.left();
        for (width, cell) in column_widths.iter().zip(0..self.0.model.cols()) {
            let header_data = Column {
                text: Text::from(self.0.model.header_data(cell).unwrap_or_default()),
            };
            render_cell(
                buf,
                &header_data,
                Rect {
                    x: col,
                    y: table_area.top(),
                    width: *width,
                    height: 1,
                },
            );
            col += *width;
        }

        // Render rows
        if self.0.model.rows() == 0 {
            return;
        }

        let (start, end) = self.0.get_row_bounds(rows_height);
        self.0.state.offset = start;

        let rows = self.0.model.rows();
        for index in (0..rows).skip(self.0.state.offset).take(end - start) {
            let (row, mut col) = (table_area.top() + current_height, table_area.left());
            current_height += 1;
            let table_row_area = Rect {
                x: col,
                y: row,
                width: table_area.width,
                height: 1,
            };
            buf.set_style(table_row_area, Style::default());
            for (width, cell) in column_widths.iter().zip(0..self.0.model.cols()) {
                let data = Column {
                    text: Text::from(
                        self.0
                            .model
                            .data(ModelIndex::new(index, cell))
                            .unwrap_or_default(),
                    ),
                };
                render_cell(
                    buf,
                    &data,
                    Rect {
                        x: col,
                        y: row,
                        width: *width,
                        height: 1,
                    },
                );
                col += *width
            }
            if has_selection && self.0.state.selected().unwrap() == index {
                buf.set_style(table_row_area, self.0.highlight_style)
            }
        }
    }
}
