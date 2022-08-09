use std::fmt::Debug;
use std::mem;
use cli_clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use indexmap::IndexMap;
use tui::buffer::Buffer;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Widget};
use crate::parser::Value;
use crate::ui::widgets::WidgetExt;
use crate::util::sub_strings;

struct State {
    pub offset: usize,
    pub index: usize,
    pub rows_size: Vec<usize>,
}

impl Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "offset: {}, index: {}, row_size: {:?}", self.offset, self.index, self.rows_size)
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            offset: 0,
            index: 0,
            rows_size: Vec::new(),
        }
    }
}

pub struct KeyValueView {
    state: State,
    data: IndexMap<String, Value>,

    focused: bool,
    visible: bool,

    width: u16,
    height: u16,
    
    on_add_to_filter: Box<dyn FnMut((&String, &Value)) + 'static>,
}

impl KeyValueView {
    pub fn new() -> Self {
        Self {
            state: State::default(),
            data: IndexMap::new(),
            focused: false,
            visible: false,
            width: 0,
            height: 0,
            
            on_add_to_filter: Box::new(|_| {}),
        }
    }

    fn calculate_row_bounds(&mut self) {
        let offset = self.state.offset.min(self.data.len().saturating_sub(1));
        let inner_height = self.height.saturating_sub(3) as usize;
        let mut start = offset;
        let mut height = 0;

        for (index, &item) in self.state.rows_size.iter().enumerate().skip(offset) {
            height += item;
            if index == self.state.index {
                break;
            }
        }

        while height > inner_height {
            height = height.saturating_sub(self.state.rows_size[start]);
            start += 1;
        }

        self.state.offset = start.min(self.state.index);
    }

    fn next(&mut self) {
        self.state.index = self.state.index.saturating_add(1).min(self.data.len().saturating_sub(1));
        self.calculate_row_bounds();
    }

    fn prev(&mut self) {
        self.state.index = self.state.index.saturating_sub(1);
        self.calculate_row_bounds();
    }

    pub fn update_state(&mut self) {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
            .direction(Direction::Horizontal)
            .split(Rect {
                x: 1,
                y: 1,
                width: self.width.saturating_sub(1),
                height: self.height.saturating_sub(1),
            });

        for (_, v) in self.data.iter() {
            let v = v.to_string();
            let splits = sub_strings(v.as_str(), rects[1].width as usize);
            self.state.rows_size.push(splits.len().max(1));
        }
    }

    pub fn set_data(&mut self, data: IndexMap<String, Value>) {
        self.data = data;

        self.state.rows_size.clear();
        self.state.offset = 0;
        self.state.index = 0;

        self.update_state();
    }

    pub fn widget(&self) -> impl Widget + '_ {
        Renderer(&self)
    }

    pub fn on_add_to_filter(&mut self, callback: impl FnMut((&String, &Value)) + 'static) {
        self.on_add_to_filter = Box::new(callback);
    }
    
    fn emit_add_to_filter(&mut self) {
        let mut on_add_to_filter = mem::replace(&mut self.on_add_to_filter, Box::new(|_| {}));
        on_add_to_filter(self.data.get_index(self.state.index).unwrap());
        self.on_add_to_filter = on_add_to_filter;
    }
}

impl WidgetExt for KeyValueView {
    fn set_focus(&mut self, focus: bool) {
        self.focused = focus
    }

    fn focused(&self) -> bool {
        self.focused
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible
    }

    fn key_press_event(&mut self, event: KeyEvent) {
        match event {
            KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE } => {
                self.next();
            },
            KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE } => {
                self.prev();
            },
            KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::NONE } => {
                if let Ok(mut ctx) = ClipboardContext::new() {
                    if let Some((_, value)) = self.data.get_index(self.state.index) {
                        if let Ok(_) = ctx.set_contents(value.to_string()) {

                        }
                    }
                }
            },
            KeyEvent { code: KeyCode::Char('f'), modifiers: KeyModifiers::NONE } => {
                if self.data.len() > 0 {
                    self.emit_add_to_filter();
                }
            },
            KeyEvent { code: KeyCode::PageUp, modifiers: KeyModifiers::NONE } => {
                self.state.index = 0;
                self.state.offset = 0;
                self.calculate_row_bounds();
            },
            KeyEvent { code: KeyCode::PageDown, modifiers: KeyModifiers::NONE } => {
                self.state.index = self.data.len().saturating_sub(1);
                self.calculate_row_bounds();
            },
            _ => {}
        }
    }

    fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.state.rows_size.clear();
        self.update_state();
        self.calculate_row_bounds();
    }

    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }
}

struct Renderer<'a>(&'a KeyValueView);

impl<'a> Widget for Renderer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.area() == 0 {
            return
        }

        let block_style = match self.0.focused() {
            true => Style::default().fg(Color::LightYellow),
            false => Style::default()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(block_style)
            .title("Info");

        let area = {
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        };

        let rects = Layout::default()
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
            .direction(Direction::Horizontal)
            .split(area);

        // Draw header
        if area.area() == 0 {
            return
        }

        buf.set_string(rects[0].left(), rects[0].top(), "Name", Style::default());
        buf.set_string(rects[1].left(), rects[1].top(), "Value", Style::default());

        // Draw key - value pairs
        let width = rects[1].width;
        let available_height = rects[1].height;
        let mut rendered_lines = 1 as u16;
        for (i, (k, v)) in self.0.data.iter().enumerate().skip(self.0.state.offset) {
            if rendered_lines >= available_height {
                break
            }

            let style = if i == self.0.state.index {
                Style::default().fg(Color::LightMagenta)
            } else {
                Style::default()
            };

            buf.set_string(rects[0].left(), rects[1].top() + rendered_lines as u16, k, style);

            let v = v.to_string();
            let splits = sub_strings(v.as_str(), width as usize);
            splits.iter()
                .take(available_height.saturating_sub(rendered_lines) as usize)
                .enumerate()
                .for_each(|(index, s)| {
                    buf.set_string(rects[1].left(), rects[1].top() + rendered_lines + index as u16, s, style);
                });

            rendered_lines += splits.len().max(1) as u16;
        }
    }
}