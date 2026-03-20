use std::fs;
use std::path::{Path, PathBuf};

/// Represents a browser profile
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: String,
    pub path: PathBuf,
    pub is_relative: bool,
}

/// Represents a Firefox container
#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    pub name: String,
    pub user_context_id: u32,
}

/// Detects profiles for Firefox-based browsers
pub fn detect_firefox_profiles(binary_name: &str) -> Vec<Profile> {
    let mut profiles = Vec::new();

    // Try the standard Firefox profiles location first
    let mut profiles_ini_path = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    profiles_ini_path.push(".mozilla");
    profiles_ini_path.push("firefox");
    profiles_ini_path.push("profiles.ini");

    // If the binary name suggests a different Mozilla-based browser, try its specific path
    if binary_name.to_lowercase().contains("librewolf") {
        profiles_ini_path = PathBuf::from(std::env::var("HOME").unwrap_or_default());
        profiles_ini_path.push(".mozilla");
        profiles_ini_path.push("librewolf");
        profiles_ini_path.push("profiles.ini");
    } else if binary_name.to_lowercase().contains("waterfox") {
        profiles_ini_path = PathBuf::from(std::env::var("HOME").unwrap_or_default());
        profiles_ini_path.push(".mozilla");
        profiles_ini_path.push("waterfox");
        profiles_ini_path.push("profiles.ini");
    }

    if let Ok(contents) = fs::read_to_string(&profiles_ini_path) {
        let mut _current_section = String::new();
        let mut name = None;
        let mut path = None;
        let mut is_relative = None;

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                // Save previous profile if we have one
                if let (Some(name_val), Some(path_val), Some(is_relative_val)) =
                    (name.take(), path.take(), is_relative.take())
                {
                    let path_buf = if is_relative_val {
                        let mut base_path = profiles_ini_path.clone();
                        base_path.pop(); // Remove profiles.ini
                        base_path.push(path_val);
                        base_path
                    } else {
                        PathBuf::from(path_val)
                    };

                    profiles.push(Profile {
                        name: name_val,
                        path: path_buf,
                        is_relative: is_relative_val,
                    });
                }

                // Start new section
                _current_section = line[1..line.len() - 1].to_string();
                name = None;
                path = None;
                is_relative = None;
            } else if let Some(equals_pos) = line.find('=') {
                let key = line[..equals_pos].trim();
                let value = line[equals_pos + 1..].trim();

                match key {
                    "Name" => name = Some(value.to_string()),
                    "Path" => path = Some(value.to_string()),
                    "IsRelative" => is_relative = Some(value == "1"),
                    _ => {}
                }
            }
        }

        // Handle the last profile
        if let (Some(name_val), Some(path_val), Some(is_relative_val)) =
            (name.take(), path.take(), is_relative.take())
        {
            let path_buf = if is_relative_val {
                let mut base_path = profiles_ini_path.clone();
                base_path.pop(); // Remove profiles.ini
                base_path.push(path_val);
                base_path
            } else {
                PathBuf::from(path_val)
            };

            profiles.push(Profile {
                name: name_val,
                path: path_buf,
                is_relative: is_relative_val,
            });
        }
    }

    profiles
}

/// Detects containers for a Firefox profile
pub fn detect_firefox_containers(profile_path: &Path) -> Vec<Container> {
    let mut containers = Vec::new();
    let mut containers_json_path = profile_path.to_path_buf();
    containers_json_path.push("containers.json");

    if let Ok(contents) = fs::read_to_string(&containers_json_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
            if let Some(identities) = json.get("identities").and_then(|v| v.as_array()) {
                for identity in identities {
                    if let Some(obj) = identity.as_object() {
                        // Only include public containers
                        if obj.get("public").and_then(|v| v.as_bool()).unwrap_or(false) {
                            if let (Some(name_val), Some(id_val)) = (
                                obj.get("name").and_then(|v| v.as_str()),
                                obj.get("userContextId").and_then(|v| v.as_u64()),
                            ) {
                                // Map known localization keys to human-readable names
                                let display_name = match name_val {
                                    "userContextPersonal.label" => "Personal",
                                    "userContextWork.label" => "Work",
                                    "userContextBanking.label" => "Banking",
                                    "userContextShopping.label" => "Shopping",
                                    _ => name_val,
                                };

                                containers.push(Container {
                                    name: display_name.to_string(),
                                    user_context_id: id_val as u32,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    containers
}

/// Detects profiles for Chromium-based browsers
pub fn detect_chromium_profiles(binary_name: &str) -> Vec<Profile> {
    let mut profiles = Vec::new();

    // Determine the config directory based on the binary name
    let config_dir = match binary_name {
        "google-chrome" => PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".config")
            .join("google-chrome"),
        "chromium" | "chromium-browser" => PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".config")
            .join("chromium"),
        "brave-browser" => PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".config")
            .join("BraveSoftware")
            .join("Brave-Browser"),
        "microsoft-edge" => PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".config")
            .join("microsoft-edge"),
        "vivaldi" => PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".config")
            .join("vivaldi"),
        "opera" => PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".config")
            .join("opera"),
        _ => PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".config")
            .join(binary_name),
    };

    if !config_dir.is_dir() {
        return profiles;
    }

    // Check for Default profile
    let default_dir = config_dir.join("Default");
    if default_dir.is_dir() {
        let profile_name = get_profile_name(&default_dir, "Default");
        profiles.push(Profile {
            name: profile_name,
            path: default_dir,
            is_relative: false,
        });
    }

    // Check for Profile N directories
    if let Ok(entries) = fs::read_dir(&config_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if file_name.starts_with("Profile ") {
                    let profile_name = get_profile_name(&path, file_name);
                    profiles.push(Profile {
                        name: profile_name,
                        path,
                        is_relative: false,
                    });
                }
            }
        }
    }

    profiles
}

/// Gets the profile name from the Preferences file or falls back to the directory name
fn get_profile_name(profile_dir: &Path, fallback_name: &str) -> String {
    let preferences_path = profile_dir.join("Preferences");
    if let Ok(contents) = fs::read_to_string(&preferences_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
            if let Some(profile_obj) = json.get("profile").and_then(|v| v.as_object()) {
                if let Some(name_val) = profile_obj.get("name").and_then(|v| v.as_str()) {
                    return name_val.to_string();
                }
            }
        }
    }
    fallback_name.to_string()
}

/// Detects profiles for unknown browsers (returns a single default profile)
pub fn detect_unknown_profiles() -> Vec<Profile> {
    vec![Profile {
        name: "Default".to_string(),
        path: PathBuf::new(),
        is_relative: false,
    }]
}

/// Determines if a browser is Firefox-based
pub fn is_firefox_based(binary_name: &str) -> bool {
    let lower = binary_name.to_lowercase();
    lower.contains("firefox") || lower.contains("librewolf") || lower.contains("waterfox")
}

/// Determines if a browser is Chromium-based
pub fn is_chromium_based(binary_name: &str) -> bool {
    let lower = binary_name.to_lowercase();
    matches!(
        lower.as_str(),
        "google-chrome"
            | "chromium"
            | "chromium-browser"
            | "brave-browser"
            | "microsoft-edge"
            | "vivaldi"
            | "opera"
    ) || lower.contains("chrome")
        || lower.contains("edge")
}
