#[derive(Clone)]
pub struct DemoRecord {
    pub n_firms: usize,
    pub price: f64,
    pub eens: f64,
}

/// Simple stub dataset used by the TUI to illustrate metrics.
pub struct DemoStats {
    records: Vec<DemoRecord>,
}

impl DemoStats {
    pub fn load_default() -> Self {
        let records = vec![
            DemoRecord {
                n_firms: 5,
                price: 48.3,
                eens: 0.4,
            },
            DemoRecord {
                n_firms: 7,
                price: 50.1,
                eens: 0.2,
            },
            DemoRecord {
                n_firms: 9,
                price: 52.6,
                eens: 0.3,
            },
            DemoRecord {
                n_firms: 11,
                price: 53.8,
                eens: 0.1,
            },
            DemoRecord {
                n_firms: 13,
                price: 51.4,
                eens: 0.0,
            },
        ];
        DemoStats { records }
    }

    pub fn records(&self) -> &[DemoRecord] {
        &self.records
    }

    pub fn avg_price(&self) -> f64 {
        let count = self.records.len();
        if count == 0 {
            0.0
        } else {
            self.records.iter().map(|row| row.price).sum::<f64>() / count as f64
        }
    }

    pub fn summary(&self) -> String {
        if let Some(last) = self.records.last() {
            format!(
                "{} firms | price {:.1} $/MWh, EENS {:.2}",
                last.n_firms, last.price, last.eens
            )
        } else {
            "No demo data available".to_string()
        }
    }

    pub fn gauge_metrics(&self) -> Vec<(&'static str, f64)> {
        if let Some(last) = self.records.last() {
            vec![
                ("Price", last.price),
                ("EENS", last.eens),
                ("Firms", last.n_firms as f64),
            ]
        } else {
            Vec::new()
        }
    }

    pub fn chart_points(&self) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let mut price_points = Vec::new();
        let mut eens_points = Vec::new();
        for record in &self.records {
            let x = record.n_firms as f64;
            price_points.push((x, record.price));
            eens_points.push((x, record.eens));
        }
        (price_points, eens_points)
    }
}
