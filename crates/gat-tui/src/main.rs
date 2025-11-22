use anyhow::Result;
use gat_tui::App;

fn main() -> Result<()> {
    let app = App::new();
    app.run()
}
