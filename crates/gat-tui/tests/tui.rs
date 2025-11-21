use gat_tui::App;

#[test]
fn app_renders_preview_shell() {
    let app = App::new();
    let output = app.render();
    assert!(output.contains("GAT Terminal UI"));
    assert!(output.contains("Overview"));
}
