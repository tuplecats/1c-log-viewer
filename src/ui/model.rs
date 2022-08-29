use crate::ui::index::ModelIndex;
use std::{any::Any, borrow::Cow};
use std::fmt::Display;
use tui::text::Text;
use crate::parser::Value;

#[derive(Default)]
pub struct Column<'a> {
    pub text: Text<'a>,
}

pub trait DataModel {
    fn rows(&self) -> usize;

    fn cols(&self) -> usize;

    fn header_index(&self, name: &str) -> Option<usize>;

    fn header_data(&self, column: usize) -> Option<Cow<'_, str>>;

    fn data(&self, index: ModelIndex) -> Option<Value>;

    fn as_any(&self) -> &dyn Any {
        &()
    }
}

impl<T: Display> DataModel for Vec<T> {
    fn rows(&self) -> usize {
        self.len()
    }

    fn cols(&self) -> usize {
        1
    }

    fn header_index(&self, _name: &str) -> Option<usize> {
        None
    }

    fn header_data(&self, _column: usize) -> Option<Cow<'_, str>> {
        None
    }

    fn data(&self, index: ModelIndex) -> Option<Value<'static>> {
        self.get(index.row()).map(|s| {
            Value::from(s.to_string())
        })
    }
}