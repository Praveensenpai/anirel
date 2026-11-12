//! Small platform helpers: browser launching, clipboard, TTY detection.

use std::io::{IsTerminal, Write};
use std::process::{Command, Stdio};

/// Open a URL in the user's default browser.
pub fn open_in_browser(url: &str) -> Result<(), String> {
    let candidates: &[&str] = if cfg!(target_os = "macos") {
        &["open"]
    } else if cfg!(target_os = "windows") {
        &["explorer"]
    } else {
        &["xdg-open", "gio", "gnome-open", "kde-open", "wslview"]
    };

    for program in candidates {
        let mut command = Command::new(program);
        command.arg(url).stdout(Stdio::null()).stderr(Stdio::null());
        if command.spawn().is_ok() {
            return Ok(());
        }
    }
    Err(format!("could not find a browser launcher for {url}"))
}

/// Copy text to the clipboard using the OSC 52 escape sequence.
///
/// Works over SSH and in every modern terminal, and needs no extra tooling.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let encoded = base64_encode(text.as_bytes());
    let payload = format!("\x1b]52;c;{encoded}\x07");
    let mut stdout = std::io::stdout();
    stdout
        .write_all(payload.as_bytes())
        .map_err(|e| format!("clipboard write failed: {e}"))?;
    stdout
        .flush()
        .map_err(|e| format!("clipboard flush failed: {e}"))?;
    Ok(())
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// True when stdout is an interactive terminal.
pub fn stdout_is_tty() -> bool {
    std::io::stdout().is_terminal()
}

/// True when colours should be used.
pub fn color_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false) {
        return false;
    }
    stdout_is_tty()
}

#[cfg(test)]
mod tests {
    use super::base64_encode;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
