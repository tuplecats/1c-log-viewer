use crate::ui::widgets::WidgetExt;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{cell::RefCell, mem};
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Widget},
};

pub struct LineEdit {
    name: String,
    text: String,
    cwp: RefCell<(u16, u16, usize)>,
    style: Style,
    border_text: String,

    visible: bool,
    focus: bool,

    width: u16,
    height: u16,

    on_changed: Box<dyn FnMut(&mut Self) + 'static>,
}

impl LineEdit {
    pub fn new(name: String) -> Self {
        LineEdit {
            name,
            text: String::new(),
            cwp: RefCell::new((0, 0, 0)),
            style: Style::default(),
            border_text: String::new(),

            visible: false,
            focus: false,

            width: 0,
            height: 0,

            on_changed: Box::new(|_| {}),
        }
    }

    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.scroll_to_end();
        self.emit_on_changed();
    }

    pub fn scroll_to_start(&self) {
        let (_, width, _) = *self.cwp.borrow();
        *self.cwp.borrow_mut() = (0, width, 0);
    }

    pub fn scroll_to_end(&self) {
        let width = self.width().saturating_sub(2);
        let cursor = if self.text.len() as u16 > width {
            width
        } else {
            self.text.len() as u16
        };
        *self.cwp.borrow_mut() = (
            cursor,
            width,
            self.text.len().saturating_sub(width as usize),
        );
    }

    pub fn scroll(&self, right: bool) {
        let (mut cursor, width, mut position) = *self.cwp.borrow();
        if right {
            // go forward
            if (cursor as usize + position) < self.text.len() {
                if cursor.saturating_add(1) >= width {
                    position = position.saturating_add(1);
                } else {
                    cursor = cursor.saturating_add(1);
                }
            }
        } else {
            if position.saturating_sub(1) == position {
                cursor = cursor.saturating_sub(1);
            } else {
                position = position.saturating_sub(1);
            }
        }
        *self.cwp.borrow_mut() = (cursor, width, position);
    }

    pub fn widget(&self) -> impl Widget + '_ {
        Renderer(self)
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    #[allow(dead_code)]
    pub fn style(&self) -> Style {
        self.style
    }

    pub fn set_border_text(&mut self, text: String) {
        self.border_text = text;
    }

    // Events
    pub fn on_changed<F: FnMut(&mut Self) + 'static>(&mut self, f: F) {
        self.on_changed = Box::new(f);
    }

    pub fn emit_on_changed(&mut self) {
        let mut on_changed = mem::replace(&mut self.on_changed, Box::new(|_| {}));
        on_changed(self);
        self.on_changed = on_changed;
    }
}

impl WidgetExt for LineEdit {
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
        self.set_visible(true);
    }

    fn hide(&mut self) {
        self.set_visible(false);
    }

    fn key_press_event(&mut self, event: KeyEvent) {
        match event {
            KeyEvent {
                code: KeyCode::Char(char),
                ..
            } => {
                let (cursor, _, position) = *self.cwp.borrow();
                self.text.insert(cursor as usize + position, char);
                self.scroll(true);
                self.emit_on_changed();
            }
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
            } => {
                let (cursor, _, position) = *self.cwp.borrow();
                let index = cursor as usize + position;
                if index.saturating_sub(1) != index {
                    self.text.remove(index - 1);
                    self.scroll(false);
                    self.emit_on_changed();
                }
            }
            KeyEvent {
                code: KeyCode::Delete,
                modifiers: KeyModifiers::NONE,
            } => {
                let (cursor, _, position) = *self.cwp.borrow();
                let index = cursor as usize + position;
                if index < self.text.len() {
                    self.text.remove(index);
                    self.emit_on_changed();
                }
            }
            KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::NONE,
            } => self.scroll(true),
            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE,
            } => self.scroll(false),
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.text.clear();
                self.scroll_to_start();
                self.emit_on_changed();
            }
            _ => {}
        }
    }

    fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }
}

struct Renderer<'a>(&'a LineEdit);

impl<'a> Widget for Renderer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.area() == 0 || !self.0.visible() {
            return;
        }

        let border_text = match !self.0.border_text.is_empty() {
            true if self.0.name.is_empty() => self.0.border_text.clone(),
            true => {
                format!("{} | {}", self.0.name, self.0.border_text)
            }
            false => self.0.name.clone(),
        };

        let block_style = match self.0.focused() {
            true => Style::default().fg(Color::LightYellow),
            false => Style::default(),
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(block_style)
            .title(border_text);

        let input_area = {
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        };

        let (cursor, mut width, position) = *self.0.cwp.borrow();
        if width != input_area.width {
            width = input_area.width;
        }

        let cursor_pos = position + cursor as usize;
        let end_length = width.saturating_sub(cursor_pos as u16) as usize;

        let text = Spans::from(vec![
            Span::raw(
                self.0
                    .text
                    .chars()
                    .skip(position)
                    .take(cursor as usize)
                    .collect::<String>(),
            ),
            Span::styled(
                self.0
                    .text
                    .chars()
                    .nth(cursor_pos)
                    .map(String::from)
                    .unwrap_or(String::from(" ")),
                Style::default().add_modifier(Modifier::REVERSED),
            ),
            Span::raw(
                self.0
                    .text
                    .chars()
                    .skip(cursor_pos + 1)
                    .take(end_length)
                    .collect::<String>(),
            ),
        ]);

        buf.set_spans(input_area.x, input_area.y, &text, width);

        *self.0.cwp.borrow_mut() = (cursor, width, position);
    }
}
