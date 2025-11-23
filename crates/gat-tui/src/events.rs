use crate::models::{AppState, ExecutionMode, PaneId};

/// All possible events in the application
#[derive(Clone, Debug)]
pub enum AppEvent {
    // Input events
    KeyPress(KeyEvent),

    // Pane navigation
    SwitchPane(PaneId),

    // Command queue
    CommandQueued(String),
    CommandCompleted(String, bool),

    // Modal control
    ShowModal(ModalType),
    CloseModal,

    // Settings
    UpdateSetting(String, String),

    // Generic system events
    Tick,
    Quit,
}

#[derive(Clone, Debug, Copy)]
pub enum KeyEvent {
    // Navigation
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,

    // Input
    Tab,
    ShiftTab,
    Enter,
    Escape,
    Backspace,
    Delete,

    // Special
    CtrlA,
    CtrlC,
    CtrlM,
    CtrlHome,
    CtrlEnd,
    ShiftEnter,

    // Pane switches
    Hotkey(char),
}

#[derive(Clone, Debug)]
pub enum ModalType {
    CommandExecution,
    Settings,
    Confirmation(String),
    Info(String, String),
}

/// Reducer function: transforms state based on events
pub fn reduce(mut state: AppState, event: AppEvent) -> AppState {
    match event {
        AppEvent::KeyPress(key_event) => {
            reduce_key_event(&mut state, key_event);
        }
        AppEvent::SwitchPane(pane_id) => {
            state.active_pane = pane_id;
        }
        AppEvent::CommandQueued(cmd_id) => {
            // Log command to notifications
            state.notifications.push(crate::models::Notification {
                message: format!("Command queued: {}", cmd_id),
                kind: crate::models::NotificationKind::Info,
                timestamp: std::time::SystemTime::now(),
            });
        }
        AppEvent::CommandCompleted(cmd_id, success) => {
            let kind = if success {
                crate::models::NotificationKind::Success
            } else {
                crate::models::NotificationKind::Error
            };
            state.notifications.push(crate::models::Notification {
                message: format!(
                    "Command {}: {}",
                    cmd_id,
                    if success { "completed" } else { "failed" }
                ),
                kind,
                timestamp: std::time::SystemTime::now(),
            });
        }
        AppEvent::ShowModal(modal_type) => {
            state.modal_state = Some(match modal_type {
                ModalType::CommandExecution => {
                    crate::models::ModalState::CommandExecution(crate::models::CommandModalState {
                        command_text: String::new(),
                        execution_mode: ExecutionMode::Full,
                        output: Vec::new(),
                    })
                }
                ModalType::Settings => {
                    crate::models::ModalState::Settings(crate::models::SettingsModalState {
                        selected_field: 0,
                        theme: state.settings.theme,
                        auto_save: state.settings.auto_save_on_pane_switch,
                        confirm_delete: state.settings.confirm_on_delete,
                    })
                }
                ModalType::Confirmation(msg) => {
                    crate::models::ModalState::Confirmation(crate::models::ConfirmationState {
                        message: msg,
                        yes_label: "Yes".to_string(),
                        no_label: "No".to_string(),
                    })
                }
                ModalType::Info(title, msg) => {
                    crate::models::ModalState::Info(crate::models::InfoState {
                        title,
                        message: msg,
                        details: None,
                    })
                }
            });
        }
        AppEvent::CloseModal => {
            state.modal_state = Some(crate::models::ModalState::None);
        }
        AppEvent::UpdateSetting(key, value) => match key.as_str() {
            "auto_save" => {
                state.settings.auto_save_on_pane_switch = value == "true";
            }
            "confirm_delete" => {
                state.settings.confirm_on_delete = value == "true";
            }
            "gat_cli_path" => {
                state.settings.gat_cli_path = value;
            }
            "command_timeout" => {
                if let Ok(timeout) = value.parse() {
                    state.settings.command_timeout_secs = timeout;
                }
            }
            _ => {}
        },
        AppEvent::Tick => {
            // Handle periodic updates
        }
        AppEvent::Quit => {
            // Will be handled in main event loop
        }
    }
    state
}

fn reduce_key_event(state: &mut AppState, key: KeyEvent) {
    match key {
        KeyEvent::Hotkey(c) => {
            // Switch panes by hotkey
            let pane = match c {
                '1' => Some(PaneId::Dashboard),
                '2' => Some(PaneId::Operations),
                '3' => Some(PaneId::Datasets),
                '4' => Some(PaneId::Pipeline),
                '5' => Some(PaneId::Commands),
                'h' => Some(PaneId::Help),
                _ => None,
            };
            if let Some(p) = pane {
                state.active_pane = p;
            }
        }
        KeyEvent::Escape => {
            state.modal_state = Some(crate::models::ModalState::None);
        }
        KeyEvent::Up => {
            let pane_state = state.current_pane_state_mut();
            if pane_state.selected_row > 0 {
                pane_state.selected_row -= 1;
            }
        }
        KeyEvent::Down => {
            let pane_state = state.current_pane_state_mut();
            pane_state.selected_row += 1;
        }
        KeyEvent::PageUp => {
            let pane_state = state.current_pane_state_mut();
            if pane_state.scroll_position > 10 {
                pane_state.scroll_position -= 10;
            } else {
                pane_state.scroll_position = 0;
            }
        }
        KeyEvent::PageDown => {
            let pane_state = state.current_pane_state_mut();
            pane_state.scroll_position += 10;
        }
        KeyEvent::Home => {
            let pane_state = state.current_pane_state_mut();
            pane_state.scroll_position = 0;
            pane_state.selected_row = 0;
        }
        KeyEvent::End => {
            let pane_state = state.current_pane_state_mut();
            pane_state.selected_row = usize::MAX;
        }
        _ => {
            // Other keys handled in component-specific logic
        }
    }
}
