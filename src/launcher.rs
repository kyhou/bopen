use std::ffi::CString;

use fork::{fork, Fork};

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
    let args: Vec<CString> = if super::profile::is_firefox_based(&clean_exec) {
        build_firefox_args(&binary_path, profile, container, url, incognito, new_window)
    } else if super::profile::is_chromium_based(&clean_exec) {
        build_chromium_args(&binary_path, profile, url, incognito, new_window)
    } else {
        vec![CString::new(binary_path.as_str())?, CString::new(url)?]
    };

    // Fork and exec - this is the approach used by similar projects like brofile
    // fork() creates a child process, exec() replaces it with the browser
    unsafe {
        match fork() {
            Ok(Fork::Child) => {
                // Child process - exec the browser
                let exec_path = CString::new(binary_path.as_str())?;
                let mut argv: Vec<*const i8> = args.iter().map(|s| s.as_ptr()).collect();
                argv.push(std::ptr::null());

                // Use setsid to create new session before exec
                libc::setsid();

                libc::execvp(exec_path.as_ptr(), argv.as_ptr());
                // If execvp returns, it failed
                std::process::exit(1);
            }
            Ok(Fork::Parent(_)) => {
                // Parent process - success, parent will exit shortly
            }
            Err(e) => {
                return Err(format!("fork failed: {}", e).into());
            }
        }
    }

    Ok(())
}

/// Builds Firefox arguments as a CString vector for execvp
fn build_firefox_args(
    exec: &str,
    profile: &Profile,
    container: Option<&Container>,
    url: &str,
    incognito: bool,
    new_window: bool,
) -> Vec<CString> {
    let mut args = vec![
        CString::new(exec).unwrap(),
        CString::new("--no-remote").unwrap(),
        CString::new("-P").unwrap(),
        CString::new(profile.name.as_str()).unwrap(),
    ];

    if let Some(container) = container {
        let uri = format!("ext+container:name={}&url={}", container.name, url);
        args.push(CString::new(uri).unwrap());
    } else {
        if incognito {
            args.push(CString::new("--private-window").unwrap());
        } else if new_window {
            args.push(CString::new("--new-window").unwrap());
        }
        args.push(CString::new(url).unwrap());
    }

    args
}

/// Builds Chromium arguments as a CString vector for execvp
fn build_chromium_args(
    exec: &str,
    profile: &Profile,
    url: &str,
    incognito: bool,
    new_window: bool,
) -> Vec<CString> {
    let mut args = vec![
        CString::new(exec).unwrap(),
        CString::new(format!("--profile-directory={}", profile.name)).unwrap(),
    ];

    if incognito {
        args.push(CString::new("--incognito").unwrap());
    }
    if new_window {
        args.push(CString::new("--new-window").unwrap());
    }
    args.push(CString::new(url).unwrap());

    args
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
