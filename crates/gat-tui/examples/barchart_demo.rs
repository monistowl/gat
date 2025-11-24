/// Demo of the bar chart visualization
///
/// Run with:
/// cargo run --example barchart_demo --features fancy-ui

use gat_tui::ui::{BarChartView, ColorHint};

fn main() {
    println!("=== GAT Analytics Bar Chart Visualization Demo ===\n");

    // Example 1: Reliability Analysis - LOLE by Scenario
    println!("Example 1: Reliability Analysis - LOLE");
    println!("---------------------------------------");
    let reliability_chart = BarChartView::new()
        .with_title("Loss of Load Expectation by Scenario")
        .add_bar("Summer Peak", 2.5, ColorHint::Warning)
        .add_bar("Winter Peak", 0.5, ColorHint::Good)
        .add_bar("Spring Average", 0.0, ColorHint::Good)
        .add_bar("Fall Shoulder", 1.2, ColorHint::Good)
        .value_suffix(" h/year")
        .bar_width(40)
        .with_legend();

    for line in reliability_chart.render_lines() {
        println!("{}", line);
    }

    println!("\n");

    // Example 2: Deliverability Score
    println!("Example 2: Deliverability Score by Bus");
    println!("---------------------------------------");
    let ds_chart = BarChartView::new()
        .with_title("Deliverability Score (%)")
        .add_bar("Bus_001", 95.5, ColorHint::Good)
        .add_bar("Bus_002", 87.3, ColorHint::Warning)
        .add_bar("Bus_003", 78.2, ColorHint::Warning)
        .add_bar("Bus_004", 65.8, ColorHint::Critical)
        .max_value(100.0)
        .value_suffix("%")
        .bar_width(40)
        .with_legend();

    for line in ds_chart.render_lines() {
        println!("{}", line);
    }

    println!("\n");

    // Example 3: ELCC Comparison
    println!("Example 3: Effective Load Carrying Capability");
    println!("----------------------------------------------");
    let elcc_chart = BarChartView::new()
        .with_title("ELCC by Resource Type")
        .add_bar("Wind Farm A", 28.5, ColorHint::Warning)
        .add_bar("Solar Array B", 8.2, ColorHint::Warning)
        .add_bar("Battery C", 72.0, ColorHint::Good)
        .add_bar("Gas Turbine D", 95.0, ColorHint::Good)
        .max_value(100.0)
        .value_suffix(" MW")
        .bar_width(40)
        .with_legend();

    for line in elcc_chart.render_lines() {
        println!("{}", line);
    }

    println!("\n");

    // Example 4: Power Flow Utilization
    println!("Example 4: Transmission Line Utilization");
    println!("-----------------------------------------");
    let pf_chart = BarChartView::new()
        .with_title("Line Utilization (%)")
        .add_bar("Line_001", 90.0, ColorHint::Warning)
        .add_bar("Line_002", 104.0, ColorHint::Critical)
        .add_bar("Line_003", 25.0, ColorHint::Good)
        .add_bar("Line_004", 78.5, ColorHint::Good)
        .add_bar("Line_005", 112.0, ColorHint::Critical)
        .max_value(120.0)
        .value_suffix("%")
        .bar_width(40)
        .with_legend();

    for line in pf_chart.render_lines() {
        println!("{}", line);
    }

    println!("\n");

    // Example 5: Compact chart (no legend, narrower)
    println!("Example 5: Compact Chart (80x24 terminals)");
    println!("-------------------------------------------");
    let compact_chart = BarChartView::new()
        .add_bar("Scenario A", 45.0, ColorHint::Good)
        .add_bar("Scenario B", 78.0, ColorHint::Warning)
        .add_bar("Scenario C", 92.0, ColorHint::Critical)
        .max_value(100.0)
        .bar_width(25)
        .show_values(true);

    for line in compact_chart.render_lines() {
        println!("{}", line);
    }

    println!("\n=== Demo Complete ===");
    println!("\nBar Chart Symbols:");
    println!("  ▓ = Good (green)");
    println!("  ▒ = Warning (yellow)");
    println!("  ░ = Critical (red)");
    println!("  █ = Neutral (default)");
}
