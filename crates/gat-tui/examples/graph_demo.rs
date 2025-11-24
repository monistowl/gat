/// Demo of the DAG graph visualization
///
/// Run with:
/// cargo run --example graph_demo --features fancy-ui

use gat_tui::ui::GraphView;

fn main() {
    println!("=== GAT Pipeline DAG Visualization Demo ===\n");

    // Example 1: Simple linear pipeline
    println!("Example 1: Simple Linear Pipeline");
    println!("-----------------------------------");
    let simple_graph = GraphView::new()
        .add_node("source", "Load Dataset", "◆", 0)
        .add_node("clean", "Clean Data", "▲", 1)
        .add_node("feature", "Feature Engineering", "★", 2)
        .add_node("output", "Save Results", "■", 3)
        .add_edge("source", "clean")
        .add_edge("clean", "feature")
        .add_edge("feature", "output")
        .with_legend();

    for line in simple_graph.render_lines() {
        println!("{}", line);
    }

    println!("\n");

    // Example 2: Power system workflow
    println!("Example 2: Power System Analysis Pipeline");
    println!("------------------------------------------");
    let power_graph = GraphView::new()
        .add_node("telemetry", "Live Telemetry", "◆", 0)
        .add_node("resample", "Resample (15min)", "▲", 1)
        .add_node("validate", "Validate Topology", "○", 2)
        .add_node("powerflow", "AC Power Flow", "⬡", 3)
        .add_node("contingency", "N-1 Analysis", "⬡", 4)
        .add_node("warehouse", "Data Warehouse", "■", 5)
        .add_edge("telemetry", "resample")
        .add_edge("resample", "validate")
        .add_edge("validate", "powerflow")
        .add_edge("powerflow", "contingency")
        .add_edge("contingency", "warehouse")
        .with_legend();

    for line in power_graph.render_lines() {
        println!("{}", line);
    }

    println!("\n");

    // Example 3: Compact mode
    println!("Example 3: Compact Mode (80x24 terminals)");
    println!("------------------------------------------");
    let compact_graph = GraphView::new()
        .add_node("in", "Grid Input", "◆", 0)
        .add_node("pf", "Power Flow", "⬡", 1)
        .add_node("out", "Results", "■", 2)
        .add_edge("in", "pf")
        .add_edge("pf", "out")
        .compact();

    for line in compact_graph.render_lines() {
        println!("{}", line);
    }

    println!("\n=== Demo Complete ===");
}
