//! Everything OS-specific lives behind this module. Each platform exposes the
//! same free functions; all of them block and are called via `spawn_blocking`.
//!
//! Windows is implemented, macOS and Linux get the stub until they are ported.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
pub use self::stub::*;

/// Media transport commands.
#[derive(Debug, Clone, Copy)]
pub enum MediaCmd {
    PlayPause,
    Next,
    Prev,
}

/// Expands `%NAME%` environment references in a launch target.
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
