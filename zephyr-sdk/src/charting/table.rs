use serde::Serialize;

#[derive(Serialize, Default)]
pub struct TableStyle {
    #[serde(rename = "font-size")]
    font_size: Option<String>,
}

#[derive(Serialize, Default)]
pub struct Style {
    table: TableStyle,
}

#[derive(Serialize, Default)]
pub struct Pagination {
    summary: bool,
    limit: i32,
}

#[derive(Serialize, Default)]
pub struct Table {
    columns: Vec<String>,
    data: Vec<Vec<String>>,
    style: Style,
    pagination: Pagination,
}

impl Table {
    pub fn new() -> Self {
        let mut new = Self::default();
        new.style.table.font_size = Some("12px".into());
        new.pagination.summary = true;
        new.pagination.limit = 10;

        new
    }

    pub fn columns(mut self, cols: Vec<String>) -> Self {
        self.columns = cols;
        self
    }

    pub fn row(mut self, row: Vec<String>) -> Self {
        self.data.push(row);
        self
    }
}
