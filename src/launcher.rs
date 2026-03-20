use std::process::Command;

use crate::browser::Browser;
use crate::profile::{Container, Profile};

/// Launches the browser with the given parameters
pub fn launch(
    browser: &Browser,
    profile: &Profile,
    container: Option<&Container>,
    url: &str,
    incognito: bool,
    new_window: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Clean the exec path (remove placeholders like %u)
    let clean_exec = clean_exec(&browser.exec);

    // Check if the browser binary exists
    if !is_binary_available(&clean_exec) {
        return Err(format!("Browser binary not found: {}", clean_exec).into());
    }

    // Build the command based on browser type
    let mut command = if super::profile::is_firefox_based(&clean_exec) {
        build_firefox_command(&clean_exec, profile, container, url, incognito, new_window)?
    } else if super::profile::is_chromium_based(&clean_exec) {
        build_chromium_command(&clean_exec, profile, url, incognito, new_window)
    } else {
        build_unknown_command(&clean_exec, url)
    };

    // Spawn the process detached from the terminal
    command.spawn()?;

    Ok(())
}

/// Cleans the Exec field by removing URL placeholders
fn clean_exec(exec: &str) -> String {
    exec.replace("%u", "")
        .replace("%U", "")
        .replace("%f", "")
        .replace("%F", "")
        .replace("%i", "")
        .replace("%c", "")
        .replace("%k", "")
        .trim()
        .to_string()
}

/// Checks if a binary is available in PATH
fn is_binary_available(binary: &str) -> bool {
    // Extract just the binary name (first token)
    let binary_name = binary.split_whitespace().next().unwrap_or(binary);

    // Check if it's an absolute path
    if std::path::Path::new(binary_name).is_absolute() {
        return std::path::Path::new(binary_name).exists();
    }

    // Check in PATH
    if let Ok(paths) = std::env::var("PATH") {
        for path in paths.split(':') {
            let full_path = std::path::Path::new(path).join(binary_name);
            if full_path.exists() {
                return true;
            }
        }
    }

    false
}

/// Builds the command for Firefox-based browsers
fn build_firefox_command(
    exec: &str,
    profile: &Profile,
    container: Option<&Container>,
    url: &str,
    incognito: bool,
    new_window: bool,
) -> Result<Command, Box<dyn std::error::Error>> {
    let mut command = Command::new(exec);

    // Base Firefox arguments - use --no-remote to prevent reusing existing windows
    command.arg("--no-remote");
    command.arg("-P");
    command.arg(&profile.name);

    // Handle container vs regular profile
    if let Some(container) = container {
        // Container mode requires the Open URL in Container extension
        // Build the ext+container: URI
        let uri = format!("ext+container:name={}&url={}", container.name, url);
        command.arg(uri);
    } else {
        // Regular profile mode - flags must come BEFORE the URL
        if incognito {
            command.arg("--private-window");
        } else if new_window {
            command.arg("--new-window");
        }
        // URL comes last
        command.arg(url);
    }

    Ok(command)
}

/// Builds the command for Chromium-based browsers
fn build_chromium_command(
    exec: &str,
    profile: &Profile,
    url: &str,
    incognito: bool,
    new_window: bool,
) -> Command {
    let mut command = Command::new(exec);

    // Base Chromium arguments
    command.arg(format!("--profile-directory={}", profile.name));
    command.arg(url);

    // Add incognito flag if requested
    if incognito {
        command.arg("--incognito");
    }

    // Add new window flag if requested
    if new_window {
        command.arg("--new-window");
    }

    command
}

/// Builds the command for unknown browsers
fn build_unknown_command(exec: &str, url: &str) -> Command {
    let mut command = Command::new(exec);
    command.arg(url);
    command
}
