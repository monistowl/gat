use crate::message::{Message, ModalMessage};
/// Examples of using the modal system
///
/// This module shows patterns for using modals in message handlers.
use crate::models::AppState;

/// Example 1: Opening a confirmation dialog for a delete action
#[allow(dead_code)]
pub fn example_confirm_delete(state: &mut AppState) {
    state.show_confirmation(
        "Are you sure you want to delete this item?".to_string(),
        "Delete".to_string(),
        "Cancel".to_string(),
    );
}

/// Example 2: Opening an info dialog for errors
pub fn example_show_error(state: &mut AppState, error_msg: &str) {
    state.show_info(
        "Error".to_string(),
        error_msg.to_string(),
        Some("Please check your input and try again.".to_string()),
    );
}

/// Example 3: Opening a command execution modal
pub fn example_show_command(state: &mut AppState) {
    state.show_command_modal("gat-cli datasets list --limit 10\n--format table".to_string());
}

/// Example 4: Showing a success message
pub fn example_show_success(state: &mut AppState, operation: &str) {
    state.show_info(
        "Success".to_string(),
        format!("{} completed successfully", operation),
        None,
    );
}

/// Example 5: Using modal messages in the message enum
pub fn example_modal_message_flow() -> Message {
    Message::OpenModal(ModalMessage::ConfirmAction(
        "Proceed with operation?".to_string(),
    ))
}

/// Example 6: Closing a modal
pub fn example_close_modal(state: &mut AppState) {
    state.close_modal();
}

/// Example 7: Checking if a modal is currently open
pub fn example_check_modal_state(state: &AppState) -> bool {
    state.is_modal_open()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirm_dialog() {
        let mut state = AppState::new();
        example_confirm_delete(&mut state);
        assert!(state.is_modal_open());
    }

    #[test]
    fn test_error_dialog() {
        let mut state = AppState::new();
        example_show_error(&mut state, "Something went wrong");
        assert!(state.is_modal_open());
    }

    #[test]
    fn test_command_modal() {
        let mut state = AppState::new();
        example_show_command(&mut state);
        assert!(state.is_modal_open());
    }

    #[test]
    fn test_success_dialog() {
        let mut state = AppState::new();
        example_show_success(&mut state, "Upload");
        assert!(state.is_modal_open());
    }

    #[test]
    fn test_close_modal() {
        let mut state = AppState::new();
        example_show_error(&mut state, "Error");
        assert!(state.is_modal_open());
        example_close_modal(&mut state);
        assert!(!state.is_modal_open());
    }

    #[test]
    fn test_modal_message_flow() {
        let _msg = example_modal_message_flow();
        // Message created successfully
    }
}
