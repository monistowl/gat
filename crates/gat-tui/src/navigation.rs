/// Navigation and routing system for the application
///
/// Handles pane switching, tab navigation, modal management, and history
use crate::models::PaneId;
use std::collections::VecDeque;

/// Navigation history for back navigation
#[derive(Clone, Debug)]
pub struct NavigationHistory {
    stack: VecDeque<NavigationState>,
    max_depth: usize,
}

/// Complete navigation state at a point in time
#[derive(Clone, Debug)]
pub struct NavigationState {
    pub pane: PaneId,
    pub tab: usize,
    pub modal: Option<String>,
}

impl NavigationHistory {
    pub fn new() -> Self {
        NavigationHistory {
            stack: VecDeque::new(),
            max_depth: 20,
        }
    }

    /// Record current navigation state
    pub fn push(&mut self, state: NavigationState) {
        self.stack.push_front(state);
        // Limit history size
        while self.stack.len() > self.max_depth {
            self.stack.pop_back();
        }
    }

    /// Go back to previous state
    pub fn pop(&mut self) -> Option<NavigationState> {
        if self.stack.len() > 1 {
            self.stack.pop_front(); // Remove current
            self.stack.pop_front() // Return previous
        } else {
            None
        }
    }

    /// Go back to previous state (alternative implementation)
    pub fn go_back(&mut self) -> Option<NavigationState> {
        // Remove current state and return the one before it (if it exists)
        if !self.stack.is_empty() {
            self.stack.pop_front();
        }
        self.stack.front().cloned()
    }

    /// Get current state without removing
    pub fn current(&self) -> Option<NavigationState> {
        self.stack.front().cloned()
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.stack.clear();
    }
}

impl Default for NavigationHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Navigation event dispatcher
pub struct NavigationRouter;

impl NavigationRouter {
    /// Map keyboard hotkeys to pane navigation
    pub fn hotkey_to_pane(ch: char) -> Option<PaneId> {
        match ch {
            '1' => Some(PaneId::Dashboard),
            '2' => Some(PaneId::Operations),
            '3' => Some(PaneId::Datasets),
            '4' => Some(PaneId::Pipeline),
            '5' => Some(PaneId::Commands),
            'h' => Some(PaneId::Help),
            _ => None,
        }
    }

    /// Check if character is a valid pane hotkey
    pub fn is_pane_hotkey(ch: char) -> bool {
        matches!(ch, '1' | '2' | '3' | '4' | '5' | 'h')
    }

    /// Get all available panes
    pub fn all_panes() -> &'static [PaneId] {
        &[
            PaneId::Dashboard,
            PaneId::Operations,
            PaneId::Datasets,
            PaneId::Pipeline,
            PaneId::Commands,
            PaneId::Help,
        ]
    }

    /// Validate navigation state
    pub fn validate_pane(pane: PaneId) -> bool {
        Self::all_panes().contains(&pane)
    }

    /// Get previous pane in cycle
    pub fn previous_pane(current: PaneId) -> PaneId {
        let panes = Self::all_panes();
        let idx = panes.iter().position(|&p| p == current).unwrap_or(0);
        if idx == 0 {
            panes[panes.len() - 1]
        } else {
            panes[idx - 1]
        }
    }

    /// Get next pane in cycle
    pub fn next_pane(current: PaneId) -> PaneId {
        let panes = Self::all_panes();
        let idx = panes.iter().position(|&p| p == current).unwrap_or(0);
        if idx == panes.len() - 1 {
            panes[0]
        } else {
            panes[idx + 1]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_to_pane() {
        assert_eq!(
            NavigationRouter::hotkey_to_pane('1'),
            Some(PaneId::Dashboard)
        );
        assert_eq!(
            NavigationRouter::hotkey_to_pane('2'),
            Some(PaneId::Operations)
        );
        assert_eq!(NavigationRouter::hotkey_to_pane('h'), Some(PaneId::Help));
        assert_eq!(NavigationRouter::hotkey_to_pane('x'), None);
    }

    #[test]
    fn test_is_pane_hotkey() {
        assert!(NavigationRouter::is_pane_hotkey('1'));
        assert!(NavigationRouter::is_pane_hotkey('h'));
        assert!(!NavigationRouter::is_pane_hotkey('x'));
    }

    #[test]
    fn test_navigation_history() {
        let mut history = NavigationHistory::new();
        let state1 = NavigationState {
            pane: PaneId::Dashboard,
            tab: 0,
            modal: None,
        };
        let state2 = NavigationState {
            pane: PaneId::Commands,
            tab: 0,
            modal: None,
        };

        history.push(state1.clone());
        history.push(state2);

        assert_eq!(history.current().map(|s| s.pane), Some(PaneId::Commands));
        history.go_back();
        assert_eq!(history.current().map(|s| s.pane), Some(PaneId::Dashboard));
    }

    #[test]
    fn test_pane_cycling() {
        assert_eq!(
            NavigationRouter::next_pane(PaneId::Dashboard),
            PaneId::Operations
        );
        assert_eq!(
            NavigationRouter::previous_pane(PaneId::Dashboard),
            PaneId::Help
        );
        assert_eq!(NavigationRouter::next_pane(PaneId::Help), PaneId::Dashboard);
    }
}
