use charming_fork_zephyr::Chart;
use serde::Serialize;
pub use table::Table;

mod table;

#[derive(Serialize, Default)]
pub struct Dashboard {
    title: Option<Title>,
    description: Option<String>,
    data: Vec<DashboardEntry>,
}

#[derive(Serialize)]
pub enum ChartType {
    #[serde(rename = "chart")]
    Chart,
    #[serde(rename = "table")]
    Table,
}

#[derive(Serialize)]
pub enum ChartTypeWrapped {
    #[serde(rename = "inner")]
    Chart(Chart),

    #[serde(rename = "inner")]
    Table(Table),
}

#[derive(Serialize, Default)]
struct Title {
    text: String,
}

#[derive(Serialize, Default)]
pub struct DashboardEntry {
    #[serde(rename = "type")]
    chart_type: Option<ChartType>,
    title: Title,
    height: String,
    width: String,
    #[serde(flatten)]
    inner: Option<ChartTypeWrapped>,
}

impl Dashboard {
    pub fn title(mut self, title: &impl ToString) -> Self {
        self.title = Some(Title { text: title.to_string() });
        self
    }

    pub fn description(mut self, description: &impl ToString) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn entry(mut self, entry: DashboardEntry) -> Self {
        self.data.push(entry);
        self
    }

    pub fn new() -> Self {
        Self::default()
    }
}

impl DashboardEntry {
    pub fn new() -> Self {
        Self {
            chart_type: None,
            title: Title {
                text: String::new(),
            },
            height: String::from("300px"),
            width: String::from("1000px"),
            inner: None,
        }
    }

    pub fn title(mut self, title: impl ToString) -> Self {
        self.title = Title {
            text: title.to_string(),
        };
        self
    }

    pub fn chart(mut self, chart: Chart) -> Self {
        self.chart_type = Some(ChartType::Chart);
        self.inner = Some(ChartTypeWrapped::Chart(chart));

        self
    }

    pub fn table(mut self, table: Table) -> Self {
        self.chart_type = Some(ChartType::Table);
        self.inner = Some(ChartTypeWrapped::Table(table));

        self
    }
}
