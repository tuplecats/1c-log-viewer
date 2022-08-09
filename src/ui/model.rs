use crate::ui::index::ModelIndex;
use std::any::Any;
use std::borrow::Cow;
use tui::text::Text;

#[derive(Default)]
pub struct Column<'a> {
    pub text: Text<'a>,
}

pub trait TableModel {
    fn rows(&self) -> usize;

    fn cols(&self) -> usize;

    fn header_index(&self, name: &str) -> Option<usize>;

    fn header_data(&self, column: usize) -> Option<Cow<'_, str>>;

    fn data(&self, index: ModelIndex) -> Option<Cow<'_, str>>;

    fn as_any(&self) -> &dyn Any {
        &()
    }
}
