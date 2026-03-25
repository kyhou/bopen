use crate::browser::Browser;
use crate::config::Config;
use crate::profile::{Container, Profile};
use std::path::PathBuf;

/// Represents the focusable elements in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Url,
    Browser,
    Profile,
    Container,
    IncognitoToggle,
    NewWindowToggle,
    CopyButton,
    OpenButton,
    QuitButton,
}

/// The main application state
#[derive(Debug)]
pub struct App {
    /// The URL to open
    pub url: String,
    /// Cursor position in the URL field
    pub url_cursor_pos: usize,
    /// List of discovered browsers
    pub browsers: Vec<Browser>,
    /// List of profiles for the selected browser
    pub profiles: Vec<Profile>,
    /// List of containers for the selected profile (if Firefox-based)
    pub containers: Vec<Container>,
    /// Currently selected browser index
    pub selected_browser: usize,
    /// Currently selected profile index
    pub selected_profile: usize,
    /// Currently selected container index (None if not applicable or no selection)
    pub selected_container: Option<usize>,
    /// Whether incognito/private mode is enabled
    pub incognito: bool,
    /// Whether new window is enabled
    pub new_window: bool,
    /// Currently focused widget
    pub focus: Focus,
    /// Error message to display (if any)
    pub error: Option<String>,
    /// Info message to display (if any)
    pub info: Option<String>,
    /// Which dropdown is currently open (if any)
    pub dropdown_open: Option<Focus>,
    /// Persistent configuration
    pub config: Config,
    /// Flag to request application exit (allows cleanup before exit)
    pub exit_requested: bool,
}

impl App {
    /// Creates a new App instance with default state
    pub fn new(initial_url: Option<String>) -> Self {
        let url_provided = initial_url.is_some();
        let url = initial_url.unwrap_or_default();
        let url_len = url.len();
        let mut app = Self {
            url,
            url_cursor_pos: url_len,
            browsers: Vec::new(),
            profiles: Vec::new(),
            containers: Vec::new(),
            selected_browser: 0,
            selected_profile: 0,
            selected_container: None,
            incognito: false,
            new_window: false,
            focus: Focus::Url,
            error: None,
            info: None,
            dropdown_open: None,
            config: Config::load(),
            exit_requested: false,
        };

        // Discover browsers on startup
        app.refresh_browsers();

        // Apply saved configuration if available
        app.apply_config();

        // If URL was provided, start focus on browser menu
        if url_provided {
            app.focus = Focus::Browser;
        }

        app
    }

    /// Refreshes the list of discovered browsers
    pub fn refresh_browsers(&mut self) {
        self.browsers = crate::browser::discover_browsers();
        if self.browsers.is_empty() {
            self.error = Some("No browsers found".to_string());
        } else {
            // Reset selections if the current selections are out of bounds
            if self.selected_browser >= self.browsers.len() {
                self.selected_browser = 0;
            }
            self.update_profile_and_container_lists();
        }
    }

    /// Updates the profile and container lists based on the selected browser
    pub fn update_profile_and_container_lists(&mut self) {
        if self.browsers.is_empty() {
            self.profiles = Vec::new();
            self.containers = Vec::new();
            return;
        }

        let browser = &self.browsers[self.selected_browser];
        let binary_name = browser.exec.split_whitespace().next().unwrap_or("");

        // Clear previous selections
        self.selected_profile = 0;
        self.selected_container = None;

        // Detect profiles based on browser type
        if crate::profile::is_firefox_based(binary_name) {
            self.profiles = crate::profile::detect_firefox_profiles(binary_name);
            if !self.profiles.is_empty() {
                // Update containers for the selected profile
                self.update_containers();
            } else {
                self.profiles = vec![Profile {
                    name: "Default".to_string(),
                    path: PathBuf::new(),
                    is_relative: false,
                }];
                self.containers = Vec::new();
            }
        } else if crate::profile::is_chromium_based(binary_name) {
            self.profiles = crate::profile::detect_chromium_profiles(binary_name);
            if self.profiles.is_empty() {
                self.profiles = vec![Profile {
                    name: "Default".to_string(),
                    path: PathBuf::new(),
                    is_relative: false,
                }];
            }
            self.containers = Vec::new();
        } else {
            self.profiles = crate::profile::detect_unknown_profiles();
            self.containers = Vec::new();
        }

        // Ensure profile selection is in bounds
        if self.selected_profile >= self.profiles.len() {
            self.selected_profile = 0;
        }
    }

    /// Updates the container list for the currently selected profile
    pub fn update_containers(&mut self) {
        if self.browsers.is_empty() || self.profiles.is_empty() {
            self.containers = Vec::new();
            return;
        }

        let browser = &self.browsers[self.selected_browser];
        let binary_name = browser.exec.split_whitespace().next().unwrap_or("");
        let profile = &self.profiles[self.selected_profile];

        if crate::profile::is_firefox_based(binary_name) {
            self.containers = crate::profile::detect_firefox_containers(&profile.path);
            if self.containers.is_empty() {
                self.selected_container = None;
            } else {
                // Try to restore last used container if it still exists
                if let Some(ref last_container) = self.config.last_container {
                    if let Some(index) = self
                        .containers
                        .iter()
                        .position(|c| c.name == *last_container)
                    {
                        self.selected_container = Some(index);
                    } else {
                        self.selected_container = Some(0);
                    }
                } else {
                    self.selected_container = Some(0);
                }
            }
        } else {
            self.containers = Vec::new();
            self.selected_container = None;
        }
    }

    /// Applies the saved configuration to the app state
    pub fn apply_config(&mut self) {
        // Apply last used browser
        if let Some(ref last_browser) = self.config.last_browser {
            if let Some(index) = self.browsers.iter().position(|b| b.name == *last_browser) {
                self.selected_browser = index;
                // Refresh profiles for the newly selected browser
                self.update_profile_and_container_lists();
            }
        }

        // Apply last used profile (now profiles list is correct for the browser)
        if let Some(ref last_profile) = self.config.last_profile {
            if let Some(index) = self.profiles.iter().position(|p| p.name == *last_profile) {
                self.selected_profile = index;
            }
        }

        // Refresh containers for the selected profile
        self.update_containers();

        // Apply last used container (now containers list is correct for the profile)
        if let Some(ref last_container) = self.config.last_container {
            if let Some(index) = self
                .containers
                .iter()
                .position(|c| c.name == *last_container)
            {
                self.selected_container = Some(index);
            }
        }

        // Apply toggle states
        self.incognito = self.config.last_incognito;
        self.new_window = self.config.last_new_window;
    }

    /// Saves the current state to the configuration
    pub fn save_config(&mut self) {
        if let Some(browser) = self.browsers.get(self.selected_browser) {
            self.config.last_browser = Some(browser.name.clone());
        }
        if let Some(profile) = self.profiles.get(self.selected_profile) {
            self.config.last_profile = Some(profile.name.clone());
        }
        if let Some(container_index) = self.selected_container {
            if let Some(container) = self.containers.get(container_index) {
                self.config.last_container = Some(container.name.clone());
            }
        }
        self.config.last_incognito = self.incognito;
        self.config.last_new_window = self.new_window;
        let _ = self.config.save();
    }

    /// Handles the tick event (for future use)
    #[allow(dead_code)]
    pub fn tick(&mut self) {}

    /// Sets an error message
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error = Some(message.into());
    }

    /// Clears the error message
    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// Sets an info message
    pub fn set_info(&mut self, message: impl Into<String>) {
        self.info = Some(message.into());
    }

    /// Clears the info message
    pub fn clear_info(&mut self) {
        self.info = None;
    }

    /// Requests the application to exit (allows cleanup before exit)
    pub fn exit(&mut self) {
        self.exit_requested = true;
    }

    /// Toggles the dropdown for the given focus
    pub fn toggle_dropdown(&mut self, focus: Focus) {
        match self.dropdown_open {
            Some(open_focus) if open_focus == focus => {
                self.dropdown_open = None;
            }
            _ => {
                self.dropdown_open = Some(focus);
            }
        }
    }

    /// Closes the currently open dropdown
    pub fn close_dropdown(&mut self) {
        self.dropdown_open = None;
    }

    /// Toggles incognito mode (mutually exclusive with new_window)
    pub fn toggle_incognito(&mut self) {
        if self.incognito {
            // Already incognito, turn it off
            self.incognito = false;
        } else {
            // Turn on incognito, turn off new_window
            self.incognito = true;
            self.new_window = false;
            // Warn if container is selected
            if self.selected_container.is_some() {
                self.set_info("Incognito is not available with containers".to_string());
            }
        }
    }

    /// Toggles new window mode (mutually exclusive with incognito)
    pub fn toggle_new_window(&mut self) {
        if self.new_window {
            // Already new window, turn it off
            self.new_window = false;
        } else {
            // Turn on new window, turn off incognito
            self.new_window = true;
            self.incognito = false;
        }
    }

    /// Moves the selection in the currently open dropdown
    pub fn select_next_in_dropdown(&mut self) {
        if let Some(open_focus) = self.dropdown_open {
            match open_focus {
                Focus::Browser => {
                    if !self.browsers.is_empty() {
                        self.selected_browser = (self.selected_browser + 1) % self.browsers.len();
                        self.update_profile_and_container_lists();
                    }
                }
                Focus::Profile => {
                    if !self.profiles.is_empty() {
                        self.selected_profile = (self.selected_profile + 1) % self.profiles.len();
                        self.update_containers();
                    }
                }
                Focus::Container => {
                    if !self.containers.is_empty() {
                        if let Some(selected) = self.selected_container.as_mut() {
                            *selected = (*selected + 1) % self.containers.len();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Moves the selection in the currently open dropdown backwards
    pub fn select_previous_in_dropdown(&mut self) {
        if let Some(open_focus) = self.dropdown_open {
            match open_focus {
                Focus::Browser => {
                    if !self.browsers.is_empty() {
                        if self.selected_browser == 0 {
                            self.selected_browser = self.browsers.len() - 1;
                        } else {
                            self.selected_browser -= 1;
                        }
                        self.update_profile_and_container_lists();
                    }
                }
                Focus::Profile => {
                    if !self.profiles.is_empty() {
                        if self.selected_profile == 0 {
                            self.selected_profile = self.profiles.len() - 1;
                        } else {
                            self.selected_profile -= 1;
                        }
                        self.update_containers();
                    }
                }
                Focus::Container => {
                    if !self.containers.is_empty() {
                        if let Some(selected) = self.selected_container.as_mut() {
                            if *selected == 0 {
                                *selected = self.containers.len() - 1;
                            } else {
                                *selected -= 1;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Handles a key press event
    pub fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) {
        // Clear any transient messages on key press
        self.clear_error();
        self.clear_info();

        match key.code {
            crossterm::event::KeyCode::Tab => {
                self.focus_next();
            }
            crossterm::event::KeyCode::BackTab => {
                self.focus_previous();
            }
            crossterm::event::KeyCode::Esc => {
                self.close_dropdown();
            }
            crossterm::event::KeyCode::Enter => {
                self.handle_enter();
            }
            crossterm::event::KeyCode::Up => {
                if self.dropdown_open.is_some() {
                    // Navigate within dropdown
                    self.select_previous_in_dropdown();
                } else {
                    // Navigate between fields (reverse Tab)
                    self.focus_previous();
                }
            }
            crossterm::event::KeyCode::Down => {
                if self.dropdown_open.is_some() {
                    // Navigate within dropdown
                    self.select_next_in_dropdown();
                } else {
                    // Navigate between fields (like Tab)
                    self.focus_next();
                }
            }
            crossterm::event::KeyCode::Left => {
                if self.focus == Focus::Url && self.url_cursor_pos > 0 {
                    self.url_cursor_pos -= 1;
                }
            }
            crossterm::event::KeyCode::Right => {
                if self.focus == Focus::Url && self.url_cursor_pos < self.url.len() {
                    self.url_cursor_pos += 1;
                }
            }
            crossterm::event::KeyCode::Backspace => {
                if self.focus == Focus::Url && self.url_cursor_pos > 0 {
                    self.url.remove(self.url_cursor_pos - 1);
                    self.url_cursor_pos -= 1;
                }
            }
            crossterm::event::KeyCode::Delete => {
                if self.focus == Focus::Url && self.url_cursor_pos < self.url.len() {
                    self.url.remove(self.url_cursor_pos);
                }
            }
            crossterm::event::KeyCode::Char(c) => {
                if self.focus == Focus::Url {
                    // Insert character at cursor position
                    self.url.insert(self.url_cursor_pos, c);
                    self.url_cursor_pos += 1;
                } else if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    // Handle Ctrl+key shortcuts
                    match c {
                        'o' => self.handle_open(),
                        'i' => {
                            self.toggle_incognito();
                        }
                        'w' => {
                            self.toggle_new_window();
                        }
                        'q' => {
                            self.exit();
                        }
                        _ => {}
                    }
                } else {
                    // Handle regular character shortcuts when not in URL field
                    match c {
                        'o' => self.handle_open(),
                        'c' | 'C' => {
                            self.copy_url_to_clipboard();
                        }
                        'i' => {
                            self.toggle_incognito();
                        }
                        'w' => {
                            self.toggle_new_window();
                        }
                        'q' => {
                            self.exit();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    /// Moves focus to the next focusable element
    pub fn focus_next(&mut self) {
        self.focus = match self.focus {
            Focus::Url => Focus::Browser,
            Focus::Browser => Focus::Profile,
            Focus::Profile => {
                if self.is_container_row_visible() {
                    Focus::Container
                } else {
                    Focus::IncognitoToggle
                }
            }
            Focus::Container => Focus::IncognitoToggle,
            Focus::IncognitoToggle => Focus::NewWindowToggle,
            Focus::NewWindowToggle => Focus::CopyButton,
            Focus::CopyButton => Focus::OpenButton,
            Focus::OpenButton => Focus::QuitButton,
            Focus::QuitButton => Focus::Url,
        };
        self.close_dropdown();
    }

    /// Moves focus to the previous focusable element
    pub fn focus_previous(&mut self) {
        self.focus = match self.focus {
            Focus::Url => Focus::QuitButton,
            Focus::Browser => Focus::Url,
            Focus::Profile => Focus::Browser,
            Focus::Container => Focus::Profile,
            Focus::IncognitoToggle => {
                if self.is_container_row_visible() {
                    Focus::Container
                } else {
                    Focus::Profile
                }
            }
            Focus::NewWindowToggle => Focus::IncognitoToggle,
            Focus::CopyButton => Focus::NewWindowToggle,
            Focus::OpenButton => Focus::CopyButton,
            Focus::QuitButton => Focus::OpenButton,
        };
        self.close_dropdown();
    }

    /// Returns true if the container row should be visible
    pub fn is_container_row_visible(&mut self) -> bool {
        if self.browsers.is_empty() {
            return false;
        }
        let browser = &self.browsers[self.selected_browser];
        let binary_name = browser.exec.split_whitespace().next().unwrap_or("");
        crate::profile::is_firefox_based(binary_name) && !self.containers.is_empty()
    }

    /// Handles the Enter key press based on current focus
    fn handle_enter(&mut self) {
        match self.focus {
            Focus::Browser | Focus::Profile | Focus::Container => {
                self.toggle_dropdown(self.focus);
            }
            Focus::IncognitoToggle => {
                self.toggle_incognito();
            }
            Focus::NewWindowToggle => {
                self.toggle_new_window();
            }
            Focus::CopyButton => {
                self.copy_url_to_clipboard();
            }
            Focus::OpenButton => {
                self.handle_open();
            }
            Focus::QuitButton => {
                self.exit();
            }
            Focus::Url => {} // Do nothing for URL field on Enter
        }
    }

    /// Copies the current URL to the clipboard
    pub fn copy_url_to_clipboard(&mut self) {
        if self.url.trim().is_empty() {
            self.set_error("URL is empty".to_string());
            return;
        }
        match crate::clipboard::copy(&self.url) {
            Ok(_) => self.set_info("URL copied to clipboard!".to_string()),
            Err(e) => self.set_error(format!("Failed to copy: {}", e)),
        }
    }

    /// Handles the open action (validates and launches)
    fn handle_open(&mut self) {
        // Validate URL
        if self.url.trim().is_empty() {
            self.set_error("URL cannot be empty".to_string());
            return;
        }

        // Validate browser selection
        if self.browsers.is_empty() {
            self.set_error("No browsers available".to_string());
            return;
        }

        // Check if incognito and container are both selected (incompatible)
        if self.incognito && self.selected_container.is_some() {
            self.set_error("Incognito is not available with containers".to_string());
            return;
        }

        // Save configuration before launching (do this before taking references to avoid borrowing conflicts)
        self.save_config();

        // Get selected browser, profile, and container after saving config
        let browser = &self.browsers[self.selected_browser];
        let profile = &self.profiles[self.selected_profile];
        let container = self
            .selected_container
            .and_then(|index| self.containers.get(index));

        // Launch the browser
        if let Err(e) = crate::launcher::launch(
            browser,
            profile,
            container,
            &self.url,
            self.incognito,
            self.new_window,
        ) {
            self.set_error(format!("Failed to launch browser: {}", e));
        } else {
            // Exit successfully after launching
            self.exit();
        }
    }
}
