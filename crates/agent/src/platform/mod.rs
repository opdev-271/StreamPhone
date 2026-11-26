//! Everything OS-specific lives behind this module. Each platform exposes the
//! same free functions; all of them block and are called via `spawn_blocking`.
//!
//! Windows and macOS are implemented; Linux gets the stub until it is ported.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

// Also built for tests elsewhere: its text handling is checked on every platform.
#[cfg(any(target_os = "macos", test))]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(not(any(windows, target_os = "macos")))]
mod stub;
#[cfg(not(any(windows, target_os = "macos")))]
pub use self::stub::*;

/// Media transport commands.
#[derive(Debug, Clone, Copy)]
pub enum MediaCmd {
    PlayPause,
    Next,
    Prev,
}

/// Expands `%NAME%` environment references in a launch target.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn expand_env(input: &str) -> String {
    let mut out = String::new();
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(value) if !name.is_empty() => out.push_str(&value),
                    _ => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::expand_env;

    #[test]
    fn expands_known_variables_only() {
        std::env::set_var("SP_TEST_DIR", "C:\\Apps");
        assert_eq!(expand_env("%SP_TEST_DIR%\\x.exe"), "C:\\Apps\\x.exe");
        assert_eq!(expand_env("%SP_NOT_SET_123%\\x"), "%SP_NOT_SET_123%\\x");
        assert_eq!(expand_env("100% done"), "100% done");
        assert_eq!(expand_env("plain"), "plain");
    }
}
