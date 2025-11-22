use gat_tui::App;

/// Test that verifies the App responds to sequential keypresses
/// This simulates an interactive session with the TUI
#[test]
fn test_interactive_keypresses_and_responses() {
    let mut app = App::new();

    // Step 1: Initial state - should show Dashboard
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 1: Initial State");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let initial_render = app.render();
    println!("Active pane: {:?}", app.active_menu_label());
    println!("Output length: {} chars, {} lines\n", initial_render.len(), initial_render.lines().count());

    assert_eq!(app.active_menu_label(), Some("Dashboard"), "Initial pane should be Dashboard");
    assert!(initial_render.contains("Dashboard"), "Dashboard content missing");
    assert!(initial_render.contains("[*1]"), "Dashboard indicator missing");

    // Step 2: Press '2' -> Operations
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 2: Press '2' (Operations)");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    app.select_menu_item('2');
    let render_2 = app.render();
    println!("Active pane: {:?}", app.active_menu_label());
    println!("Output length: {} chars, {} lines\n", render_2.len(), render_2.lines().count());

    assert_eq!(app.active_menu_label(), Some("Operations"), "Should switch to Operations");
    assert!(render_2.contains("[*2]"), "Operations indicator missing");
    assert!(render_2.contains("DERMS"), "Operations content missing");
    assert_ne!(initial_render, render_2, "Render should change when switching panes");

    // Step 3: Press '3' -> Datasets
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 3: Press '3' (Datasets)");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    app.select_menu_item('3');
    let render_3 = app.render();
    println!("Active pane: {:?}", app.active_menu_label());
    println!("Output length: {} chars, {} lines\n", render_3.len(), render_3.lines().count());

    assert_eq!(app.active_menu_label(), Some("Datasets"), "Should switch to Datasets");
    assert!(render_3.contains("[*3]"), "Datasets indicator missing");
    assert!(render_3.contains("Data catalog"), "Datasets content missing");
    assert_ne!(render_2, render_3, "Render should change when switching panes");

    // Step 4: Press '4' -> Pipeline
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 4: Press '4' (Pipeline)");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    app.select_menu_item('4');
    let render_4 = app.render();
    println!("Active pane: {:?}", app.active_menu_label());
    println!("Output length: {} chars, {} lines\n", render_4.len(), render_4.lines().count());

    assert_eq!(app.active_menu_label(), Some("Pipeline"), "Should switch to Pipeline");
    assert!(render_4.contains("[*4]"), "Pipeline indicator missing");
    assert!(render_4.contains("Pipeline composer"), "Pipeline content missing");
    assert_ne!(render_3, render_4, "Render should change when switching panes");

    // Step 5: Press '5' -> Commands
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 5: Press '5' (Commands)");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    app.select_menu_item('5');
    let render_5 = app.render();
    println!("Active pane: {:?}", app.active_menu_label());
    println!("Output length: {} chars, {} lines\n", render_5.len(), render_5.lines().count());

    assert_eq!(app.active_menu_label(), Some("Commands"), "Should switch to Commands");
    assert!(render_5.contains("[*5]"), "Commands indicator missing");
    assert!(render_5.contains("Commands workspace"), "Commands content missing");
    assert_ne!(render_4, render_5, "Render should change when switching panes");

    // Step 6: Press '1' -> Back to Dashboard
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 6: Press '1' (Back to Dashboard)");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    app.select_menu_item('1');
    let render_6 = app.render();
    println!("Active pane: {:?}", app.active_menu_label());
    println!("Output length: {} chars, {} lines\n", render_6.len(), render_6.lines().count());

    assert_eq!(app.active_menu_label(), Some("Dashboard"), "Should return to Dashboard");
    assert!(render_6.contains("[*1]"), "Dashboard indicator missing");
    assert_eq!(initial_render, render_6, "Should be identical to initial render");

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ SUCCESS: All interactive tests passed!");
    println!("║ The UI properly responds to all keypresses and updates output");
    println!("╚════════════════════════════════════════════════════════════════╝\n");
}

/// Test that verifies the actual output format is reasonable
#[test]
fn test_output_format_is_structured() {
    let app = App::new();
    let output = app.render();

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ OUTPUT FORMAT VERIFICATION");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Split into lines
    let lines: Vec<&str> = output.lines().collect();

    println!("Total lines: {}", lines.len());
    println!("Max line width: {}", lines.iter().map(|l| l.len()).max().unwrap_or(0));

    // Check for proper structure markers
    let has_title = output.contains("GAT Terminal UI");
    let has_menu = output.contains("Menu");
    let has_pane_content = output.contains("▶") || output.contains("▼");

    println!("\nStructure checks:");
    println!("  Has title: {}", has_title);
    println!("  Has menu: {}", has_menu);
    println!("  Has pane indicators: {}", has_pane_content);

    assert!(has_title, "Output should have title");
    assert!(has_menu, "Output should have menu bar");
    assert!(has_pane_content, "Output should have pane content indicators");

    // Check that it's not just one long line
    println!("\nLine length distribution:");
    let short_lines = lines.iter().filter(|l| l.len() < 40).count();
    let medium_lines = lines.iter().filter(|l| l.len() >= 40 && l.len() < 100).count();
    let long_lines = lines.iter().filter(|l| l.len() >= 100).count();

    println!("  Short (<40 chars): {}", short_lines);
    println!("  Medium (40-100 chars): {}", medium_lines);
    println!("  Long (100+ chars): {}", long_lines);

    // It should have a variety of line lengths, not be a single dump
    assert!(lines.len() > 10, "Should have multiple lines of output, not a single dump");
    assert!(short_lines > 0, "Should have some short lines (structure)");
    assert!(medium_lines > 0 || long_lines > 0, "Should have content lines");

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ Output is properly structured (not a blob)");
    println!("╚════════════════════════════════════════════════════════════════╝\n");
}

/// Test that simulates a realistic interaction sequence
#[test]
fn test_realistic_interaction_workflow() {
    let mut app = App::new();

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ REALISTIC WORKFLOW SIMULATION");
    println!("║ Dashboard → Operations → Pipeline → Commands → Dashboard");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Track all renders
    let mut renders = vec![];
    let mut labels = vec![];

    // Initial
    renders.push(app.render());
    labels.push(app.active_menu_label().unwrap_or("Unknown").to_string());

    // Navigate through workflow
    let navigation = vec![
        ('2', "Operations", "Queue"),
        ('4', "Pipeline", "composer"),
        ('5', "Commands", "workspace"),
        ('1', "Dashboard", "Dashboard"),
    ];

    for (key, expected_label, expected_content) in navigation {
        app.select_menu_item(key);
        let render = app.render();
        let label = app.active_menu_label().unwrap_or("Unknown").to_string();

        println!("Pressed '{}': {} pane", key, label);
        println!("  Expected: {} ({})", expected_label, expected_content);
        println!("  Output length: {} chars", render.len());
        println!("  Contains expected content: {}", render.contains(expected_content));

        assert_eq!(label, expected_label, "Label mismatch after pressing '{}'", key);
        assert!(render.contains(expected_content), "Content mismatch for key '{}'", key);

        renders.push(render);
        labels.push(label);
    }

    println!("\nNavigation sequence:");
    for (i, label) in labels.iter().enumerate() {
        println!("  Step {}: {}", i, label);
    }

    // Verify each render is unique (except the first and last should be Dashboard)
    assert_eq!(renders[0], renders[4], "Initial and final Dashboard renders should be identical");
    assert_ne!(renders[0], renders[1], "Dashboard and Operations should be different");
    assert_ne!(renders[1], renders[2], "Operations and Pipeline should be different");
    assert_ne!(renders[2], renders[3], "Pipeline and Commands should be different");

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ SUCCESS: Realistic workflow verified!");
    println!("║ All pane switches produce different output as expected");
    println!("╚════════════════════════════════════════════════════════════════╝\n");
}
