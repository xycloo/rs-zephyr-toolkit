#![allow(missing_docs)]

use charming_fork_zephyr::{
    component::{Axis, Legend},
    element::{AreaStyle, AxisType, Color, ColorStop, Tooltip, Trigger},
    series::{Bar, Line},
    Chart,
};
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
        self.title = Some(Title {
            text: title.to_string(),
        });
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

pub struct DashboardBuilder {
    dashboard: Dashboard,
}

impl DashboardBuilder {
    pub fn new(title: &str, desc: &str) -> Self {
        let dashboard = Dashboard::new();
        let dashboard = dashboard.title(&title.to_string());
        let dashboard = dashboard.description(&desc.to_string());

        Self { dashboard }
    }

    pub fn add_table(mut self, title: &str, columns: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let mut table = Table::new().columns(columns);
        for row in rows {
            table = table.row(row);
        }
        self.dashboard = self
            .dashboard
            .entry(DashboardEntry::new().title(title).table(table));
        self
    }

    pub fn add_bar_chart(
        mut self,
        title: &str,
        hover_title: &str,
        categories: Vec<&str>,
        data: Vec<i64>,
    ) -> Self {
        let chart = Chart::new()
            .legend(Legend::new().show(true))
            .tooltip(Tooltip::new().trigger(Trigger::Axis))
            .x_axis(Axis::new().type_(AxisType::Category).data(categories))
            .y_axis(Axis::new().type_(AxisType::Value))
            .series(Bar::new().name(hover_title).data(data));

        self.dashboard = self
            .dashboard
            .entry(DashboardEntry::new().title(title).chart(chart));
        self
    }

    pub fn add_line_chart(
        mut self,
        title: &str,
        categories: Vec<String>,
        series: Vec<(&str, Vec<i64>)>,
    ) -> Self {
        let mut chart = Chart::new()
            .legend(Legend::new().show(true).left("150px").top("3%"))
            .tooltip(Tooltip::new().trigger(Trigger::Axis))
            .x_axis(Axis::new().type_(AxisType::Category).data(categories))
            .y_axis(Axis::new().type_(AxisType::Value));

        for (name, data) in series {
            chart = chart.series(Line::new().name(name).data(data).area_style(
                AreaStyle::new().color(Color::LinearGradient {
                    x: 0,
                    y: 0,
                    x2: 0,
                    y2: 1,
                    color_stops: vec![
                        ColorStop::new(0, "rgb(84, 112, 198)"),
                        ColorStop::new(1, "rgb(79, 209, 242)"),
                    ],
                }),
            ));
        }

        self.dashboard = self
            .dashboard
            .entry(DashboardEntry::new().title(title).chart(chart));
        self
    }

    pub fn build(self) -> Dashboard {
        self.dashboard
    }
}
