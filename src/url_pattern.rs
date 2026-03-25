use regex::Regex;
use serde::{Deserialize, Serialize};

/// Represents a URL pattern rule for auto-launching browsers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrlPattern {
    /// Regex pattern to match against URLs
    pub pattern: String,
    /// Browser name to use when pattern matches
    pub browser: String,
    /// Profile name to use (optional)
    pub profile: Option<String>,
    /// Container name to use for Firefox (optional)
    pub container: Option<String>,
    /// Whether to open in incognito/private mode
    pub incognito: bool,
    /// Whether to open in a new window
    pub new_window: bool,
}

/// Result of a successful pattern match
#[derive(Debug, Clone)]
pub struct PatternMatch<'a> {
    /// The pattern that matched
    pub pattern: &'a UrlPattern,
    /// The browser name to use
    pub browser_name: &'a str,
    /// The profile name to use (if specified)
    pub profile_name: Option<&'a str>,
    /// The container name to use (if specified)
    pub container_name: Option<&'a str>,
    /// Whether to open in incognito mode
    pub incognito: bool,
    /// Whether to open in a new window
    pub new_window: bool,
}

impl UrlPattern {
    /// Checks if the given URL matches this pattern
    pub fn matches(&self, url: &str) -> bool {
        match Regex::new(&self.pattern) {
            Ok(re) => re.is_match(url),
            Err(_) => {
                // If regex is invalid, try simple contains match
                url.contains(&self.pattern)
            }
        }
    }

    /// Validates that the pattern is a valid regex
    pub fn validate(&self) -> Result<(), String> {
        match Regex::new(&self.pattern) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Invalid regex pattern '{}': {}", self.pattern, e)),
        }
    }
}

/// Finds the first matching pattern for the given URL
pub fn find_matching_pattern<'a>(
    url: &str,
    patterns: &'a [UrlPattern],
) -> Option<PatternMatch<'a>> {
    for pattern in patterns {
        if pattern.matches(url) {
            return Some(PatternMatch {
                pattern,
                browser_name: &pattern.browser,
                profile_name: pattern.profile.as_deref(),
                container_name: pattern.container.as_deref(),
                incognito: pattern.incognito,
                new_window: pattern.new_window,
            });
        }
    }
    None
}

/// Validates all patterns in the list
pub fn validate_patterns(patterns: &[UrlPattern]) -> Result<(), Vec<String>> {
    let errors: Vec<String> = patterns.iter().filter_map(|p| p.validate().err()).collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_match() {
        let pattern = UrlPattern {
            pattern: r".*github\.com.*".to_string(),
            browser: "Firefox".to_string(),
            profile: Some("work".to_string()),
            container: None,
            incognito: false,
            new_window: false,
        };

        assert!(pattern.matches("https://github.com/user/repo"));
        assert!(pattern.matches("http://github.com"));
        assert!(!pattern.matches("https://google.com"));
    }

    #[test]
    fn test_find_matching_pattern() {
        let patterns = vec![
            UrlPattern {
                pattern: r".*github\.com.*".to_string(),
                browser: "Firefox".to_string(),
                profile: Some("work".to_string()),
                container: None,
                incognito: false,
                new_window: false,
            },
            UrlPattern {
                pattern: r".*youtube\.com.*".to_string(),
                browser: "Google Chrome".to_string(),
                profile: Some("Personal".to_string()),
                container: None,
                incognito: false,
                new_window: false,
            },
        ];

        let result = find_matching_pattern("https://github.com/user/repo", &patterns);
        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.browser_name, "Firefox");
        assert_eq!(match_result.profile_name, Some("work"));

        let result = find_matching_pattern("https://youtube.com/watch", &patterns);
        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.browser_name, "Google Chrome");

        let result = find_matching_pattern("https://example.com", &patterns);
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_pattern_fallback() {
        let pattern = UrlPattern {
            pattern: "github.com".to_string(), // Not a regex, but valid string
            browser: "Firefox".to_string(),
            profile: None,
            container: None,
            incognito: false,
            new_window: false,
        };

        // Should fall back to contains match
        assert!(pattern.matches("https://github.com/user"));
        assert!(!pattern.matches("https://google.com"));
    }
}
