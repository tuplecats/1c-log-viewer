#[derive(Debug, Default)]
pub struct ModelIndex(usize, usize);

impl ModelIndex {
    pub fn new(row: usize, col: usize) -> Self {
        ModelIndex(row, col)
    }

    pub fn row(&self) -> usize {
        self.0
    }

    pub fn column(&self) -> usize {
        self.1
    }
}
