use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs,
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

const SAVE_FILE: &str = "savegame.json";

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::load_or_new();
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

#[derive(Serialize, Deserialize)]
enum AppState {
    Home,
    Game,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Resources {
    firestone: u64,
    emberash: u64,
    heatcores: u64,
    sulfur_ore: u64,
    charcoal_essence: u64,
    ashen_dust: u64,
}

#[derive(Serialize, Deserialize)]
struct GameSlot {
    resources: Resources,
    last_command: String,
    unlocked_tools: Vec<Tool>,
    current_area: Area,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum Tool {
    Flamestarter,
    BlazeHammer,
    MoltenCutter,
    Pyrodrill,
    FireManipulator,
    PhoenixBeacon,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum Area {
    ScorchedPlains,
    EmberFields,
    FlameforgeRuins,
    InfernoWells,
    PyroNexus,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
enum MessageColor {
    Red,
    Green,
    Yellow,
    Blue,
    Cyan,
    White,
}

impl MessageColor {
    fn to_color(&self) -> Color {
        match self {
            MessageColor::Red => Color::Red,
            MessageColor::Green => Color::Green,
            MessageColor::Yellow => Color::Yellow,
            MessageColor::Blue => Color::Blue,
            MessageColor::Cyan => Color::Cyan,
            MessageColor::White => Color::White,
        }
    }
}

// Runtime version of Message
struct Message {
    content: String,
    color: MessageColor,
    timestamp: Instant,
}

// Storage version of Message
#[derive(Serialize, Deserialize, Clone)]
struct StoredMessage {
    content: String,
    color: MessageColor,
}

#[derive(Serialize, Deserialize)]
struct App {
    state: AppState,
    input: String,
    resources: Resources,
    last_command: String,
    commands: Vec<String>,
    game_slots: [Option<GameSlot>; 3],
    current_tool: Option<Tool>,
    current_area: Area,
    messages: Vec<StoredMessage>,
    message_index: usize,  // Track position in ring buffer
}

impl App {
    fn new() -> App {
        App {
            state: AppState::Home,
            input: String::new(),
            resources: Resources {
                firestone: 0,
                emberash: 0,
                heatcores: 0,
                sulfur_ore: 0,
                charcoal_essence: 0,
                ashen_dust: 0,
            },
            last_command: String::new(),
            commands: vec![
                "collect".to_string(),
                "mine".to_string(),
                "explore".to_string(),
                "craft".to_string(),
                "quit".to_string()
            ],
            game_slots: [None, None, None],
            current_tool: Some(Tool::Flamestarter),
            current_area: Area::ScorchedPlains,
            messages: vec![StoredMessage {
                content: "Welcome to Pyrobase. Type 'help' for commands.".to_string(),
                color: MessageColor::Yellow,
            }],
            message_index: 0,
        }
    }

    fn load_or_new() -> App {
        if let Ok(data) = fs::read_to_string(SAVE_FILE) {
            if let Ok(app) = serde_json::from_str(&data) {
                return app;
            }
        }
        App::new()
    }

    fn save(&self) {
        if let Ok(data) = serde_json::to_string(self) {
            let _ = fs::write(SAVE_FILE, data);
        }
    }

    fn get_autocomplete_suggestions(&self) -> Vec<String> {
        self.commands
            .iter()
            .filter(|cmd| cmd.starts_with(&self.input))
            .cloned()
            .collect()
    }

    fn add_message(&mut self, content: &str, color: MessageColor) {
        if self.messages.len() >= 1000 {
            // Use ring buffer behavior
            self.message_index = (self.message_index + 1) % 1000;
            if let Some(msg) = self.messages.get_mut(self.message_index) {
                msg.content = content.to_string();
                msg.color = color;
            } else {
                self.messages.push(StoredMessage {
                    content: content.to_string(),
                    color,
                });
            }
        } else {
            self.messages.push(StoredMessage {
                content: content.to_string(),
                color,
            });
            self.message_index = self.messages.len() - 1;
        }
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let tick_rate = Duration::from_millis(50); // Increased update frequency
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.state {
                    AppState::Home => match key.code {
                        KeyCode::Char('1') => {
                            app.state = AppState::Game;
                            // Load game slot 1
                            if let Some(slot) = &app.game_slots[0] {
                                app.resources = slot.resources.clone();
                                app.last_command = slot.last_command.clone();
                                app.current_tool = slot.unlocked_tools.first().cloned();
                                app.current_area = slot.current_area.clone();
                            }
                        }
                        KeyCode::Char('2') => {
                            app.state = AppState::Game;
                            // Load game slot 2
                            if let Some(slot) = &app.game_slots[1] {
                                app.resources = slot.resources.clone();
                                app.last_command = slot.last_command.clone();
                                app.current_tool = slot.unlocked_tools.first().cloned();
                                app.current_area = slot.current_area.clone();
                            }
                        }
                        KeyCode::Char('3') => {
                            app.state = AppState::Game;
                            // Load game slot 3
                            if let Some(slot) = &app.game_slots[2] {
                                app.resources = slot.resources.clone();
                                app.last_command = slot.last_command.clone();
                                app.current_tool = slot.unlocked_tools.first().cloned();
                                app.current_area = slot.current_area.clone();
                            }
                        }
                        KeyCode::Char('n') => {
                            app.state = AppState::Game;
                            // Start new game
                            app.resources = Resources {
                                firestone: 0,
                                emberash: 0,
                                heatcores: 0,
                                sulfur_ore: 0,
                                charcoal_essence: 0,
                                ashen_dust: 0,
                            };
                            app.last_command = String::new();
                            app.current_tool = Some(Tool::Flamestarter);
                            app.current_area = Area::ScorchedPlains;
                        }
                        _ => {}
                    },
                    AppState::Game => match key.code {
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Enter => {
                            app.last_command = app.input.clone();
                            match app.input.trim().to_lowercase().as_str() {
                                "q" | "quit" => {
                                    app.save();
                                    return Ok(());
                                }
                                "collect" => {
                                    app.resources.firestone += 1;
                                    app.add_message("Collected 1 firestone", MessageColor::Green);
                                }
                                "mine" => {
                                    match app.current_tool {
                                        Some(Tool::BlazeHammer) | Some(Tool::Pyrodrill) => {
                                            app.resources.emberash += 2;
                                            app.add_message("Mined 2 emberash", MessageColor::Green);
                                        }
                                        _ => {
                                            app.add_message("You need better mining tools!", MessageColor::Red);
                                        }
                                    }
                                }
                                "help" => {
                                    app.add_message("Available commands:", MessageColor::Cyan);
                                    app.add_message("collect, mine, explore, craft, quit", MessageColor::Cyan);
                                }
                                "" => {}
                                _ => {
                                    app.add_message("Unknown command. Type 'help' for commands.", MessageColor::Red);
                                }
                            }
                            app.input.clear();
                        }
                        _ => {}
                    },
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            // Update game state
            if let AppState::Game = app.state {
                app.resources.firestone += 1; // Increment resources over time
            }
            terminal.draw(|f| ui(f, &app))?; // Force redraw every tick
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut tui::Frame<B>, app: &App) {
    match app.state {
        AppState::Home => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let title = Paragraph::new("Welcome to the Game!")
                .block(Block::default().borders(Borders::ALL).title("Home"))
                .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
            let slot1 = Paragraph::new("1. Load Game Slot 1")
                .block(Block::default().borders(Borders::ALL).title("Slot 1"))
                .style(Style::default().fg(Color::Green));
            let slot2 = Paragraph::new("2. Load Game Slot 2")
                .block(Block::default().borders(Borders::ALL).title("Slot 2"))
                .style(Style::default().fg(Color::Green));
            let slot3 = Paragraph::new("3. Load Game Slot 3")
                .block(Block::default().borders(Borders::ALL).title("Slot 3"))
                .style(Style::default().fg(Color::Green));
            let new_game = Paragraph::new("n. Start New Game")
                .block(Block::default().borders(Borders::ALL).title("New Game"))
                .style(Style::default().fg(Color::Yellow));

            f.render_widget(title, chunks[0]);
            f.render_widget(slot1, chunks[1]);
            f.render_widget(slot2, chunks[2]);
            f.render_widget(slot3, chunks[3]);
            f.render_widget(new_game, chunks[4]);
        }
        AppState::Game => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(70),
                        Constraint::Percentage(30),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(30),
                        Constraint::Percentage(40),
                        Constraint::Percentage(30),
                    ]
                    .as_ref(),
                )
                .split(chunks[0]);

            let resources_text = format!(
                "Firestone: {}\nEmberash: {}\nHeatcores: {}\nSulfur Ore: {}\nCharcoal Essence: {}\nAshen Dust: {}",
                app.resources.firestone,
                app.resources.emberash,
                app.resources.heatcores,
                app.resources.sulfur_ore,
                app.resources.charcoal_essence,
                app.resources.ashen_dust
            );

            let materials = Paragraph::new(resources_text)
                .block(Block::default().borders(Borders::ALL).title("Resources"))
                .style(Style::default().fg(Color::Green));

            let current_tool = if let Some(tool) = &app.current_tool {
                format!("Current Tool: {:?}", tool)
            } else {
                "No tool equipped".to_string()
            };

            let status = format!("{}\nCurrent Area: {:?}", current_tool, app.current_area);
            let status_widget = Paragraph::new(status)
                .block(Block::default().borders(Borders::ALL).title("Status"))
                .style(Style::default().fg(Color::Yellow));

            let help_text = "Commands:\n collect - gather resources\n mine - mine rare resources\n explore - discover new areas\n craft - create tools";
            let help = Paragraph::new(help_text)
                .block(Block::default().borders(Borders::ALL).title("Help"))
                .style(Style::default().fg(Color::Cyan));

            f.render_widget(materials, top_chunks[0]);
            f.render_widget(status_widget, top_chunks[1]);
            f.render_widget(help, top_chunks[2]);

            let terminal_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(10), // Increased message area height
                        Constraint::Length(3),  // Input area
                    ]
                    .as_ref(),
                )
                .split(chunks[1]);

            let visible_messages = app.messages.iter()
                .rev()
                .take(10)
                .map(|msg| {
                    Spans::from(vec![
                        Span::styled(
                            format!("> {}", msg.content),
                            Style::default().fg(msg.color.to_color())
                        )
                    ])
                })
                .collect::<Vec<_>>();

            let messages_widget = Paragraph::new(visible_messages)
                .block(Block::default().borders(Borders::ALL).title("Messages"))
                .style(Style::default().fg(Color::White));

            let suggestions = if app.input.is_empty() {
                "".to_string()
            } else {
                let suggestions = app.get_autocomplete_suggestions().join(", ");
                format!(" [{}]", suggestions)
            };

            let cursor = Span::styled("_", Style::default().fg(Color::White).add_modifier(Modifier::SLOW_BLINK));
            let input_content = vec![
                Span::raw("> "),
                Span::raw(&app.input),
                cursor,
                Span::raw(suggestions),
            ];

            let input_widget = Paragraph::new(Spans::from(input_content))
                .block(Block::default().borders(Borders::ALL).title("Input"))
                .style(Style::default().fg(Color::White));

            f.render_widget(messages_widget, terminal_chunks[0]);
            f.render_widget(input_widget, terminal_chunks[1]);
        }
    }
}