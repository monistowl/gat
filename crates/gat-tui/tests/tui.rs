use std::cell::RefCell;

use gat_tui::ui::{CommandModal, ExecutionMode};
use gat_tui::{App, CommandHandle};

// ============================================================================
// TUI TEST HARNESS - Programmatic TUI Pilot
// ============================================================================
//
// This harness allows tests to:
// 1. Send keypresses/hotkeys to the TUI
// 2. Capture the rendered terminal state (text dump)
// 3. Assert on UI content and structure
// 4. Sequence complex workflows
//
// Usage:
//   let mut pilot = TuiPilot::new();
//   pilot.press('2');  // Switch to Operations pane
//   pilot.screenshot();  // Capture current render
//   pilot.assert_contains("DERMS + ADMS");  // Assert on output
//   pilot.show();  // Print captured state for manual inspection
// ============================================================================

/// Programmatic TUI pilot for testing and rapid iteration
pub struct TuiPilot {
    app: App,
    history: Vec<String>,
    current_screenshot: String,
}

impl TuiPilot {
    /// Create a new TUI pilot with default app initialization
    pub fn new() -> Self {
        let app = App::new();
        let current_screenshot = app.render();
        Self {
            app,
            history: vec![],
            current_screenshot,
        }
    }

    /// Send a hotkey input to the TUI (single character)
    pub fn press(&mut self, hotkey: char) -> &mut Self {
        self.app.select_menu_item(hotkey);
        self
    }

    /// Send a sequence of hotkeys (e.g., "123h")
    pub fn press_sequence(&mut self, sequence: &str) -> &mut Self {
        for ch in sequence.chars() {
            self.press(ch);
        }
        self
    }

    /// Capture current terminal state as text output
    pub fn screenshot(&mut self) -> &mut Self {
        let output = self.app.render();
        self.history.push(output.clone());
        self.current_screenshot = output;
        self
    }

    /// Get the current captured screenshot as string
    pub fn current(&self) -> &str {
        &self.current_screenshot
    }

    /// Get screenshot at specific history index
    pub fn at_step(&self, index: usize) -> Option<&str> {
        self.history.get(index).map(|s| s.as_str())
    }

    /// Get all captured screenshots
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Assert that current screenshot contains pattern
    pub fn assert_contains(&self, pattern: &str) -> &Self {
        assert!(
            self.current_screenshot.contains(pattern),
            "Expected pattern not found: '{}'\n\nCurrent output:\n{}",
            pattern,
            self.current_screenshot
        );
        self
    }

    /// Assert that current screenshot does NOT contain pattern
    pub fn assert_not_contains(&self, pattern: &str) -> &Self {
        assert!(
            !self.current_screenshot.contains(pattern),
            "Unexpected pattern found: '{}'\n\nCurrent output:\n{}",
            pattern,
            self.current_screenshot
        );
        self
    }

    /// Assert current active menu label
    pub fn assert_active(&self, label: &str) -> &Self {
        assert_eq!(
            self.app.active_menu_label(),
            Some(label),
            "Expected active pane: {}, got: {:?}",
            label,
            self.app.active_menu_label()
        );
        self
    }

    /// Get current active menu label
    pub fn active(&self) -> Option<&str> {
        self.app.active_menu_label()
    }

    /// Assert line count in screenshot (useful for responsive testing)
    pub fn assert_line_count(&self, expected: usize) -> &Self {
        let count = self.current_screenshot.lines().count();
        assert_eq!(
            count, expected,
            "Expected {} lines, got {}\n\nOutput:\n{}",
            expected, count, self.current_screenshot
        );
        self
    }

    /// Assert width (max line length) in screenshot
    pub fn assert_max_width(&self, max_width: usize) -> &Self {
        let actual_width = self
            .current_screenshot
            .lines()
            .map(|line| line.len())
            .max()
            .unwrap_or(0);
        assert!(
            actual_width <= max_width,
            "Expected max width {}, got {}\n\nOutput:\n{}",
            max_width,
            actual_width,
            self.current_screenshot
        );
        self
    }

    /// Print current screenshot to stdout (for manual inspection)
    pub fn show(&self) {
        println!("\n╔════════════════════════════════════════════════════════════════╗");
        println!("║ TUI SCREENSHOT (Step {})", self.history.len());
        println!("║ Active Pane: {:?}", self.app.active_menu_label());
        println!("╚════════════════════════════════════════════════════════════════╝\n");
        println!("{}", self.current_screenshot);
        println!("\n");
    }

    /// Print screenshot with line numbers (for debugging)
    pub fn show_with_lines(&self) {
        println!("\n╔════════════════════════════════════════════════════════════════╗");
        println!("║ TUI SCREENSHOT WITH LINE NUMBERS (Step {})", self.history.len());
        println!("║ Active Pane: {:?}", self.app.active_menu_label());
        println!("╚════════════════════════════════════════════════════════════════╝\n");
        for (i, line) in self.current_screenshot.lines().enumerate() {
            println!("{:3} │ {}", i + 1, line);
        }
        println!("\n");
    }

    /// Save all history to a text file for analysis
    pub fn save_history(&self, path: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;
        for (i, screenshot) in self.history.iter().enumerate() {
            writeln!(file, "╔════════════════════════════════════════════════════════════════╗")?;
            writeln!(file, "║ STEP {} - Active: {:?}", i, self.app.active_menu_label())?;
            writeln!(file, "╚════════════════════════════════════════════════════════════════╝\n")?;
            writeln!(file, "{}\n\n", screenshot)?;
        }
        Ok(())
    }

    /// Return a dump of current state for test failure messages
    pub fn dump(&self) -> String {
        format!(
            "TUI State (Step {}):\n\
             Active Pane: {:?}\n\
             Screenshot:\n\
             {}",
            self.history.len(),
            self.app.active_menu_label(),
            self.current_screenshot
        )
    }
}

impl Default for TuiPilot {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ORIGINAL TESTS (Refactored with TuiPilot)
// ============================================================================

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
    let mut pilot = TuiPilot::new();
    pilot.screenshot();

    pilot.assert_active("Dashboard").assert_contains("[*1] Dashboard");

    pilot.press('5').screenshot();
    pilot.assert_active("Commands").assert_contains("[*5] Commands");
}

#[test]
fn pane_switching_changes_layouts() {
    let mut pilot = TuiPilot::new();

    pilot.press('3').screenshot();
    pilot.assert_contains("Data catalog").assert_contains("Public data connectors");

    pilot.press('2').screenshot();
    pilot.assert_contains("DERMS + ADMS actions").assert_contains("Operator notes");
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

// ============================================================================
// NEW TESTS - Complex Workflows with TuiPilot
// ============================================================================

#[test]
fn pilot_workflow_full_navigation() {
    let mut pilot = TuiPilot::new();

    // Start on Dashboard
    pilot.screenshot().assert_active("Dashboard");

    // Navigate through all panes
    pilot.press('2').screenshot().assert_active("Operations");
    pilot.press('3').screenshot().assert_active("Datasets");
    pilot.press('4').screenshot().assert_active("Pipeline");
    pilot.press('5').screenshot().assert_active("Commands");
    pilot.press('h').screenshot().assert_active("Help > Quickstart");

    // Loop back to Dashboard
    pilot.press('1').screenshot().assert_active("Dashboard");

    // Verify history
    assert_eq!(pilot.history().len(), 7);
}

#[test]
fn pilot_sequence_input() {
    let mut pilot = TuiPilot::new();

    // Send multiple presses in sequence
    pilot.press_sequence("2345h1").screenshot();
    pilot.assert_active("Dashboard");

    // Verify we visited all panes
    assert_eq!(pilot.history().len(), 1); // Only one screenshot call
}

#[test]
fn pilot_assertion_chaining() {
    let mut pilot = TuiPilot::new();

    pilot
        .press('2')
        .screenshot()
        .assert_active("Operations")
        .assert_contains("DERMS + ADMS actions")
        .assert_not_contains("Data catalog");
}

#[test]
fn pilot_step_history_inspection() {
    let mut pilot = TuiPilot::new();

    // Capture snapshots at different steps
    pilot.screenshot();
    let step_0_contains_dashboard = pilot.at_step(0).unwrap().contains("Dashboard");
    assert!(step_0_contains_dashboard);

    pilot.press('2').screenshot();
    let step_1_contains_operations = pilot.at_step(1).unwrap().contains("Operations");
    assert!(step_1_contains_operations);

    // Verify we have both steps
    assert_eq!(pilot.history().len(), 2);
}

#[test]
fn pipeline_shows_new_transform_options() {
    let mut pilot = TuiPilot::new();

    pilot.press('4').screenshot();
    pilot.assert_active("Pipeline")
        .assert_contains("Scenario materialization")
        .assert_contains("Feature engineering")
        .assert_contains("GNN")
        .assert_contains("KPI");
}

#[test]
fn operations_shows_batch_and_allocation() {
    let mut pilot = TuiPilot::new();

    pilot.press('2').screenshot();
    pilot.assert_active("Operations")
        .assert_contains("Batch Operations")
        .assert_contains("Allocation Analysis")
        .assert_contains("Congestion rents")
        .assert_contains("KPI contribution");
}


// Uncomment to use for manual testing:
//
// #[test]
// #[ignore]
// fn pilot_manual_inspection() {
//     let mut pilot = TuiPilot::new();
//
//     pilot.screenshot().show();
//     pilot.press('2').screenshot().show();
//     pilot.press('3').screenshot().show();
//     pilot.press('4').screenshot().show();
//
//     pilot.save_history("/tmp/tui_pilot_history.txt").ok();
// }
