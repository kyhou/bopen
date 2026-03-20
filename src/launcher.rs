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

    // Resolve the full path to the binary
    let binary_path = resolve_binary_path(&clean_exec)
        .ok_or_else(|| format!("Browser binary not found: {}", clean_exec))?;

    // Build command arguments based on browser type
    let args = if super::profile::is_firefox_based(&clean_exec) {
        build_firefox_args(&binary_path, profile, container, url, incognito, new_window)
    } else if super::profile::is_chromium_based(&clean_exec) {
        build_chromium_args(&binary_path, profile, url, incognito, new_window)
    } else {
        vec![binary_path.clone(), url.to_string()]
    };

    // Build the full command string with proper quoting
    let cmd_str = args
        .iter()
        .map(|s| shell_quote(s))
        .collect::<Vec<_>>()
        .join(" ");

    // Use setsid to create a new session for the browser
    // This detaches it from the terminal so it survives when terminal closes
    let mut command = Command::new("setsid");
    command.arg("sh");
    command.arg("-c");
    command.arg(&cmd_str);

    // Explicitly inherit all environment variables
    command.envs(std::env::vars());

    command.spawn()?;

    // Small delay to allow the browser to start before we exit
    std::thread::sleep(std::time::Duration::from_millis(100));

    Ok(())
}

/// Builds Firefox arguments as a vector
fn build_firefox_args(
    exec: &str,
    profile: &Profile,
    container: Option<&Container>,
    url: &str,
    incognito: bool,
    new_window: bool,
) -> Vec<String> {
    let mut args = vec![
        exec.to_string(),
        "--no-remote".to_string(),
        "-P".to_string(),
        profile.name.clone(),
    ];

    if let Some(container) = container {
        let uri = format!("ext+container:name={}&url={}", container.name, url);
        args.push(uri);
    } else {
        if incognito {
            args.push("--private-window".to_string());
        } else if new_window {
            args.push("--new-window".to_string());
        }
        args.push(url.to_string());
    }

    args
}

/// Builds Chromium arguments as a vector
fn build_chromium_args(
    exec: &str,
    profile: &Profile,
    url: &str,
    incognito: bool,
    new_window: bool,
) -> Vec<String> {
    let mut args = vec![
        exec.to_string(),
        format!("--profile-directory={}", profile.name),
    ];

    if incognito {
        args.push("--incognito".to_string());
    }
    if new_window {
        args.push("--new-window".to_string());
    }
    args.push(url.to_string());

    args
}

/// Properly quote a string for shell execution
fn shell_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace("'", "'\\''"))
    }
}

/// Resolves a binary name to its full path
fn resolve_binary_path(binary: &str) -> Option<String> {
    let binary_name = binary.split_whitespace().next().unwrap_or(binary);

    // If it's an absolute path and exists, return it
    if std::path::Path::new(binary_name).is_absolute() {
        if std::path::Path::new(binary_name).exists() {
            return Some(binary_name.to_string());
        }
    }

    // Common system directories to check (for desktop environment launches)
    let system_dirs = ["/usr/bin", "/usr/local/bin", "/snap/bin", "/opt/bin"];

    // Check in PATH first
    if let Ok(paths) = std::env::var("PATH") {
        for path in paths.split(':') {
            let full_path = std::path::Path::new(path).join(binary_name);
            if full_path.exists() {
                return full_path.to_str().map(String::from);
            }
        }
    }

    // Check common system directories
    for dir in system_dirs {
        let full_path = std::path::Path::new(dir).join(binary_name);
        if full_path.exists() {
            return full_path.to_str().map(String::from);
        }
    }

    // Last resort: return the binary name and let the system try
    Some(binary_name.to_string())
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
