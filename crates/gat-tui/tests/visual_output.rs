use gat_tui::App;

/// Visual test that shows the actual rendered output
/// Run with: cargo test --test visual_output -- --nocapture
#[test]
fn visual_initial_render() {
    let app = App::new();
    let output = app.render();

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ INITIAL RENDER - Visual Output");
    println!("╚════════════════════════════════════════════════════════════════╝\n");
    println!("{}", output);
    println!("\n════════════════════════════════════════════════════════════════\n");

    // Print with line numbers for debugging
    println!("OUTPUT WITH LINE NUMBERS:\n");
    for (i, line) in output.lines().enumerate() {
        println!("{:3} │ {}", i + 1, line);
    }
    println!("\n════════════════════════════════════════════════════════════════\n");

    // Print metadata
    println!("METADATA:");
    println!("  Total lines: {}", output.lines().count());
    println!("  Max width: {}", output.lines().map(|l| l.len()).max().unwrap_or(0));
    println!("  Char count: {}", output.len());
    println!("\n════════════════════════════════════════════════════════════════\n");
}

/// Visual test that shows navigation between panes
#[test]
fn visual_navigation_sequence() {
    let mut app = App::new();

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ NAVIGATION SEQUENCE - Step by Step");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Step 1: Initial state
    println!("STEP 1: INITIAL STATE");
    let output1 = app.render();
    println!("{}", output1);
    println!("\n───────────────────────────────────────────────────────────────\n");

    // Step 2: Switch to Operations (hotkey '2')
    app.select_menu_item('2');
    println!("STEP 2: After pressing '2' (Operations)");
    let output2 = app.render();
    println!("{}", output2);
    println!("\n───────────────────────────────────────────────────────────────\n");

    // Step 3: Switch to Datasets (hotkey '3')
    app.select_menu_item('3');
    println!("STEP 3: After pressing '3' (Datasets)");
    let output3 = app.render();
    println!("{}", output3);
    println!("\n───────────────────────────────────────────────────────────────\n");

    // Step 4: Switch to Pipeline (hotkey '4')
    app.select_menu_item('4');
    println!("STEP 4: After pressing '4' (Pipeline)");
    let output4 = app.render();
    println!("{}", output4);
    println!("\n───────────────────────────────────────────────────────────────\n");

    // Step 5: Switch to Commands (hotkey '5')
    app.select_menu_item('5');
    println!("STEP 5: After pressing '5' (Commands)");
    let output5 = app.render();
    println!("{}", output5);
    println!("\n───────────────────────────────────────────────────────────────\n");
}

/// Visual test that analyzes the structure of the output
#[test]
fn visual_output_structure_analysis() {
    let app = App::new();
    let output = app.render();

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ OUTPUT STRUCTURE ANALYSIS");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Analyze line structure
    let lines: Vec<&str> = output.lines().collect();

    println!("LINE STRUCTURE ANALYSIS:");
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            println!("{:3} │ [EMPTY]", i + 1);
        } else if line.len() > 80 {
            println!("{:3} │ [LONG: {} chars] {}", i + 1, line.len(), &line[..80.min(line.len())]);
        } else {
            println!("{:3} │ {} chars │ {}", i + 1, line.len(), line);
        }
    }

    println!("\n\nCONTENT ANALYSIS:");

    // Check for common keywords
    let keywords = vec![
        "▶", "▼", "▍", "·", "[*", "Dashboard", "Operations",
        "Datasets", "Pipeline", "Commands", "Quickstart",
        "GAT Terminal UI"
    ];

    for keyword in keywords {
        let count = output.matches(keyword).count();
        if count > 0 {
            println!("  '{}': {} occurrences", keyword, count);
        }
    }

    println!("\n════════════════════════════════════════════════════════════════\n");
}
