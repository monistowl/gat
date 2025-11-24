/// Demo of progress bars and spinners
///
/// Run with:
/// cargo run --example progress_demo --features fancy-ui

use gat_tui::ui::{ProgressBarView, ProgressStatus, SpinnerStyle, SpinnerView};

fn main() {
    println!("=== GAT Progress Indicators Demo ===\n");

    // Example 1: Batch Operations Progress
    println!("Example 1: Batch Jobs Progress Tracking");
    println!("---------------------------------------");
    let batch_progress = ProgressBarView::new()
        .with_title("Running Jobs")
        .add_progress("Power Flow Analysis", 0.75, ProgressStatus::Active)
        .add_progress("Contingency N-1", 0.42, ProgressStatus::Active)
        .add_progress("State Estimation", 1.0, ProgressStatus::Complete)
        .add_progress("Dataset Upload", 0.0, ProgressStatus::Failed)
        .bar_width(40);

    for line in batch_progress.render_lines() {
        println!("{}", line);
    }

    println!("\n");

    // Example 2: Dataset Upload Progress
    println!("Example 2: Dataset Upload Progress");
    println!("-----------------------------------");
    let upload_progress = ProgressBarView::new()
        .with_title("Upload Queue")
        .add_progress("ieee14.raw", 1.0, ProgressStatus::Complete)
        .add_progress("ieee33.raw", 0.63, ProgressStatus::Active)
        .add_progress("scenarios.json", 0.28, ProgressStatus::Active)
        .add_progress("timeseries.csv", 0.0, ProgressStatus::Paused)
        .bar_width(40);

    for line in upload_progress.render_lines() {
        println!("{}", line);
    }

    println!("\n");

    // Example 3: Scenario Processing
    println!("Example 3: Scenario Processing Status");
    println!("-------------------------------------");
    let scenario_progress = ProgressBarView::new()
        .with_title("Scenario Batch Processing")
        .add_progress("Summer Peak 2025", 1.0, ProgressStatus::Complete)
        .add_progress("Winter Peak 2025", 0.88, ProgressStatus::Active)
        .add_progress("Spring Shoulder", 0.35, ProgressStatus::Active)
        .add_progress("Fall Average", 0.0, ProgressStatus::Active)
        .bar_width(40);

    for line in scenario_progress.render_lines() {
        println!("{}", line);
    }

    println!("\n");

    // Example 4: Compact Progress (no labels)
    println!("Example 4: Compact Progress Bars");
    println!("--------------------------------");
    let compact_progress = ProgressBarView::new()
        .add_progress("Task 1", 0.25, ProgressStatus::Active)
        .add_progress("Task 2", 0.50, ProgressStatus::Active)
        .add_progress("Task 3", 0.75, ProgressStatus::Active)
        .add_progress("Task 4", 1.0, ProgressStatus::Complete)
        .bar_width(30)
        .hide_labels();

    for line in compact_progress.render_lines() {
        println!("{}", line);
    }

    println!("\n");

    // Example 5: Spinners
    println!("Example 5: Spinner Animations");
    println!("-----------------------------");
    println!("Different spinner styles (showing frame 0):\n");

    let dots_spinner = SpinnerView::new("Loading datasets", SpinnerStyle::Dots);
    println!("Dots:   {}", dots_spinner.render());

    let line_spinner = SpinnerView::new("Running analysis", SpinnerStyle::Line);
    println!("Line:   {}", line_spinner.render());

    let arrow_spinner = SpinnerView::new("Fetching results", SpinnerStyle::Arrow);
    println!("Arrow:  {}", arrow_spinner.render());

    let box_spinner = SpinnerView::new("Processing grid", SpinnerStyle::Box);
    println!("Box:    {}", box_spinner.render());

    let circle_spinner = SpinnerView::new("Waiting for response", SpinnerStyle::Circle);
    println!("Circle: {}", circle_spinner.render());

    println!("\n");

    // Example 6: Mixed status visualization
    println!("Example 6: Multi-Job Queue with All Status Types");
    println!("------------------------------------------------");
    let mixed_progress = ProgressBarView::new()
        .with_title("Job Queue Status")
        .add_progress("Job #1234", 1.0, ProgressStatus::Complete)
        .add_progress("Job #1235", 0.65, ProgressStatus::Active)
        .add_progress("Job #1236", 0.45, ProgressStatus::Paused)
        .add_progress("Job #1237", 0.15, ProgressStatus::Failed)
        .add_progress("Job #1238", 0.0, ProgressStatus::Active)
        .bar_width(40);

    for line in mixed_progress.render_lines() {
        println!("{}", line);
    }

    println!("\n=== Demo Complete ===");
    println!("\nProgress Status Symbols:");
    println!("  ▶ = Active (running)");
    println!("  ✓ = Complete (success)");
    println!("  ✗ = Failed (error)");
    println!("  ‖ = Paused (suspended)");
    println!("\nBar Characters:");
    println!("  █ = Active fill");
    println!("  ▓ = Complete fill");
    println!("  ░ = Failed fill");
    println!("  ▒ = Paused fill");
}
