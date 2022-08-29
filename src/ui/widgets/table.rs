use crate::ui::{index::ModelIndex, model::DataModel, widgets::WidgetExt};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{cell::RefCell, mem, rc::Rc};
use tui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

#[derive(Default)]
struct State {
    begin: usize,
    index: Option<usize>,
}

impl State {
    fn selected(&self) -> Option<usize> {
        self.index
    }

    fn select(&mut self, index: Option<usize>) {
        self.index = index;
        if index.is_none() {
            self.begin = 0;
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TableViewStyle {
    common: Style,
    selected_row_style: Style,
    header_style: Style,
    column_spacing: u16,
}

impl TableViewStyle {
    #[allow(dead_code)]
    pub fn common(mut self, style: Style) -> Self {
        self.common = style;
        self
    }

    #[allow(dead_code)]
    pub fn selected_row_style(mut self, style: Style) -> Self {
        self.selected_row_style = style;
        self
    }

    #[allow(dead_code)]
    pub fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }
}

impl Default for TableViewStyle {
    fn default() -> Self {
        TableViewStyle {
            common: Style::default(),
            selected_row_style: Style::default().bg(Color::White).fg(Color::Black),
            header_style: Style::default().bg(Color::Green).fg(Color::Black),
            column_spacing: 1,
        }
    }
}

pub struct TableView {
    state: State,
    model: Option<Rc<RefCell<dyn DataModel>>>,
    widths: Vec<Constraint>,
    style: TableViewStyle,

    visible: bool,
    focus: bool,
    width: u16,
    height: u16,

    on_selection_changed: Box<dyn FnMut(&mut Self, Option<usize>) + 'static>,
}

impl TableView {
    pub fn new(widths: Vec<Constraint>) -> Self {
        Self {
            state: State::default(),
            model: None,
            widths,
            style: TableViewStyle::default(),
            visible: true,
            focus: false,
            width: 0,
            height: 0,

            on_selection_changed: Box::new(|_, _| {}),
        }
    }

    pub fn set_model(&mut self, model: Rc<RefCell<dyn DataModel>>) {
        self.state = State::default();
        self.model = Some(model);
    }

    #[allow(dead_code)]
    pub fn style(&self) -> TableViewStyle {
        self.style
    }

    #[allow(dead_code)]
    pub fn set_style(&mut self, style: TableViewStyle) {
        self.style = style;
    }

    pub fn reset_state(&mut self) {
        self.state.select(None);
        self.state.begin = 0;
        self.update_state();
        self.emit_selection_changed();
    }

    fn update_state(&mut self) {
        let index = self.state.index.unwrap_or(0);
        let row_count = self.height.saturating_sub(4) as usize;

        if row_count == 0 {
            return;
        }

        if index > (self.state.begin + row_count) {
            self.state.begin = index - row_count;
        } else if index < self.state.begin {
            self.state.begin = index;
        }
    }

    pub fn next(&mut self) {
        if let Some(model) = self.model.clone() {
            let i = self.next_inner(self.state.selected(), model.borrow().rows());
            self.state.select(i);
            self.update_state();
            self.emit_selection_changed();
        }
    }

    fn next_inner(&mut self, current: Option<usize>, length: usize) -> Option<usize> {
        if length == 0 {
            return None;
        }

        Some(match current {
            Some(i) => {
                if i >= length - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        })
    }

    pub fn prev(&mut self) {
        if let Some(model) = self.model.clone() {
            let i = self.prev_inner(self.state.selected(), model.borrow().rows());
            self.state.select(i);
            self.update_state();
            self.emit_selection_changed();
        }
    }

    fn prev_inner(&mut self, current: Option<usize>, length: usize) -> Option<usize> {
        if length == 0 {
            return None;
        }

        Some(match current {
            Some(i) => {
                if i == 0 {
                    length - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        })
    }

    pub fn widget(&self) -> impl Widget + '_ {
        Renderer(self)
    }

    fn get_column_widths(&self, max_width: u16) -> Vec<u16> {
        let mut constraints = Vec::with_capacity(self.widths.len() * 2);
        for constraint in self.widths.iter() {
            constraints.push(*constraint);
            constraints.push(Constraint::Length(self.style.column_spacing));
        }

        if !self.widths.is_empty() {
            constraints.pop();
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

        chunks.iter().step_by(2).map(|c| c.width).collect()
    }

    pub fn on_selection_changed(
        &mut self,
        callback: impl FnMut(&mut Self, Option<usize>) + 'static,
    ) {
        self.on_selection_changed = Box::new(callback);
    }

    pub fn emit_selection_changed(&mut self) {
        let mut on_selection_changed =
            mem::replace(&mut self.on_selection_changed, Box::new(|_, _| {}));
        on_selection_changed(self, self.state.index);
        self.on_selection_changed = on_selection_changed;
    }

    fn rows(&self) -> usize {
        if let Some(model) = self.model.clone() {
            model.borrow().rows()
        } else {
            0
        }
    }
}

impl WidgetExt for TableView {
    fn set_focus(&mut self, focus: bool) {
        self.focus = focus;
    }

    fn focused(&self) -> bool {
        self.focus
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn show(&mut self) {
        self.set_visible(true)
    }

    fn hide(&mut self) {
        self.set_visible(false)
    }

    fn key_press_event(&mut self, event: KeyEvent) {
        match event {
            KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            } => self.prev(),
            KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            } => self.next(),
            KeyEvent {
                code: KeyCode::PageUp,
                modifiers: KeyModifiers::NONE,
            } => {
                self.state.begin = 0;
                self.state.index = if self.rows() > 0 { Some(0) } else { None };
                self.emit_selection_changed();
            }
            KeyEvent {
                code: KeyCode::PageDown,
                modifiers: KeyModifiers::NONE,
            } => {
                self.state.select(if self.rows() > 0 {
                    Some(self.rows() - 1)
                } else {
                    None
                });
                self.update_state();
                self.emit_selection_changed();
            }
            _ => {}
        }
    }

    fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.update_state();
    }

    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }
}

struct Renderer<'a>(&'a TableView);

impl<'a> Widget for Renderer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.area() == 0 || !self.0.visible() {
            return;
        }

        let block_style = match self.0.focused() {
            true => Style::default().fg(Color::LightYellow),
            false => Style::default(),
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(block_style)
            .title(format!(
                "{}/{}",
                self.0.state.selected().map_or(0, |i| i + 1),
                self.0
                    .model
                    .as_ref()
                    .map_or(0, |model| model.borrow().rows())
            ));

        let model = match self.0.model {
            Some(ref model) => model.borrow(),
            None => return,
        };

        let rows = model.rows();
        let cols = model.cols();

        let table_area = {
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        };

        let has_selection = self.0.state.selected().is_some();
        let rows_height = table_area.height.saturating_sub(1);
        let column_widths = self.0.get_column_widths(table_area.width);
        let mut current_height = 1;
        let (data_rows, data_columns) = (rows, cols);

        buf.set_style(
            Rect {
                x: table_area.left(),
                y: table_area.top(),
                width: table_area.width,
                height: table_area.height.min(1),
            },
            self.0.style.header_style,
        );

        let mut col = table_area.left();
        for (&width, cell) in column_widths.iter().zip(0..data_columns) {
            let header_data = model.header_data(cell).unwrap_or_default();
            buf.set_stringn(
                col,
                table_area.top(),
                header_data,
                width as usize,
                Style::default(),
            );
            col += width + 1;
        }

        // Render rows
        if data_rows == 0 {
            return;
        }

        let (start, end) = (
            self.0.state.begin,
            self.0.state.begin + rows_height as usize,
        );
        //self.0.state.offset = start;

        for index in (0..data_rows).skip(self.0.state.begin).take(end - start) {
            let (row, mut col) = (table_area.top() + current_height, table_area.left());
            current_height += 1;
            let table_row_area = Rect {
                x: col,
                y: row,
                width: table_area.width,
                height: 1,
            };

            if has_selection && self.0.state.selected().unwrap() == index {
                buf.set_style(table_row_area, self.0.style.selected_row_style)
            }

            for (&width, cell) in column_widths.iter().zip(0..data_columns) {
                let data = model
                    .data(ModelIndex::new(index, cell))
                    .map(|d| d.to_string())
                    .unwrap_or_default();

                buf.set_stringn(col, row, data, width as usize, Style::default());
                col += width + 1;
            }
        }
    }
}
