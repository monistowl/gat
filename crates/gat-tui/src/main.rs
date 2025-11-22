use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::time::Duration;
use tuirealm::{
    application::PollStrategy,
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent},
    props::{Color, Style, TextModifiers},
    ratatui::layout::{Constraint, Direction, Layout, Rect},
    ratatui::style::Stylize,
    ratatui::widgets::{Block, Borders, Paragraph},
    terminal::{CrosstermTerminalAdapter, TerminalBridge},
    Application, AttrValue, Attribute, Component, Event as TuiEvent, EventListenerCfg, Frame,
    MockComponent, NoUserEvent, Props, State,
};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Id {
    Dashboard,
    Operations,
    Datasets,
    Pipeline,
    Commands,
}

#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    SwitchPane(Id),
}

// Simple Label component
pub struct Label {
    props: Props,
}

impl Label {
    pub fn new(text: &str) -> Self {
        let mut props = Props::default();
        props.set(Attribute::Text, AttrValue::String(text.to_string()));
        Self { props }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.props
            .set(Attribute::Foreground, AttrValue::Color(color));
        self
    }

    pub fn with_bold(mut self) -> Self {
        self.props.set(
            Attribute::TextProps,
            AttrValue::TextModifiers(TextModifiers::BOLD),
        );
        self
    }
}

impl MockComponent for Label {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let text = self
            .props
            .get_or(Attribute::Text, AttrValue::String(String::default()))
            .unwrap_string();
        let foreground = self
            .props
            .get_or(Attribute::Foreground, AttrValue::Color(Color::White))
            .unwrap_color();
        let modifiers = self
            .props
            .get_or(
                Attribute::TextProps,
                AttrValue::TextModifiers(TextModifiers::empty()),
            )
            .unwrap_text_modifiers();

        let mut style = Style::default().fg(foreground);
        if modifiers.contains(TextModifiers::BOLD) {
            style = style.bold();
        }
        if modifiers.contains(TextModifiers::DIM) {
            style = style.dim();
        }

        frame.render_widget(Paragraph::new(text).style(style), area);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, _: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for Label {
    fn on(&mut self, ev: TuiEvent<NoUserEvent>) -> Option<Msg> {
        match ev {
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Esc, ..
            }) => Some(Msg::AppClose),
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Char('1'),
                ..
            }) => Some(Msg::SwitchPane(Id::Dashboard)),
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Char('2'),
                ..
            }) => Some(Msg::SwitchPane(Id::Operations)),
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Char('3'),
                ..
            }) => Some(Msg::SwitchPane(Id::Datasets)),
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Char('4'),
                ..
            }) => Some(Msg::SwitchPane(Id::Pipeline)),
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Char('5'),
                ..
            }) => Some(Msg::SwitchPane(Id::Commands)),
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Char('q'),
                ..
            }) => Some(Msg::AppClose),
            _ => None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Create tuirealm application
    let event_listener = EventListenerCfg::default()
        .crossterm_input_listener(Duration::from_millis(20), 20);
    let mut app: Application<Id, Msg, NoUserEvent> = Application::init(event_listener);

    // Mount components
    app.mount(
        Id::Dashboard,
        Box::new(
            Label::new("Dashboard - Status overview and quick actions")
                .with_color(Color::Cyan)
                .with_bold(),
        ),
        vec![],
    )?;

    app.mount(
        Id::Operations,
        Box::new(
            Label::new("Operations - Workflow management")
                .with_color(Color::Green)
                .with_bold(),
        ),
        vec![],
    )?;

    app.mount(
        Id::Datasets,
        Box::new(
            Label::new("Datasets - Data management and uploads")
                .with_color(Color::Yellow)
                .with_bold(),
        ),
        vec![],
    )?;

    app.mount(
        Id::Pipeline,
        Box::new(
            Label::new("Pipeline - Workflow definitions")
                .with_color(Color::Magenta)
                .with_bold(),
        ),
        vec![],
    )?;

    app.mount(
        Id::Commands,
        Box::new(
            Label::new("Commands - Execute CLI commands")
                .with_color(Color::Blue)
                .with_bold(),
        ),
        vec![],
    )?;

    app.active(&Id::Dashboard)?;

    // Initialize terminal bridge
    let mut terminal = TerminalBridge::init(CrosstermTerminalAdapter::new()?)?;
    let mut current_pane = Id::Dashboard;
    let mut should_quit = false;

    // Main loop
    while !should_quit {
        // Render
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Min(5),
                ])
                .split(f.area());

            // Header
            let header = Paragraph::new("GAT TUI - Press 1-5 to switch panes, ESC or Q to quit")
                .style(Style::default().fg(Color::Cyan).bold())
                .block(Block::default().borders(Borders::BOTTOM));
            f.render_widget(header, chunks[0]);

            // Menu
            let menu = Paragraph::new("[1] Dashboard  [2] Operations  [3] Datasets  [4] Pipeline  [5] Commands")
                .style(Style::default().fg(Color::White))
                .block(Block::default().borders(Borders::BOTTOM));
            f.render_widget(menu, chunks[1]);

            // Active pane content
            app.view(&current_pane, f, chunks[2]);
        })?;

        // Handle events
        match app.tick(PollStrategy::Once) {
            Ok(messages) if !messages.is_empty() => {
                for msg in messages {
                    match msg {
                        Msg::AppClose => {
                            should_quit = true;
                        }
                        Msg::SwitchPane(pane) => {
                            current_pane = pane.clone();
                            app.active(&pane)?;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Cleanup
    terminal.restore()?;
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;

    Ok(())
}
