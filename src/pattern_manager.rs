use crate::browser::Browser;
use crate::config::Config;
use crate::profile::{Container, Profile};
use crate::url_pattern::UrlPattern;

/// Represents the current mode of the pattern manager
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternManagerMode {
    /// Viewing the list of patterns
    List,
    /// Adding a new pattern
    Add,
    /// Editing an existing pattern
    Edit,
}

/// Represents which field is currently focused in the form
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Pattern,
    Browser,
    Profile,
    Container,
    Incognito,
    NewWindow,
    SaveButton,
    CancelButton,
}

/// Manages the URL pattern configuration UI
#[derive(Debug)]
pub struct PatternManager {
    /// Current mode (List, Add, Edit)
    pub mode: PatternManagerMode,
    /// List of patterns being managed
    pub patterns: Vec<UrlPattern>,
    /// Currently selected pattern index (for List mode)
    pub selected_index: usize,
    /// Form data for Add/Edit mode
    pub form: PatternForm,
    /// Cursor position in the pattern field
    pub pattern_cursor_pos: usize,
    /// Which form field is focused
    pub focused_field: FormField,
    /// Which dropdown is open (if any)
    pub dropdown_open: Option<FormField>,
    /// Available browsers for dropdown
    pub available_browsers: Vec<Browser>,
    /// Available profiles for dropdown (based on selected browser)
    pub available_profiles: Vec<Profile>,
    /// Available containers for dropdown (based on selected profile)
    pub available_containers: Vec<Container>,
    /// Selected browser index in dropdown
    pub selected_browser_index: usize,
    /// Selected profile index in dropdown
    pub selected_profile_index: usize,
    /// Selected container index in dropdown
    pub selected_container_index: usize,
    /// Error message to display
    pub error: Option<String>,
    /// Info message to display
    pub info: Option<String>,
    /// Whether the pattern manager should close
    pub should_close: bool,
    /// Whether patterns were modified
    pub modified: bool,
}

/// Form data for creating/editing a pattern
#[derive(Debug, Clone)]
pub struct PatternForm {
    pub pattern: String,
    pub browser: String,
    pub profile: String,
    pub container: String,
    pub incognito: bool,
    pub new_window: bool,
}

impl Default for PatternForm {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            browser: String::new(),
            profile: String::new(),
            container: String::new(),
            incognito: false,
            new_window: false,
        }
    }
}

impl PatternManager {
    /// Creates a new PatternManager from the current config
    pub fn new(config: &Config) -> Self {
        // Load available browsers
        let browsers = crate::browser::discover_browsers();
        let mut pm = Self {
            mode: PatternManagerMode::List,
            patterns: config.url_patterns.clone(),
            selected_index: 0,
            form: PatternForm::default(),
            pattern_cursor_pos: 0,
            focused_field: FormField::Pattern,
            dropdown_open: None,
            available_browsers: browsers.clone(),
            available_profiles: Vec::new(),
            available_containers: Vec::new(),
            selected_browser_index: 0,
            selected_profile_index: 0,
            selected_container_index: 0,
            error: None,
            info: None,
            should_close: false,
            modified: false,
        };

        // If browsers available, pre-select first one and load its profiles
        if let Some(browser) = browsers.first() {
            pm.form.browser = browser.name.clone();
            pm.update_profiles_for_browser();
        }

        pm
    }

    /// Updates available profiles based on selected browser
    fn update_profiles_for_browser(&mut self) {
        self.available_profiles.clear();
        self.available_containers.clear();
        self.form.profile.clear();
        self.form.container.clear();
        self.selected_profile_index = 0;
        self.selected_container_index = 0;

        if let Some(browser) = self
            .available_browsers
            .iter()
            .find(|b| b.name == self.form.browser)
        {
            let binary_name = browser.exec.split_whitespace().next().unwrap_or("");

            if crate::profile::is_firefox_based(binary_name) {
                self.available_profiles = crate::profile::detect_firefox_profiles(binary_name);
            } else if crate::profile::is_chromium_based(binary_name) {
                self.available_profiles = crate::profile::detect_chromium_profiles(binary_name);
            } else {
                self.available_profiles = crate::profile::detect_unknown_profiles();
            }

            // Pre-select first profile if available
            if let Some(profile) = self.available_profiles.first() {
                self.form.profile = profile.name.clone();
                self.update_containers_for_profile();
            }
        }
    }

    /// Updates available containers based on selected profile (Firefox only)
    fn update_containers_for_profile(&mut self) {
        self.available_containers.clear();
        self.form.container.clear();
        self.selected_container_index = 0;

        // Only load containers for Firefox-based browsers
        if let Some(browser) = self
            .available_browsers
            .iter()
            .find(|b| b.name == self.form.browser)
        {
            let binary_name = browser.exec.split_whitespace().next().unwrap_or("");

            if !crate::profile::is_firefox_based(binary_name) {
                return;
            }

            if let Some(profile) = self
                .available_profiles
                .iter()
                .find(|p| p.name == self.form.profile)
            {
                self.available_containers =
                    crate::profile::detect_firefox_containers(&profile.path);

                // Pre-select first container if available
                if let Some(container) = self.available_containers.first() {
                    self.form.container = container.name.clone();
                }
            }
        }
    }

    /// Returns true if container field should be visible
    pub fn is_container_field_visible(&self) -> bool {
        if let Some(browser) = self
            .available_browsers
            .iter()
            .find(|b| b.name == self.form.browser)
        {
            let binary_name = browser.exec.split_whitespace().next().unwrap_or("");
            return crate::profile::is_firefox_based(binary_name)
                && !self.available_containers.is_empty();
        }
        false
    }

    /// Starts adding a new pattern
    pub fn start_add(&mut self) {
        self.mode = PatternManagerMode::Add;
        self.form = PatternForm::default();
        self.pattern_cursor_pos = 0;
        self.focused_field = FormField::Pattern;
        self.dropdown_open = None;

        // Pre-select first browser if available
        if let Some(browser) = self.available_browsers.first() {
            self.form.browser = browser.name.clone();
            self.update_profiles_for_browser();
        }

        self.clear_error();
        self.clear_info();
    }

    /// Starts editing the selected pattern
    pub fn start_edit(&mut self) {
        if self.patterns.is_empty() {
            self.set_error("No patterns to edit");
            return;
        }

        if let Some(pattern) = self.patterns.get(self.selected_index) {
            self.mode = PatternManagerMode::Edit;
            self.form = PatternForm {
                pattern: pattern.pattern.clone(),
                browser: pattern.browser.clone(),
                profile: pattern.profile.clone().unwrap_or_default(),
                container: pattern.container.clone().unwrap_or_default(),
                incognito: pattern.incognito,
                new_window: pattern.new_window,
            };
            self.pattern_cursor_pos = pattern.pattern.len();

            // Update available profiles/containers for the browser
            self.update_profiles_for_browser();

            // Try to select the matching profile
            if let Some(idx) = self
                .available_profiles
                .iter()
                .position(|p| p.name == self.form.profile)
            {
                self.selected_profile_index = idx;
                self.update_containers_for_profile();

                // Try to select the matching container
                if let Some(cidx) = self
                    .available_containers
                    .iter()
                    .position(|c| c.name == self.form.container)
                {
                    self.selected_container_index = cidx;
                }
            }

            // Try to select the matching browser
            if let Some(bidx) = self
                .available_browsers
                .iter()
                .position(|b| b.name == self.form.browser)
            {
                self.selected_browser_index = bidx;
            }

            self.focused_field = FormField::Pattern;
            self.dropdown_open = None;
            self.clear_error();
            self.clear_info();
        }
    }

    /// Deletes the selected pattern
    pub fn delete_selected(&mut self) {
        if self.patterns.is_empty() {
            self.set_error("No patterns to delete");
            return;
        }

        self.patterns.remove(self.selected_index);
        self.modified = true;

        // Adjust selection if needed
        if self.selected_index >= self.patterns.len() && !self.patterns.is_empty() {
            self.selected_index = self.patterns.len() - 1;
        }

        self.set_info("Pattern deleted");
    }

    /// Saves the current form (Add or Edit mode)
    pub fn save_form(&mut self) {
        // Validate required fields
        if self.form.pattern.trim().is_empty() {
            self.set_error("Pattern cannot be empty");
            return;
        }

        if self.form.browser.trim().is_empty() {
            self.set_error("Browser cannot be empty");
            return;
        }

        // Validate regex pattern
        let pattern = UrlPattern {
            pattern: self.form.pattern.clone(),
            browser: self.form.browser.clone(),
            profile: if self.form.profile.trim().is_empty() {
                None
            } else {
                Some(self.form.profile.clone())
            },
            container: if self.form.container.trim().is_empty() {
                None
            } else {
                Some(self.form.container.clone())
            },
            incognito: self.form.incognito,
            new_window: self.form.new_window,
        };

        if let Err(e) = pattern.validate() {
            self.set_error(format!("Invalid pattern: {}", e));
            return;
        }

        // Save the pattern
        match self.mode {
            PatternManagerMode::Add => {
                self.patterns.push(pattern);
                self.set_info("Pattern added");
            }
            PatternManagerMode::Edit => {
                if self.patterns.get(self.selected_index).is_some() {
                    self.patterns[self.selected_index] = pattern;
                    self.set_info("Pattern updated");
                }
            }
            _ => {}
        }

        self.modified = true;
        self.mode = PatternManagerMode::List;
        self.dropdown_open = None;
    }

    /// Cancels the current Add/Edit operation
    pub fn cancel_form(&mut self) {
        self.mode = PatternManagerMode::List;
        self.dropdown_open = None;
        self.clear_error();
        self.clear_info();
    }

    /// Toggles the dropdown for the current field
    pub fn toggle_dropdown(&mut self) {
        match self.focused_field {
            FormField::Browser => {
                if self.dropdown_open == Some(FormField::Browser) {
                    self.dropdown_open = None;
                } else if !self.available_browsers.is_empty() {
                    self.dropdown_open = Some(FormField::Browser);
                    // Sync selection with current value
                    if let Some(idx) = self
                        .available_browsers
                        .iter()
                        .position(|b| b.name == self.form.browser)
                    {
                        self.selected_browser_index = idx;
                    }
                }
            }
            FormField::Profile => {
                if self.dropdown_open == Some(FormField::Profile) {
                    self.dropdown_open = None;
                } else if !self.available_profiles.is_empty() {
                    self.dropdown_open = Some(FormField::Profile);
                    if let Some(idx) = self
                        .available_profiles
                        .iter()
                        .position(|p| p.name == self.form.profile)
                    {
                        self.selected_profile_index = idx;
                    }
                }
            }
            FormField::Container => {
                if self.dropdown_open == Some(FormField::Container) {
                    self.dropdown_open = None;
                } else if !self.available_containers.is_empty() {
                    self.dropdown_open = Some(FormField::Container);
                    if let Some(idx) = self
                        .available_containers
                        .iter()
                        .position(|c| c.name == self.form.container)
                    {
                        self.selected_container_index = idx;
                    }
                }
            }
            _ => {}
        }
    }

    /// Closes any open dropdown
    pub fn close_dropdown(&mut self) {
        self.dropdown_open = None;
    }

    /// Moves to the next item in the open dropdown
    pub fn select_next_in_dropdown(&mut self) {
        match self.dropdown_open {
            Some(FormField::Browser) => {
                if !self.available_browsers.is_empty() {
                    self.selected_browser_index =
                        (self.selected_browser_index + 1) % self.available_browsers.len();
                    self.form.browser = self.available_browsers[self.selected_browser_index]
                        .name
                        .clone();
                    self.update_profiles_for_browser();
                }
            }
            Some(FormField::Profile) => {
                if !self.available_profiles.is_empty() {
                    self.selected_profile_index =
                        (self.selected_profile_index + 1) % self.available_profiles.len();
                    self.form.profile = self.available_profiles[self.selected_profile_index]
                        .name
                        .clone();
                    self.update_containers_for_profile();
                }
            }
            Some(FormField::Container) => {
                if !self.available_containers.is_empty() {
                    self.selected_container_index =
                        (self.selected_container_index + 1) % self.available_containers.len();
                    self.form.container = self.available_containers[self.selected_container_index]
                        .name
                        .clone();
                }
            }
            _ => {}
        }
    }

    /// Moves to the previous item in the open dropdown
    pub fn select_previous_in_dropdown(&mut self) {
        match self.dropdown_open {
            Some(FormField::Browser) => {
                if !self.available_browsers.is_empty() {
                    if self.selected_browser_index == 0 {
                        self.selected_browser_index = self.available_browsers.len() - 1;
                    } else {
                        self.selected_browser_index -= 1;
                    }
                    self.form.browser = self.available_browsers[self.selected_browser_index]
                        .name
                        .clone();
                    self.update_profiles_for_browser();
                }
            }
            Some(FormField::Profile) => {
                if !self.available_profiles.is_empty() {
                    if self.selected_profile_index == 0 {
                        self.selected_profile_index = self.available_profiles.len() - 1;
                    } else {
                        self.selected_profile_index -= 1;
                    }
                    self.form.profile = self.available_profiles[self.selected_profile_index]
                        .name
                        .clone();
                    self.update_containers_for_profile();
                }
            }
            Some(FormField::Container) => {
                if !self.available_containers.is_empty() {
                    if self.selected_container_index == 0 {
                        self.selected_container_index = self.available_containers.len() - 1;
                    } else {
                        self.selected_container_index -= 1;
                    }
                    self.form.container = self.available_containers[self.selected_container_index]
                        .name
                        .clone();
                }
            }
            _ => {}
        }
    }

    /// Moves to the next field in the form
    pub fn next_field(&mut self) {
        self.close_dropdown();
        self.focused_field = match self.focused_field {
            FormField::Pattern => FormField::Browser,
            FormField::Browser => FormField::Profile,
            FormField::Profile => {
                if self.is_container_field_visible() {
                    FormField::Container
                } else {
                    FormField::Incognito
                }
            }
            FormField::Container => FormField::Incognito,
            FormField::Incognito => FormField::NewWindow,
            FormField::NewWindow => FormField::SaveButton,
            FormField::SaveButton => FormField::CancelButton,
            FormField::CancelButton => FormField::Pattern,
        };
    }

    /// Moves to the previous field in the form
    pub fn previous_field(&mut self) {
        self.close_dropdown();
        self.focused_field = match self.focused_field {
            FormField::Pattern => FormField::CancelButton,
            FormField::Browser => FormField::Pattern,
            FormField::Profile => FormField::Browser,
            FormField::Container => FormField::Profile,
            FormField::Incognito => {
                if self.is_container_field_visible() {
                    FormField::Container
                } else {
                    FormField::Profile
                }
            }
            FormField::NewWindow => FormField::Incognito,
            FormField::SaveButton => FormField::NewWindow,
            FormField::CancelButton => FormField::SaveButton,
        };
    }

    /// Moves to the next pattern in the list
    pub fn next_pattern(&mut self) {
        if !self.patterns.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.patterns.len();
        }
    }

    /// Moves to the previous pattern in the list
    pub fn previous_pattern(&mut self) {
        if !self.patterns.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.patterns.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Toggles the incognito checkbox
    pub fn toggle_incognito(&mut self) {
        self.form.incognito = !self.form.incognito;
        // Incognito and new window are mutually exclusive
        if self.form.incognito {
            self.form.new_window = false;
        }
    }

    /// Toggles the new window checkbox
    pub fn toggle_new_window(&mut self) {
        self.form.new_window = !self.form.new_window;
        // Incognito and new window are mutually exclusive
        if self.form.new_window {
            self.form.incognito = false;
        }
    }

    /// Handles a key event
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        self.clear_error();
        self.clear_info();

        match self.mode {
            PatternManagerMode::List => self.handle_list_key(key),
            PatternManagerMode::Add | PatternManagerMode::Edit => self.handle_form_key(key),
        }
    }

    fn handle_list_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_close = true;
            }
            KeyCode::Char('a') => {
                self.start_add();
            }
            KeyCode::Char('e') => {
                self.start_edit();
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                self.delete_selected();
            }
            KeyCode::Up => {
                self.previous_pattern();
            }
            KeyCode::Down => {
                self.next_pattern();
            }
            _ => {}
        }
    }

    fn handle_form_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        // If dropdown is open, handle dropdown navigation
        if self.dropdown_open.is_some() {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.close_dropdown();
                    return;
                }
                KeyCode::Up => {
                    self.select_previous_in_dropdown();
                    return;
                }
                KeyCode::Down => {
                    self.select_next_in_dropdown();
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Tab => {
                self.next_field();
            }
            KeyCode::BackTab => {
                self.previous_field();
            }
            KeyCode::Esc => {
                self.cancel_form();
            }
            KeyCode::Enter => match self.focused_field {
                FormField::Browser | FormField::Profile | FormField::Container => {
                    self.toggle_dropdown();
                }
                FormField::SaveButton => {
                    self.save_form();
                }
                FormField::CancelButton => {
                    self.cancel_form();
                }
                FormField::Incognito => {
                    self.toggle_incognito();
                }
                FormField::NewWindow => {
                    self.toggle_new_window();
                }
                _ => {}
            },
            KeyCode::Left => {
                if self.focused_field == FormField::Pattern && self.pattern_cursor_pos > 0 {
                    self.pattern_cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if self.focused_field == FormField::Pattern
                    && self.pattern_cursor_pos < self.form.pattern.len()
                {
                    self.pattern_cursor_pos += 1;
                }
            }
            KeyCode::Up => {
                self.previous_field();
            }
            KeyCode::Down => {
                self.next_field();
            }
            KeyCode::Char(c) => {
                match self.focused_field {
                    FormField::Pattern => {
                        // Insert character at cursor position
                        self.form.pattern.insert(self.pattern_cursor_pos, c);
                        self.pattern_cursor_pos += 1;
                    }
                    FormField::Incognito
                    | FormField::NewWindow
                    | FormField::SaveButton
                    | FormField::CancelButton => {
                        // Handle shortcuts when not in text field
                        match c {
                            'i' => self.toggle_incognito(),
                            'w' => self.toggle_new_window(),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Backspace => {
                if self.focused_field == FormField::Pattern && self.pattern_cursor_pos > 0 {
                    self.form.pattern.remove(self.pattern_cursor_pos - 1);
                    self.pattern_cursor_pos -= 1;
                }
            }
            KeyCode::Delete => {
                if self.focused_field == FormField::Pattern
                    && self.pattern_cursor_pos < self.form.pattern.len()
                {
                    self.form.pattern.remove(self.pattern_cursor_pos);
                }
            }
            _ => {}
        }
    }

    /// Saves patterns to config
    pub fn save_to_config(&self, config: &mut Config) -> Result<(), Box<dyn std::error::Error>> {
        config.url_patterns = self.patterns.clone();
        config.save()
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_manager_new() {
        let config = Config::default();
        let manager = PatternManager::new(&config);
        assert_eq!(manager.mode, PatternManagerMode::List);
        assert!(manager.patterns.is_empty());
    }

    #[test]
    fn test_add_pattern() {
        let config = Config::default();
        let mut manager = PatternManager::new(&config);

        manager.start_add();
        assert_eq!(manager.mode, PatternManagerMode::Add);

        manager.form.pattern = r".*github\.com.*".to_string();
        manager.form.browser = "Firefox".to_string();
        manager.form.profile = "work".to_string();

        manager.save_form();

        assert_eq!(manager.mode, PatternManagerMode::List);
        assert_eq!(manager.patterns.len(), 1);
        assert!(manager.modified);
    }

    #[test]
    fn test_delete_pattern() {
        let mut config = Config::default();
        config.url_patterns.push(UrlPattern {
            pattern: r".*test.*".to_string(),
            browser: "Firefox".to_string(),
            profile: None,
            container: None,
            incognito: false,
            new_window: false,
        });

        let mut manager = PatternManager::new(&config);
        assert_eq!(manager.patterns.len(), 1);

        manager.delete_selected();
        assert!(manager.patterns.is_empty());
        assert!(manager.modified);
    }

    #[test]
    fn test_invalid_pattern_validation() {
        let config = Config::default();
        let mut manager = PatternManager::new(&config);

        manager.start_add();
        manager.form.pattern = "[invalid".to_string(); // Invalid regex
        manager.form.browser = "Firefox".to_string();

        manager.save_form();

        // Should not save due to invalid regex
        assert_eq!(manager.mode, PatternManagerMode::Add);
        assert!(manager.error.is_some());
    }
}
