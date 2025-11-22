use anyhow::Result;
use gat_tui::Application;

#[tokio::main]
async fn main() -> Result<()> {
    let app = Application::new();

    // Minimal hello world: print app state
    println!("GAT TUI - Starting application");
    println!("Active pane: {}", app.state().active_pane.label());

    Ok(())
}
