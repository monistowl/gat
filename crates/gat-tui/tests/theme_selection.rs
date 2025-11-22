use gat_tui::ui::Theme;

#[test]
fn test_utf8_theme() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ UTF-8 THEME");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let theme = Theme::new();
    println!("Accent: {}", theme.accent);
    println!("Muted: {}", theme.muted);
    println!("Heavy border: {}", theme.heavy_border);
    println!("Light border: {}", theme.light_border);
    println!("Table gap: {}", theme.table_gap);
    println!("Empty icon: {}", theme.empty_icon);

    println!("\nSample output with UTF-8:");
    println!("{}─ Example", theme.heavy_border.repeat(4));
    println!("{} Item 1", theme.accent);
    println!("{} Item 2", theme.accent);
    println!("  {}{}{}", "Value", theme.table_gap, "Status");
}

#[test]
fn test_ascii_theme() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ ASCII THEME");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let theme = Theme::ascii();
    println!("Accent: {}", theme.accent);
    println!("Muted: {}", theme.muted);
    println!("Heavy border: {}", theme.heavy_border);
    println!("Light border: {}", theme.light_border);
    println!("Table gap: {}", theme.table_gap);
    println!("Empty icon: {}", theme.empty_icon);

    println!("\nSample output with ASCII:");
    println!("{}─ Example", theme.heavy_border.repeat(4));
    println!("{} Item 1", theme.accent);
    println!("{} Item 2", theme.accent);
    println!("  {}{}{}", "Value", theme.table_gap, "Status");
}

#[test]
fn test_auto_theme_selection() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ AUTO THEME SELECTION");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let lang = std::env::var("LANG").unwrap_or_else(|_| "(not set)".to_string());
    let lc_all = std::env::var("LC_ALL").unwrap_or_else(|_| "(not set)".to_string());
    let lc_ctype = std::env::var("LC_CTYPE").unwrap_or_else(|_| "(not set)".to_string());

    println!("Environment:");
    println!("  LANG: {}", lang);
    println!("  LC_ALL: {}", lc_all);
    println!("  LC_CTYPE: {}", lc_ctype);

    let theme = Theme::auto();
    println!("\nAuto-selected theme:");
    println!("  Accent: {} (3-byte UTF-8: ▍, ASCII: |)", theme.accent);

    if theme.accent == "▍" {
        println!("  → Using UTF-8 theme");
    } else {
        println!("  → Using ASCII theme");
    }

    println!("\nSample with auto theme:");
    println!("{}─ Example", theme.heavy_border.repeat(4));
    println!("{} Item 1", theme.accent);
    println!("{} Item 2", theme.accent);
}

#[test]
fn show_theme_difference() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║ THEME VISUAL COMPARISON");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("UTF-8 Theme (fancy):");
    println!("┏━━━━ Dashboard ━━━━┓");
    println!("▶ Status Cards");
    println!("  ▼ Collapsed section");
    println!("    Value │ Status");
    println!("    ───── │ ──────");
    println!("    100ms │ Good");

    println!("\nASCII Theme (plain):");
    println!("+==== Dashboard ====+");
    println!("> Status Cards");
    println!("  v Collapsed section");
    println!("    Value | Status");
    println!("    ----- | ------");
    println!("    100ms | Good");

    println!("\nIf you're seeing the PLAIN version instead of FANCY,");
    println!("it means your terminal doesn't have UTF-8 support.\n");

    println!("To enable UTF-8:");
    println!("  export LANG=en_US.UTF-8");
    println!("  export LC_ALL=en_US.UTF-8");
}
