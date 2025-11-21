pub mod analytics_ds;
pub mod featurize_gnn;
pub mod io;
pub mod power_flow;
pub mod test_utils;
pub use analytics_ds::*;
pub use featurize_gnn::*;
pub use io::*;
pub use power_flow::*;

pub fn run_algorithm() -> String {
    "algorithm result".to_string()
}
