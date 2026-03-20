use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

/// Represents a discovered browser
#[derive(Debug, Clone, PartialEq)]
pub struct Browser {
    pub name: String,
    pub exec: String,
}

/// Discovers installed browsers by scanning .desktop files in the specified directories.
///
/// The directories are searched in order, and duplicates (by resolved binary path) are skipped.
///
/// # Returns
/// A vector of discovered browsers, or an empty vector if none are found.
pub fn discover_browsers() -> Vec<Browser> {
    let dirs = [
        "/usr/share/applications/",
        "/usr/local/share/applications/",
        &format!(
            "{}/.local/share/applications/",
            std::env::var("HOME").unwrap_or_default()
        ),
    ];

    let mut seen_execs = HashSet::new();
    let mut browsers = Vec::new();

    for dir in dirs.iter() {
        let path = Path::new(dir);
        if !path.is_dir() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if file_path.extension() != Some(OsStr::new("desktop")) {
                    continue;
                }

                if let Some(browser) = parse_desktop_file(&file_path) {
                    // Resolve the binary path from the Exec field to check for duplicates
                    let resolved_exec = resolve_exec(&browser.exec);
                    if seen_execs.insert(resolved_exec) {
                        browsers.push(browser);
                    }
                }
            }
        }
    }

    browsers
}

/// Parses a .desktop file and returns a Browser if it represents a web browser.
fn parse_desktop_file(path: &Path) -> Option<Browser> {
    let contents = fs::read_to_string(path).ok()?;

    // .desktop files can have multiple entries separated by [Section] headers
    // We need to parse each entry separately
    for entry in parse_desktop_entries(&contents) {
        if let Some(browser) = parse_single_entry(&entry) {
            return Some(browser);
        }
    }

    None
}

/// Parses a .desktop file and returns each [Desktop Entry] section as a string
fn parse_desktop_entries(contents: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut current_entry = String::new();
    let mut in_entry = false;

    for line in contents.lines() {
        let trimmed = line.trim();

        // Check for section header
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Save previous entry if it exists and has content
            if in_entry && !current_entry.trim().is_empty() {
                entries.push(current_entry.clone());
            }
            // Start new entry
            current_entry.clear();

            // Check if this is a Desktop Entry section
            if trimmed == "[Desktop Entry]" {
                current_entry.push_str(line);
                current_entry.push('\n');
                in_entry = true;
            } else {
                in_entry = false;
            }
        } else if in_entry {
            current_entry.push_str(line);
            current_entry.push('\n');
        }
    }

    // Don't forget the last entry
    if in_entry && !current_entry.trim().is_empty() {
        entries.push(current_entry);
    }

    entries
}

/// Parses a single [Desktop Entry] section and returns a Browser if valid
fn parse_single_entry(entry: &str) -> Option<Browser> {
    let mut name = None;
    let mut exec = None;
    let mut categories = None;
    let mut typ = None;

    for line in entry.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(stripped) = line.strip_prefix("Name=") {
            name = Some(stripped.to_string());
        } else if let Some(stripped) = line.strip_prefix("Exec=") {
            exec = Some(stripped.to_string());
        } else if let Some(stripped) = line.strip_prefix("Categories=") {
            categories = Some(stripped.to_string());
        } else if let Some(stripped) = line.strip_prefix("Type=") {
            typ = Some(stripped.to_string());
        }
    }

    // Check if it's an application with WebBrowser category
    if typ.as_deref() != Some("Application") {
        return None;
    }

    // Must contain WebBrowser in categories
    let has_web_browser = categories
        .as_ref()
        .map(|c| c.contains("WebBrowser"))
        .unwrap_or(false);

    if !has_web_browser {
        return None;
    }

    let browser_name = name.as_deref().unwrap_or("");

    // Filter out browser actions/features - these are not actual browsers
    if is_browser_action(browser_name) {
        return None;
    }

    Some(Browser {
        name: name.unwrap_or_else(|| "Unknown".to_string()),
        exec: exec.unwrap_or_default(),
    })
}

/// Checks if the name indicates this is a browser action/feature, not a browser itself.
/// Examples: "New Incognito Window", "Open Profile Manager", "Tor Browser Launcher Settings"
fn is_browser_action(name: &str) -> bool {
    let name_lower = name.to_lowercase();

    // Keywords that indicate this is an action/feature, not the main browser
    let action_keywords = [
        "incognito",
        "private window",
        "private browsing",
        "new window",
        "new tab",
        "profile manager",
        "profile",
        "settings",
        "preferences",
        "launcher",
        "setup",
        "wizard",
        "first run",
        "about",
        "help",
        "sync",
        "bookmarks",
        "history",
        "downloads",
        "addons",
        "extensions",
        "plugins",
        "crash reporter",
        "safe mode",
        "refresh",
        "update",
        "check for updates",
        "release notes",
        "whats new",
        " whats new",
    ];

    for keyword in action_keywords {
        if name_lower.contains(keyword) {
            return true;
        }
    }

    false
}

/// Resolves the Exec field to a binary path by removing placeholders and taking the first token.
///
/// For example, "firefox %u" becomes "firefox".
fn resolve_exec(exec: &str) -> String {
    // Remove any trailing placeholders (%u, %U, %f, %F) and split by whitespace
    let cleaned = exec
        .replace("%u", "")
        .replace("%U", "")
        .replace("%f", "")
        .replace("%F", "")
        .trim()
        .to_string();

    // Take the first token (the binary name or path)
    cleaned.split_whitespace().next().unwrap_or("").to_string()
}
