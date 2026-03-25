use std::env;
use std::io;
use std::panic;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal,
};

mod app;
mod browser;
mod clipboard;
mod config;
mod launcher;
mod pattern_manager;
mod profile;
mod url_pattern;

use crate::app::{App, Focus};
use crate::pattern_manager::PatternManager;
use crate::url_pattern::find_matching_pattern;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get URL from command line arguments
    let args: Vec<String> = env::args().collect();
    let initial_url = if args.len() > 1 {
        Some(args[1].clone())
    } else {
        None
    };

    // If URL is provided, check for matching patterns for auto-launch
    if let Some(ref url) = initial_url {
        let config = config::Config::load();

        // Check if any pattern matches the URL
        if let Some(pattern_match) = find_matching_pattern(url, &config.url_patterns) {
            // Try to find the matching browser
            let browsers = browser::discover_browsers();

            if let Some(browser) = browsers
                .iter()
                .find(|b| b.name == pattern_match.browser_name)
            {
                // Get profiles for this browser
                let binary_name = browser.exec.split_whitespace().next().unwrap_or("");
                let profiles = if profile::is_firefox_based(binary_name) {
                    profile::detect_firefox_profiles(binary_name)
                } else if profile::is_chromium_based(binary_name) {
                    profile::detect_chromium_profiles(binary_name)
                } else {
                    profile::detect_unknown_profiles()
                };

                // Find the matching profile
                if let Some(profile_name) = pattern_match.profile_name {
                    if let Some(profile) = profiles.iter().find(|p| p.name == profile_name) {
                        // Get containers if Firefox
                        let container = if pattern_match.container_name.is_some()
                            && profile::is_firefox_based(binary_name)
                        {
                            let containers = profile::detect_firefox_containers(&profile.path);
                            pattern_match
                                .container_name
                                .and_then(|name| containers.into_iter().find(|c| c.name == name))
                        } else {
                            None
                        };

                        // Launch directly without TUI
                        launcher::launch(
                            browser,
                            profile,
                            container.as_ref(),
                            url,
                            pattern_match.incognito,
                            pattern_match.new_window,
                        )?;
                        return Ok(());
                    }
                } else if let Some(profile) = profiles.first() {
                    // No profile specified, use first (default)
                    let container = if pattern_match.container_name.is_some()
                        && profile::is_firefox_based(binary_name)
                    {
                        let containers = profile::detect_firefox_containers(&profile.path);
                        pattern_match
                            .container_name
                            .and_then(|name| containers.into_iter().find(|c| c.name == name))
                    } else {
                        None
                    };

                    // Launch directly without TUI
                    launcher::launch(
                        browser,
                        profile,
                        container.as_ref(),
                        url,
                        pattern_match.incognito,
                        pattern_match.new_window,
                    )?;
                    return Ok(());
                }
            }
        }
    }

    // No pattern match or no URL - proceed with normal TUI flow
    // Setup terminal
    enable_raw_mode()?;
    // Set up panic handler to restore terminal on crash
    setup_panic_handler();

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Enter alternate screen (this saves the current terminal content)
    // When we exit, leaving alternate screen restores the previous content
    execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen).ok();

    // Create app and run it
    let mut app = App::new(initial_url);
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal - this will restore the previous terminal content
    restore_terminal();

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

/// Restores the terminal to its previous state
fn restore_terminal() {
    // Leave alternate screen (this restores the previous terminal content)
    let _ = execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

/// Sets up panic handler to restore terminal on crash
fn setup_panic_handler() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Check if exit was requested
        if app.exit_requested {
            break;
        }

        if let Event::Key(key) = event::read()? {
            // Handle Ctrl+C to gracefully exit
            if key.code == crossterm::event::KeyCode::Char('c')
                && key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                break;
            }
            app.handle_key_event(key);
        }
    }
    Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    // Create chunks for layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3), // URL
                Constraint::Length(3), // Browser
                Constraint::Length(3), // Profile
                Constraint::Length(3), // Container (conditionally shown)
                Constraint::Length(3), // Toggles
                Constraint::Length(3), // Buttons
                Constraint::Length(1), // Shortcuts
            ]
            .as_ref(),
        )
        .split(f.size());

    // Helper styles for clear visual feedback
    let selected_border_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let normal_border_style = Style::default().fg(Color::DarkGray);
    let selected_content_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let normal_content_style = Style::default().fg(Color::Gray);

    // URL input
    let url_focused = app.focus == Focus::Url;
    let url_content = if url_focused {
        format!("> {} _", app.url)
    } else {
        app.url.clone()
    };
    let url_input = Paragraph::new(url_content)
        .style(if url_focused {
            selected_content_style
        } else {
            normal_content_style
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if url_focused {
                    selected_border_style
                } else {
                    normal_border_style
                })
                .title(if url_focused {
                    "[ TYPE to enter URL ]"
                } else {
                    "URL"
                })
                .title_style(if url_focused {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    normal_border_style
                }),
        );
    f.render_widget(url_input, chunks[0]);

    // Browser dropdown
    let browser_focused = app.focus == Focus::Browser;
    let browser_items: Vec<ListItem> = app
        .browsers
        .iter()
        .map(|b| {
            let content = if app.browsers.is_empty() {
                "No browsers found".to_string()
            } else {
                b.name.clone()
            };
            ListItem::new(content)
        })
        .collect();
    let browser_list = List::new(browser_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if browser_focused {
                    selected_border_style
                } else {
                    normal_border_style
                })
                .title(if browser_focused {
                    "[ ENTER to select ]"
                } else {
                    "Browser"
                })
                .title_style(if browser_focused {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    normal_border_style
                }),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(
        browser_list,
        chunks[1],
        &mut ListState::default().with_selected(Some(app.selected_browser)),
    );

    // Profile dropdown
    let profile_focused = app.focus == Focus::Profile;
    let profile_items: Vec<ListItem> = app
        .profiles
        .iter()
        .map(|p| ListItem::new(p.name.clone()))
        .collect();
    let profile_list = List::new(profile_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if profile_focused {
                    selected_border_style
                } else {
                    normal_border_style
                })
                .title(if profile_focused {
                    "[ ENTER to select ]"
                } else {
                    "Profile"
                })
                .title_style(if profile_focused {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    normal_border_style
                }),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(
        profile_list,
        chunks[2],
        &mut ListState::default().with_selected(Some(app.selected_profile)),
    );

    // Container dropdown (only show if applicable)
    if app.is_container_row_visible() {
        let container_focused = app.focus == Focus::Container;
        let container_items: Vec<ListItem> = app
            .containers
            .iter()
            .map(|c| ListItem::new(c.name.clone()))
            .collect();
        let container_list = List::new(container_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(if container_focused {
                        selected_border_style
                    } else {
                        normal_border_style
                    })
                    .title(if container_focused {
                        "[ ENTER to select ]"
                    } else {
                        "Container"
                    })
                    .title_style(if container_focused {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        normal_border_style
                    }),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");
        f.render_stateful_widget(
            container_list,
            chunks[3],
            &mut ListState::default().with_selected(app.selected_container),
        );
    }

    // Toggles with key hints
    let incognito_focused = app.focus == Focus::IncognitoToggle;
    let new_window_focused = app.focus == Focus::NewWindowToggle;

    let incognito_text = if app.incognito { "[X]" } else { "[ ]" };
    let incognito_key = if incognito_focused { " INC" } else { " [i]" };
    let _incognito_color = if app.incognito {
        Color::Green
    } else {
        Color::Gray
    };

    let new_window_text = if app.new_window { "[X]" } else { "[ ]" };
    let new_window_key = if new_window_focused { " WIN" } else { " [w]" };
    let _new_window_color = if app.new_window {
        Color::Green
    } else {
        Color::Gray
    };

    let toggles = Paragraph::new(format!(
        "{}{} Incognito/Private     {}{} New Window",
        incognito_text, incognito_key, new_window_text, new_window_key
    ))
    .style(normal_content_style)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(if incognito_focused || new_window_focused {
                selected_border_style
            } else {
                normal_border_style
            })
            .title("Options")
            .title_style(if incognito_focused || new_window_focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                normal_border_style
            }),
    );
    f.render_widget(toggles, chunks[4]);

    // Buttons with key hints
    let copy_focused = app.focus == Focus::CopyButton;
    let open_focused = app.focus == Focus::OpenButton;
    let quit_focused = app.focus == Focus::QuitButton;

    let open_text = if open_focused {
        "[ ENTER ] Open"
    } else {
        "[o] Open"
    };
    let _open_color = if open_focused {
        Color::Cyan
    } else {
        Color::Green
    };

    let copy_text = if copy_focused {
        "[ ENTER ] Copy"
    } else {
        "[c] Copy"
    };
    let _copy_color = if copy_focused {
        Color::Cyan
    } else {
        Color::Blue
    };

    let quit_text = if quit_focused {
        "[ ESC ] Quit"
    } else {
        "[q] Quit"
    };
    let _quit_color = if quit_focused {
        Color::Red
    } else {
        Color::DarkGray
    };

    let buttons = Paragraph::new(format!(
        "{:^15}   {:^15}   {:^15}",
        open_text, copy_text, quit_text
    ))
    .style(Style::default().fg(Color::White))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(if open_focused || copy_focused || quit_focused {
                selected_border_style
            } else {
                normal_border_style
            })
            .title("Actions")
            .title_style(if open_focused || copy_focused || quit_focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                normal_border_style
            }),
    );
    f.render_widget(buttons, chunks[5]);

    // Shortcuts help
    let shortcuts = Paragraph::new(
        "TAB/Arrows: Navigate  |  ENTER: Select  |  ESC: Cancel  |  c: Copy  o: Open  i: Incognito  w: Window  q: Quit",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(shortcuts, chunks[6]);

    // Error message (if any)
    if let Some(error) = &app.error {
        let error_block = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Error")
                    .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            );
        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area);
        f.render_widget(error_block, area);
    }

    // Info message (if any)
    if let Some(info) = &app.info {
        let info_block = Paragraph::new(info.as_str())
            .style(Style::default().fg(Color::Blue))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Info")
                    .title_style(Style::default().fg(Color::Blue)),
            );
        let area = centered_rect(60, 10, f.size());
        f.render_widget(Clear, area);
        f.render_widget(info_block, area);
    }

    // Dropdown overlays
    if let Some(open_focus) = app.dropdown_open {
        let (items, selected, title) = match open_focus {
            Focus::Browser => (
                app.browsers
                    .iter()
                    .map(|b| b.name.as_str())
                    .collect::<Vec<&str>>(),
                app.selected_browser,
                "Browser",
            ),
            Focus::Profile => (
                app.profiles
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<&str>>(),
                app.selected_profile,
                "Profile",
            ),
            Focus::Container => (
                app.containers
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<&str>>(),
                app.selected_container.unwrap_or(0),
                "Container",
            ),
            _ => (vec![], 0, ""),
        };

        if !items.is_empty() {
            let dropdown_items: Vec<ListItem> =
                items.iter().map(|&item| ListItem::new(item)).collect();
            let dropdown = List::new(dropdown_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .title_style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::Cyan)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");
            let area = centered_rect(50, 30, f.size());
            // Clear the area first, then render the dropdown
            f.render_widget(Clear, area);
            f.render_stateful_widget(
                dropdown,
                area,
                &mut ListState::default().with_selected(Some(selected)),
            );
        }
    }
}

/// Helper function to create a centered rectangle
fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
