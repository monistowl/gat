use anyhow::Result;
use atty::Stream;
use gat_tui::App;

fn main() -> Result<()> {
    if !atty::is(Stream::Stdout) {
        eprintln!("gat-tui requires an interactive terminal (stdout is not a TTY).");
        eprintln!("Run `gat-gui` if you only need a rendered summary without a terminal.");
        return Ok(());
    }

    let app = App::new();
    app.run()
}
