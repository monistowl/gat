/// Minimal catalog shared by gat-tui to describe the public datasets GAT can fetch.
/// The OPSD snapshot is a published dataset described in the Open Power System Data
/// paper (doi:10.1016/j.energy.2017.07.039).
#[derive(Debug)]
pub struct DatasetEntry {
    pub id: &'static str,
    pub description: &'static str,
    pub tags: &'static [&'static str],
}

pub fn catalog() -> &'static [DatasetEntry] {
    &[
        // OPSD snapshot from the published work (10.1016/j.energy.2017.07.039).
        DatasetEntry {
            id: "opsd-time-series-2020",
            description: "Open Power System Data 60-minute single-index time series (Oct 2020).",
            tags: &["time-series", "open-data", "europe"],
        },
        // Small tutorial-sized CSV often used in demos.
        DatasetEntry {
            id: "airtravel",
            description: "Classic US air-travel passenger CSV (small sample).",
            tags: &["time-series", "tutorial"],
        },
    ]
}
