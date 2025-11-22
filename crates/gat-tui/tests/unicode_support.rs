use gat_tui::App;

#[test]
fn test_unicode_characters_in_output() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ UNICODE CHARACTER TEST");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let app = App::new();
    let output = app.render();

    // Check for specific Unicode characters
    let chars_to_check = vec![
        ("┏", "Top-left corner"),
        ("━", "Horizontal line"),
        ("┓", "Top-right corner"),
        ("▶", "Right triangle (collapsed)"),
        ("▼", "Down triangle (expanded)"),
        ("│", "Vertical line"),
        ("─", "Light horizontal"),
        ("▍", "Accent mark"),
        ("·", "Muted dot"),
    ];

    println!("Checking for Unicode characters:\n");
    for (char, desc) in chars_to_check {
        let found = output.contains(char);
        let byte_count = char.len();
        println!("  {} ({:2} bytes) - {}: {}",
            char,
            byte_count,
            desc,
            if found { "✓ FOUND" } else { "✗ NOT FOUND" }
        );
    }

    println!("\n");

    // The important one is that the rendering uses these characters
    // If they're missing, the output looks plain
    let has_box_chars = output.contains("┏") || output.contains("▶");

    if has_box_chars {
        println!("✓ Output contains Unicode box-drawing characters");
        println!("  The TUI SHOULD appear formatted in a proper terminal.\n");
    } else {
        println!("✗ WARNING: No Unicode characters found!");
        println!("  This might mean:");
        println!("    1. Theme doesn't use Unicode (check theme.rs)");
        println!("    2. Output is being corrupted/stripped");
        println!("    3. Different theme in use\n");
    }

    // Print a sample of the output to verify
    println!("First 500 chars of output:");
    println!("{}\n", output.chars().take(500).collect::<String>());
}

#[test]
fn check_locale_and_encoding() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ LOCALE AND ENCODING CHECK");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Check environment variables that affect character encoding
    let lang = std::env::var("LANG").unwrap_or_else(|_| "(not set)".to_string());
    let lc_all = std::env::var("LC_ALL").unwrap_or_else(|_| "(not set)".to_string());
    let lc_ctype = std::env::var("LC_CTYPE").unwrap_or_else(|_| "(not set)".to_string());

    println!("Environment variables:");
    println!("  LANG: {}", lang);
    println!("  LC_ALL: {}", lc_all);
    println!("  LC_CTYPE: {}", lc_ctype);

    println!("\nFor UTF-8 support, one of these should contain 'UTF-8' or 'utf8'");

    let has_utf8 = lang.contains("UTF") || lang.contains("utf") ||
                   lc_all.contains("UTF") || lc_all.contains("utf") ||
                   lc_ctype.contains("UTF") || lc_ctype.contains("utf");

    if has_utf8 {
        println!("✓ UTF-8 locale appears to be set\n");
    } else {
        println!("✗ WARNING: UTF-8 locale may not be set!");
        println!("  If you see plain text instead of boxes, this could be the issue.");
        println!("  Try: export LANG=en_US.UTF-8\n");
    }
}

#[test]
fn demonstrate_unicode_vs_ascii() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ UNICODE vs ASCII COMPARISON");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("UNICODE version (what gat-tui currently outputs):");
    println!("┏━━━━ GAT Terminal UI ━━━━┓");
    println!("▶ Dashboard");
    println!("  ▼ Status");
    println!("    • Overall: healthy");
    println!("    • Running: 1 workflow\n");

    println!("ASCII version (if Unicode isn't supported):");
    println!("+---- GAT Terminal UI ----+");
    println!("> Dashboard");
    println!("  v Status");
    println!("    * Overall: healthy");
    println!("    * Running: 1 workflow\n");

    println!("If you're seeing plain text, it might be the ASCII version.");
    println!("This would indicate the terminal doesn't have UTF-8 support.");
}
