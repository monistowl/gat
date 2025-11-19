/// Minimal catalog shared by gat-tui to describe the public datasets GAT can fetch.
#[derive(Debug)]
pub struct DatasetEntry {
    pub id: &'static str,
    pub description: &'static str,
    pub tags: &'static [&'static str],
}

pub fn catalog() -> &'static [DatasetEntry] {
    &[
        DatasetEntry {
            id: "opsd-time-series-2020",
            description: "Open Power System Data 60-minute single-index time series (Oct 2020).",
            tags: &["time-series", "open-data", "europe"],
        },
        DatasetEntry {
            id: "airtravel",
            description: "Classic US air-travel passenger CSV (small sample).",
            tags: &["time-series", "tutorial"],
        },
    ]
}
