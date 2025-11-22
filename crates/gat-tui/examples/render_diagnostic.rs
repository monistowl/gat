use gat_tui::{
    ui::{PanelRegistry, PaneContext, CommandModal},
    panes::dashboard::DashboardPane,
    panes::operations::OperationsPane,
    panes::datasets::DatasetsPane,
    panes::pipeline::PipelinePane,
    panes::commands::CommandsPane,
};

fn main() {
    let context = PaneContext::new().with_modal(CommandModal::new("Command", "Enter command", 'x'));
    let shell = PanelRegistry::new(context)
        .register(DashboardPane)
        .register(OperationsPane)
        .register(DatasetsPane)
        .register(PipelinePane)
        .register(CommandsPane)
        .into_shell("GAT TUI");

    let output = shell.render();

    println!("=== RENDER OUTPUT DIAGNOSTIC ===\n");
    println!("Output length: {} bytes", output.len());
    println!("Output char count: {} characters\n", output.chars().count());

    println!("=== RAW OUTPUT (showing whitespace) ===");
    let visible = output
        .replace('\n', "↵\n")
        .replace('\t', "→")
        .replace(' ', "·");
    println!("{}\n", visible);

    println!("=== HEX DUMP (first 500 bytes) ===");
    for (i, byte) in output.as_bytes().iter().enumerate().take(500) {
        if i % 16 == 0 {
            println!();
            print!("{:04x}: ", i);
        }
        if *byte == b'\n' {
            print!("0a ");
        } else if *byte == b'\t' {
            print!("09 ");
        } else if *byte == b' ' {
            print!("20 ");
        } else if *byte == b'\x1b' {
            print!("[ESC] ");
        } else if *byte >= 32 && *byte < 127 {
            print!("{:02x} ", byte);
        } else {
            print!("{:02x} ", byte);
        }
    }
    println!("\n");

    println!("=== ANSI ESCAPE CODE SEARCH ===");
    let has_ansi = output.contains("\x1b[");
    let escape_count = output.matches("\x1b[").count();
    println!("Contains ANSI escape sequences (\\x1b[): {}", has_ansi);
    println!("ANSI escape sequence count: {}", escape_count);

    if escape_count == 0 {
        println!("\n⚠️  WARNING: No ANSI escape codes found!");
        println!("This explains why there are no colors or text styles in the output.");
        println!("The rendering code needs to generate ANSI codes for:");
        println!("  - Colors (\\x1b[31m for red, etc.)");
        println!("  - Text styles (\\x1b[1m for bold, \\x1b[2m for dim, etc.)");
        println!("  - Backgrounds (\\x1b[41m for red background, etc.)");
    }

    println!("\n=== TERMINAL SIZE ===");
    match crossterm::terminal::size() {
        Ok((width, height)) => {
            println!("Detected terminal size: {} x {} (width x height)", width, height);
        }
        Err(e) => {
            println!("Failed to detect terminal size: {}", e);
        }
    }

    println!("\n=== FIRST 1000 CHARACTERS (readable) ===");
    let first_chars: String = output.chars().take(1000).collect();
    println!("{}", first_chars);
    if output.chars().count() > 1000 {
        println!("\n... ({} more characters)", output.chars().count() - 1000);
    }
}
