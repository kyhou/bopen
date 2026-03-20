/// Copies text to the system clipboard
pub fn copy(text: &str) -> Result<(), String> {
    // Try Wayland first (wl-copy)
    if std::process::Command::new("wl-copy")
        .arg("--trim-newline")
        .arg(text)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Ok(());
    }

    // Try X11 (xclip) as fallback
    let mut child = std::process::Command::new("xclip")
        .args(["-selection", "clipboard", "-r"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn xclip: {}", e))?;

    if let Some(ref mut stdin) = child.stdin {
        use std::io::Write;
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("Failed to write to xclip: {}", e))?;
    }

    child
        .wait()
        .map_err(|e| format!("Failed to wait for xclip: {}", e))?;

    Ok(())
}
