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

fn show_lore() -> io::Result<()> {
    let lore = [
        "In the distant future, the Earth has been ravaged by uncontrollable wildfires that have wiped out most of humanity's population and infrastructure.",
        "Amidst the chaos, a lone scientist named Dr. Aurelia Pyros, known for her pioneering work in fire-based technologies, survives.",
        "She discovers an ancient, long-buried facility known as Pyrobase, a research station once operated by an advanced civilization that mastered the art of harnessing the destructive power of fire.",
        "",
        "The facility holds the key to humanity’s survival—if Dr. Pyros can unlock its secrets.",
        "However, the base is scattered across a fractured landscape, and it will take significant resources, strategy, and time to reconstruct Pyrobase and rebuild civilization.",
        "Players must help Dr. Pyros gather resources, develop tools, and explore different sections of the Pyrobase, each guarded by environmental hazards, ancient technology, and ever-growing wildfires.",
    ];

    // Clear the terminal
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

    // Display lore slowly, one line at a time
    for line in &lore {
        println!("{}", line);
        std::thread::sleep(Duration::from_millis(500)); // Adjust the delay as needed
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Show lore at the start of the game
    show_lore()?;

    // Wait for user input to proceed
    println!("Press Enter to continue...");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;


    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new(); // Directly create a new app

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
    Game, // Remove Home state
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
    last_command: String,
    commands: Vec<String>,
    messages: Vec<StoredMessage>,
    message_index: usize,  // Track position in ring buffer
}

impl App {
    fn new() -> App {
        App {
            state: AppState::Game, // Directly start in Game state
            input: String::new(),
            last_command: String::new(),
            commands: vec![
                "quit".to_string()
            ],
            messages: vec![
                StoredMessage {
                    content: "Welcome to Pyrobase. Type 'help' for commands.".to_string(),
                    color: MessageColor::Yellow,
                },
                StoredMessage {
                    content: "Type 'quit' to exit the game.".to_string(),
                    color: MessageColor::Cyan,
                },
            ],
            message_index: 0,
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

    fn show_help(&mut self) {
        self.add_message("Available commands:", MessageColor::Cyan);
        self.add_message("quit - exit the game", MessageColor::Cyan);
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
                                    return Ok(());
                                }
                                "help" => {
                                    app.show_help();
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
            terminal.draw(|f| ui(f, &app))?; // Force redraw every tick
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut tui::Frame<B>, app: &App) {
    match app.state {
        AppState::Game => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(80), // Adjusted to close the gap
                        Constraint::Percentage(20), // Adjusted to close the gap
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let status_and_messages = vec![
                Spans::from(Span::styled(
                    "Welcome to Pyrobase. Type 'help' for commands.",
                    Style::default().fg(Color::Yellow),
                )),
                Spans::from(""),
            ];

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

            let combined_content = status_and_messages.into_iter().chain(visible_messages).collect::<Vec<_>>();

            let status_and_messages_widget = Paragraph::new(combined_content)
                .block(Block::default().borders(Borders::ALL).title("Status and Messages"))
                .style(Style::default().fg(Color::White));

            f.render_widget(status_and_messages_widget, chunks[0]);

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

            f.render_widget(input_widget, chunks[1]);
        }
    }
}