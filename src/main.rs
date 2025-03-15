use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use std::io;
use tui::backend::{Backend, CrosstermBackend};
use tui::layout::{Constraint, Direction, Layout};
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui::text::{Span, Spans};
use tui::Terminal;
use tui::widgets::{List, ListItem};
use strsim::levenshtein;
use tui::style::{Color, Style};

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let res = run_app(&mut terminal);

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

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut command_input = String::new();
    let mut output = String::new();
    let mut resources = vec![
        ("Firestone", 0, "The main resource, used for powering advanced tools and upgrading Pyrobase."),
        ("Emberash", 0, "Byproduct of gathered fire materials, used to craft basic tools."),
        ("Sulfur Ore", 0, "Required for crafting advanced tools."),
        ("Heatcores", 0, "Energy cells that power high-tier machinery."),
        ("Charcoal Essence", 0, "A rare resource for creating fire-based artifacts."),
    ];

    let commands = vec!["about", "clear", "q"];

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(60),
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(30),
                        Constraint::Percentage(70),
                    ]
                    .as_ref(),
                )
                .split(chunks[0]);

            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(70),
                        Constraint::Percentage(30),
                    ]
                    .as_ref(),
                )
                .split(top_chunks[1]);

            let map_and_events_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ]
                    .as_ref(),
                )
                .split(right_chunks[0]);

            let resource_items: Vec<ListItem> = resources.iter()
                .map(|(name, qty, _)| ListItem::new(format!("{}: {}", name, qty)))
                .collect();
            let resource_list = List::new(resource_items)
                .block(Block::default().title("Resources").borders(Borders::ALL));

            f.render_widget(resource_list, top_chunks[0]);

            let event_block = Block::default().title("Events").borders(Borders::ALL);
            f.render_widget(event_block, map_and_events_chunks[0]);

            let right_block = Block::default().title("Map").borders(Borders::ALL);
            f.render_widget(right_block, map_and_events_chunks[1]);

            let tools_block = Block::default().title("Tools").borders(Borders::ALL);
            f.render_widget(tools_block, right_chunks[1]);

            let command_block = Block::default().title("Commands").borders(Borders::ALL);
            let command_paragraph = Paragraph::new(command_input.as_ref())
                .block(command_block)
                .wrap(Wrap { trim: true });
            f.render_widget(command_paragraph, chunks[1]);

            let output_block = Block::default().title("Output").borders(Borders::ALL);
            let output_paragraph = Paragraph::new(output.as_ref())
                .block(output_block)
                .wrap(Wrap { trim: true });
            f.render_widget(output_paragraph, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char(c) => {
                    command_input.push(c);
                }
                KeyCode::Backspace => {
                    command_input.pop();
                }
                KeyCode::Enter => {
                    let input = command_input.trim();
                    if input == "q" {
                        return Ok(());
                    } else if input.starts_with("about ") {
                        let parts: Vec<&str> = input.split_whitespace().collect();
                        if parts.len() == 2 {
                            let query = parts[1];
                            let about_info = if let Ok(id) = query.parse::<usize>() {
                                resources.get(id - 1).map(|(name, _, desc)| format!("{}: {}", name, desc))
                            } else {
                                resources.iter().find(|(name, _, _)| name.eq_ignore_ascii_case(query)).map(|(name, _, desc)| format!("{}: {}", name, desc))
                            };
                            if let Some(info) = about_info {
                                output = info;
                            } else {
                                output = "Resource not found.".to_string();
                            }
                        }
                    } else if input == "clear" {
                        output.clear();
                    } else {
                        let mut suggestion = None;
                        for command in &commands {
                            if levenshtein(input, command) <= 2 {
                                suggestion = Some(command);
                                break;
                            }
                        }
                        if let Some(suggested_command) = suggestion {
                            output = format!("Unknown command: '{}'. Did you mean '{}'?", input, suggested_command);
                            output = format!("\x1b[31m{}\x1b[0m", output); // Make text red
                        } else {
                            output = format!("Unknown command: '{}'.", input);
                            output = format!("\x1b[31m{}\x1b[0m", output); // Make text red
                        }
                    }
                    command_input.clear();
                }
                KeyCode::Esc => {
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}
