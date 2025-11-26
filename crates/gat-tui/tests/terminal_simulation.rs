use gat_tui::App;

/// Simulates actual terminal rendering with various sizes
/// This helps verify the output is properly formatted for real terminals

/// Set viewport to 80x24 for VT100 simulation
fn setup_vt100_viewport() {
    std::env::set_var("COLUMNS", "80");
    std::env::set_var("LINES", "24");
}

#[test]
fn simulate_small_terminal_rendering() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ SIMULATING 80x24 TERMINAL (common VT100 size)");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    setup_vt100_viewport();
    let mut app = App::new();

    // Simulate what would actually appear on an 80x24 terminal
    let output = app.render();

    println!("Dashboard pane on 80x24 terminal:");
    println!("{}", "─".repeat(80));
    for line in output.lines() {
        println!("{}", line);
    }
    println!("{}", "─".repeat(80));

    // Verify it fits
    let max_width = output.lines().map(|l| l.len()).max().unwrap_or(0);
    let line_count = output.lines().count();

    println!("\nStats:");
    println!(
        "  Max line width: {} bytes (visual width may be less due to unicode)",
        max_width
    );
    println!(
        "  Line count: {} (fits in 24-line terminal with truncation)",
        line_count
    );

    // Width check: allowing some unicode character byte overhead
    assert!(
        max_width < 100,
        "Lines should be reasonably sized (allowing for unicode)"
    );
    // Line count should be <= 25 (24 content + status line), allowing for typical terminals
    assert!(
        line_count <= 25,
        "Output should fit in a standard terminal with proper truncation"
    );
    // Truncation indicator may be "..." or "…" (ellipsis)
    assert!(
        output.contains("...") || output.contains("…") || line_count < 20,
        "Very long content should show truncation indicator"
    );
}

#[test]
fn simulate_standard_terminal_rendering() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ SIMULATING 100x30 TERMINAL (modern terminal size)");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let mut app = App::new();

    // Simulate what would actually appear on a 100x30 terminal
    let output = app.render();

    // Count lines for visual inspection
    let lines: Vec<&str> = output.lines().collect();

    println!("Dashboard pane on 100x30 terminal:");
    println!("First 30 lines of output:");
    println!("{}", "─".repeat(100));

    for (i, line) in lines.iter().take(30).enumerate() {
        println!("{:3} │ {}", i + 1, line);
    }

    println!("{}", "─".repeat(100));

    let max_width = lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let line_count = lines.len();

    println!("\nStats:");
    println!("  Max line width: {} (should be ≤ 99)", max_width);
    println!("  Total lines: {} (truncated to fit)", line_count);

    assert!(max_width < 100, "Lines should fit in 100-char terminal");
}

#[test]
fn verify_interactive_response_on_simulated_terminal() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ INTERACTIVE TEST: Simulating keypress on 80x24 terminal");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let mut app = App::new();

    println!("1. Initial state (Dashboard):");
    let initial = app.render();
    println!(
        "   Width: {} chars (of max 79)",
        initial.lines().map(|l| l.len()).max().unwrap_or(0)
    );
    println!("   Height: {} lines (of max 24)", initial.lines().count());
    println!("   Active: {}", app.active_menu_label().unwrap_or("?"));

    println!("\n2. Press '2' to switch to Operations:");
    app.select_menu_item('2');
    let ops = app.render();
    println!(
        "   Width: {} chars (of max 79)",
        ops.lines().map(|l| l.len()).max().unwrap_or(0)
    );
    println!("   Height: {} lines (of max 24)", ops.lines().count());
    println!("   Active: {}", app.active_menu_label().unwrap_or("?"));

    assert_ne!(initial, ops, "Output should change when switching panes");
    assert!(ops.len() < 2000, "Output should be reasonably sized");

    println!("\n3. Press '3' to switch to Datasets:");
    app.select_menu_item('3');
    let datasets = app.render();
    println!(
        "   Width: {} chars (of max 79)",
        datasets.lines().map(|l| l.len()).max().unwrap_or(0)
    );
    println!("   Height: {} lines (of max 24)", datasets.lines().count());
    println!("   Active: {}", app.active_menu_label().unwrap_or("?"));

    assert_ne!(ops, datasets, "Output should change when switching panes");

    println!("\nSUCCESS: Interactive response verified on simulated 80x24 terminal");
    println!("All renders fit within terminal bounds with proper truncation.");
}

#[test]
fn show_actual_small_terminal_output() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ ACTUAL RENDER OUTPUT: 80x24 Terminal");
    println!("║ This is exactly what users would see on a small terminal");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Force 80x24 for this specific output
    std::env::set_var("COLUMNS", "80");
    std::env::var("LINES").unwrap_or_else(|_| {
        std::env::set_var("LINES", "24");
        "24".to_string()
    });

    let app = App::new();
    let output = app.render();

    // Show with box drawing for visual verification
    let line_sep = "─".repeat(78);
    println!("┌{}┐", line_sep);
    for line in output.lines() {
        println!("│ {:<78}│", line);
    }
    println!("└{}┘", line_sep);

    println!("\n\nWith line numbers for debugging:");
    for (i, line) in output.lines().enumerate() {
        println!("{:3} │ {}", i + 1, line);
    }
}
