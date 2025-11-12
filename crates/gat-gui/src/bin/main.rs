fn main() {
    match gat_gui::launch(Some("visualization.out")) {
        Ok(summary) => println!("GUI ready: {}", summary),
        Err(err) => eprintln!("GUI failed: {err}"),
    }
}
