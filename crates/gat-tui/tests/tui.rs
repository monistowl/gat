use std::cell::RefCell;

use gat_tui::ui::{CommandModal, ExecutionMode};
use gat_tui::{App, CommandHandle};

#[test]
fn app_renders_preview_shell() {
    let app = App::new();
    let output = app.render();
    assert!(output.contains("GAT Terminal UI"));
    assert!(output.contains("Commands"));
    assert!(output.contains("Run custom gat-cli command"));
}

#[test]
fn menu_navigation_activates_expected_items() {
    let mut app = App::new();

    assert_eq!(app.active_menu_label(), Some("Dashboard"));
    let initial_render = app.render();
    assert!(initial_render.contains("[*1] Dashboard"));

    app.select_menu_item('5');
    assert_eq!(app.active_menu_label(), Some("Commands"));
    let commands_render = app.render();
    assert!(commands_render.contains("[*5] Commands"));
    assert!(commands_render.contains("Active: Commands"));
}

#[test]
fn pane_switching_changes_layouts() {
    let mut app = App::new();

    app.select_menu_item('3');
    let datasets_view = app.render();
    assert!(datasets_view.contains("Data catalog"));
    assert!(datasets_view.contains("Public data connectors"));

    app.select_menu_item('2');
    let operations_view = app.render();
    assert!(operations_view.contains("[*2] Operations"));
    assert!(operations_view.contains("DERMS + ADMS actions"));
    assert!(operations_view.contains("Operator notes"));
}

#[test]
fn command_modal_submission_uses_custom_runner() {
    let mut modal = CommandModal::new(
        "Run custom gat-cli command",
        "Paste multi-line gat-cli snippets, switch between dry-run/full, then stream output below.",
        'r',
    )
    .with_mode(ExecutionMode::Full)
    .with_command_text(["gat-cli datasets list", "--limit 2"]);

    let invocations = RefCell::new(Vec::new());
    modal
        .submit_with_runner(|invocation| {
            invocations.borrow_mut().push(invocation.clone());
            Ok(CommandHandle::from_messages([
                "mock: starting full run",
                "mock: finished",
            ]))
        })
        .expect("modal should submit");

    modal.capture_output();

    let rendered = modal.render();
    assert!(rendered.contains("Full run [gat-cli datasets list --limit 2]"));
    assert!(rendered.contains("mock: starting full run"));
    assert!(rendered.contains("mock: finished"));

    let captured = invocations.into_inner();
    assert_eq!(captured.len(), 1);
    assert_eq!(
        captured[0],
        vec![
            "gat-cli".to_string(),
            "datasets".to_string(),
            "list".to_string(),
            "--limit".to_string(),
            "2".to_string()
        ]
    );
}
