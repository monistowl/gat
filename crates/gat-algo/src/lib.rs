pub mod alloc_kpi;
pub mod alloc_rents;
pub mod analytics_ds;
pub mod analytics_reliability;
pub mod featurize_gnn;
pub mod featurize_kpi;
pub mod io;
pub mod power_flow;
pub mod test_utils;
pub use alloc_kpi::*;
pub use alloc_rents::*;
pub use analytics_ds::*;
pub use analytics_reliability::*;
pub use featurize_gnn::*;
pub use featurize_kpi::*;
pub use io::*;
pub use power_flow::*;

pub fn run_algorithm() -> String {
    "algorithm result".to_string()
}
