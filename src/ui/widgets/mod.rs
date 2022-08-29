use crossterm::event::KeyEvent;

mod info;
mod lineedit;
mod table;

pub use info::*;
pub use lineedit::*;
pub use table::*;

pub trait WidgetExt {
    fn set_focus(&mut self, _focus: bool) {}

    fn focused(&self) -> bool {
        false
    }

    fn visible(&self) -> bool {
        true
    }

    fn set_visible(&mut self, _visible: bool) {}

    fn show(&mut self) {
        self.set_visible(true)
    }

    fn hide(&mut self) {
        self.set_visible(false)
    }

    fn key_press_event(&mut self, _event: KeyEvent) {}

    fn resize(&mut self, _width: u16, _height: u16) {}

    fn width(&self) -> u16;

    fn height(&self) -> u16;
}
